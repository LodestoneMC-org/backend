use std::path::Path;
use std::{fs, fs::File};
use std::collections::HashMap;
use std::io::prelude::*;
use mongodb::{bson, options::ClientOptions, sync::Client};
use rocket::State;
use rocket::fairing::Result;
use serde_json::to_string;
use crate::MyManagedState;
use crate::instance::*;
use crate::util;
use crate::properties_manager::PropertiesManager;
use uuid::Uuid; 


pub struct InstanceManager{
    instance_collection : HashMap<String, ServerInstance>,
    taken_ports : Vec<u32>, 
    path : String, // must end with /
    mongodb : Client,
}




// TODO: DB IO
// TODO : should prob change parameter String to &str
impl InstanceManager {
    pub fn new(path : String, mongodb : Client) -> Result<InstanceManager, String> {
        let path_to_config = format!("{}.lodestone_config/", path);
        fs::create_dir_all(path_to_config.as_str()).map_err(|e| e.to_string())?;
        if !Path::exists(Path::new(format!("{}server.properties", path_to_config).as_str())) {
            let mut properties_file = File::create(format!("{}server.properties", path_to_config)).unwrap();
            properties_file.write_all(
        b"enable-jmx-monitoring=false\nrcon.port=25575\nenable-command-block=false\ngamemode=survival\nenable-query=false\nlevel-name=world\nmotd=AMinecraftServer\nquery.port=25565\npvp=true\ndifficulty=easy\nnetwork-compression-threshold=256\nmax-tick-time=60000\nrequire-resource-pack=false\nmax-players=20\nuse-native-transport=true\nonline-mode=true\nenable-status=true\nallow-flight=false\nvbroadcast-rcon-to-ops=true\nview-distance=10\nserver-ip=\nresource-pack-prompt=\nallow-nether=true\nserver-port=25565\nenable-rcon=false\nsync-chunk-writes=true\nop-permission-level=4\nprevent-proxy-connections=false\nhide-online-players=false\nresource-pack=\nentity-broadcast-range-percentage=100\nsimulation-distance=10\nrcon.password=\nplayer-idle-timeout=0\nforce-gamemode=false\nrate-limit=0\nhardcore=false\nwhite-list=false\nbroadcast-console-to-ops=true\nspawn-npcs=true\nspawn-animals=true\nfunction-permission-level=2\ntext-filtering-config=\nspawn-monsters=true\nenforce-whitelist=false\nresource-pack-sha1=\nspawn-protection=16\nmax-world-size=29999984\n").unwrap();
        }

        let mut instance_collection: HashMap<String, ServerInstance> = HashMap::new();

        let database_names = mongodb
            .list_database_names(None, None).unwrap();
        for database_name in database_names.iter() {
            if !(database_name == "admin" || database_name == "config" || database_name == "local") {
            let config = mongodb
                .database(database_name)
                .collection::<InstanceConfig>("config")
                .find_one(None, None)
                .unwrap()
                .unwrap();
            let key = config.uuid.clone().unwrap();
            instance_collection.insert(key, ServerInstance::new(&config, format!("{}{}", path, config.name)));
            }
        }


        Ok(InstanceManager{
            instance_collection,
            path,
            mongodb,
            taken_ports : vec![]
        })
    }
    // TODO: server.properties 
    pub async fn create_instance(&mut self, mut config : InstanceConfig, state: &State<MyManagedState>) -> Result<String, String> {
        config.name = sanitize_filename::sanitize(config.name);

        if self.check_if_name_exists(&config.name) {
            return Err(format!("{} already exists as an instance", &config.name));
        }
        let uuid = format!("{}", Uuid::new_v4());
        config.uuid = Some(uuid.clone());

        let path_to_instance = format!("{}{}/", self.path, config.name);

        fs::create_dir(path_to_instance.as_str()).map_err(|e| e.to_string())?;
        let instance = ServerInstance::new(&config, path_to_instance.clone());
        util::download_file(&config.url.unwrap(), format!("{}server.jar", &path_to_instance).as_str(), state, instance.uuid.as_str()).await?; // TODO: get rid of await

        let path_to_eula = format!("{}eula.txt", path_to_instance);
        let mut eula_file = File::create(path_to_eula.as_str()).map_err(|_|"failed to create eula.txt".to_string())?;
        eula_file.write_all(b"#generated by Lodestone\neula=true\n").map_err(|_| "failed to write to eula,txt".to_string())?;
        
        let path_to_properties = format!("{}server.properties", path_to_instance);
        self.instance_collection.insert(uuid.clone(), instance);
        fs::copy(format!("{}../.lodestone_config/server.properties", path_to_instance), path_to_properties).unwrap();
        match config.port {
            None => {
                for port in 25565..26000 {
                    if !self.taken_ports.contains(&port) {
                        self.taken_ports.push(port);
                        println!("using port {}", port);
                        let mut pm = PropertiesManager::new(format!("{}server.properties", path_to_instance)).unwrap();
                        pm.edit_field("server-port".to_string(), port.to_string()).unwrap();
                        pm.write_to_file().unwrap();
                        config.port = Some(port);
                        break;
                    }
                }
            }
            Some(_) => (),
        }
        // TODO: DB IO
        /* TODO: 
            create a database with the uuid name 
            create config collection 
                config is everything needed to reconstruct the config 
                store InstanceConfig into database
        */ 
        self.mongodb
            .database(&uuid)
            .collection("config")
            .insert_one(bson::doc! {
                "name": &config.name,
                "version": &config.version,
                "flavour": &config.flavour,
                "port": &config.port,
                "uuid": &config.uuid.unwrap(),
                "min_ram": &config.min_ram.unwrap_or(1024),
                "max_ram": &config.max_ram.unwrap_or(2048)
            }, None).unwrap();

        Ok(uuid)
    }


    // TODO: basically drop database
    pub fn delete_instance(&mut self, uuid : String) -> Result<(), String> {
        match self.instance_collection.remove(&uuid) {
            None => Err("instance not found".to_string()),
            Some(instance) => {
                // handling db
                self.mongodb
                    .database(&uuid)
                    .drop(None)
                    .unwrap();
                
                    fs::remove_dir_all(format!("{}{}", self.path, instance.name)).map_err(|_| format!("{}{}", self.path, instance.name))?;
                Ok(())
            }
        }
    }

    pub fn clone_instance(&mut self, uuid : String) -> Result<(), String> {
        for pair in &self.instance_collection {
            if pair.0 == &uuid {
                if self.check_if_name_exists(&format!("{}_copy", &pair.1.name)) {
                    return Err(format!("{}_copy already exists as an instance", &pair.1.name));
                }
            }
        };
        Ok(())
    }

    
    pub fn send_command(&self, uuid : String, command : String) -> Result<(), String> {
        let instance = self.instance_collection.get(&uuid).ok_or("cannot send command to instance as it does not exist".to_string())?;
        instance.stdin.clone().unwrap().send(format!("{}\n", command)).map_err(|_| "failed to send command to instance".to_string())?;
        Ok(())
    }

    pub fn start_instance(&mut self, uuid : String) -> Result<(), String> {
        let instance = self.instance_collection.get_mut(&uuid).ok_or("instance cannot be started as it does not exist".to_string())?;
        instance.start(self.mongodb.clone())
    }

    pub fn stop_instance(&mut self, uuid : String) -> Result<(), String> {
        let instance = self.instance_collection.get_mut(&uuid).ok_or("instance cannot be stopped as it does not exist".to_string())?;
        instance.stop()
    }

    fn check_if_name_exists(&self, name : &String) -> bool {
        // TODO: DB IO
        let mut ret = false;
        for pair in &self.instance_collection {
            if &pair.1.name == name {
                ret = true;
                break; 
            }
        }
        ret
    }



}
