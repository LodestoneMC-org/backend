pub mod mc_configurable;
pub mod mc_resource;
pub mod mc_server;
mod util;

use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::{Arc, Mutex};

use ::serde::{Deserialize, Serialize};
use rocket::serde;
use rocket::serde::json::serde_json::to_string_pretty;

use crate::traits::t_configurable::PathBuf;

use crate::traits::{Error, ErrorInner};

#[derive(Debug, Clone, Copy)]
pub enum Flavour {
    Vanilla,
    Fabric,
    Paper,
    Spigot,
}

impl<'de> serde::Deserialize<'de> for Flavour {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "vanilla" => Ok(Flavour::Vanilla),
            "fabric" => Ok(Flavour::Fabric),
            "paper" => Ok(Flavour::Paper),
            "spigot" => Ok(Flavour::Spigot),
            _ => Err(serde::de::Error::custom(format!("Unknown flavour: {}", s))),
        }
    }
}
impl serde::Serialize for Flavour {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Flavour::Vanilla => serializer.serialize_str("vanilla"),
            Flavour::Fabric => serializer.serialize_str("fabric"),
            Flavour::Paper => serializer.serialize_str("paper"),
            Flavour::Spigot => serializer.serialize_str("spigot"),
        }
    }
}

impl ToString for Flavour {
    fn to_string(&self) -> String {
        match self {
            Flavour::Vanilla => "vanilla".to_string(),
            Flavour::Fabric => "fabric".to_string(),
            Flavour::Paper => "paper".to_string(),
            Flavour::Spigot => "spigot".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub uuid: String,
    pub name: String,
    pub version: String,
    pub fabric_loader_version: Option<String>,
    pub fabric_installer_version: Option<String>,
    // TODO: add paper support
    pub flavour: Flavour,
    pub description: String,
    pub jvm_args: Vec<String>,
    pub path: PathBuf,
    pub port: u32,
    pub min_ram: u32,
    pub max_ram: u32,
    pub creation_time: u64,
    pub auto_start: bool,
    pub restart_on_crash: bool,
    pub timeout_last_left: Option<i32>,
    pub timeout_no_activity: Option<i32>,
    pub start_on_connection: bool,
    pub backup_period: Option<i32>,
}
pub struct Instance {
    config: Config,

    // file paths
    path_to_config: PathBuf,
    path_to_eula: PathBuf,
    path_to_properties: PathBuf,

    // directory paths
    path_to_macros: PathBuf,
    path_to_resources: PathBuf,

    // variables which can be changed at runtime
    auto_start: Arc<Mutex<bool>>,
    restart_on_crash: Arc<Mutex<bool>>,
    timeout_last_left: Arc<Option<AtomicI32>>,
    timeout_no_activity: Arc<Option<AtomicI32>>,
    start_on_connection: Arc<AtomicBool>,
    backup_period: Arc<Option<AtomicI32>>,
}

impl Instance {
    pub async fn new(config: Config) -> Result<Instance, Error> {
        let path_to_config = config.path.join(".lodestone_config");
        let path_to_eula = config.path.join("eula.txt");
        let path_to_macros = config.path.join("macros");
        let path_to_resources = config.path.join("resources");
        let path_to_properties = config.path.join("server.properties");
        let path_to_runtimes = config.path.parent().unwrap().join(".lodestone");

        // create eula file
        std::fs::write(&path_to_eula, "#generated by Lodestone\neula=true").map_err(|_| Error {
            inner: ErrorInner::FailedToWriteFile,
            detail: format!("failed to write to {}", &path_to_eula.display()),
        });

        // create macros directory
        std::fs::create_dir_all(&path_to_macros).map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: format!("failed to create {}", &path_to_macros.display()),
        })?;

        // create resources directory
        std::fs::create_dir_all(path_to_resources.join("mods")).map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: format!(
                "failed to create mods directory {}",
                &path_to_resources.display()
            ),
        })?;
        std::fs::create_dir_all(path_to_resources.join("worlds")).map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: format!(
                "failed to create worlds directory {}",
                &path_to_resources.display()
            ),
        })?;
        std::fs::create_dir_all(path_to_resources.join("defaults")).map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: format!(
                "failed to create defaults directory {}",
                &path_to_resources.display()
            ),
        })?;
        std::fs::create_dir_all(&path_to_runtimes).map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: format!(
                "failed to create runtime directory {}",
                &path_to_runtimes.display()
            ),
        })?;

        // create config file
        std::fs::write(
            &path_to_config,
            to_string_pretty(&config).map_err(|_| Error {
                inner: ErrorInner::MalformedFile,
                detail: "config json malformed".to_string(),
            })?,
        )
        .map_err(|_| Error {
            inner: ErrorInner::FailedToWriteFile,
            detail: format!("failed to write to config {}", &path_to_config.display()),
        })?;

        Ok(Instance {
            auto_start: Arc::new(Mutex::new(config.auto_start)),
            restart_on_crash: Arc::new(Mutex::new(config.restart_on_crash)),
            timeout_last_left: Arc::new(config.timeout_last_left.map(|x| AtomicI32::new(x))),
            timeout_no_activity: Arc::new(config.timeout_no_activity.map(|x| AtomicI32::new(x))),
            start_on_connection: Arc::new(AtomicBool::new(config.start_on_connection)),
            backup_period: Arc::new(config.backup_period.map(|x| AtomicI32::new(x))),
            config,
            path_to_config,
            path_to_eula,
            path_to_properties,
            path_to_macros,
            path_to_resources,
        })
    }

    pub fn restore(config: Config) -> Result<Instance, Error> {
        let path_to_config = config.path.join(".lodestone_config");
        let path_to_eula = config.path.join("eula.txt");
        let path_to_macros = config.path.join("macros");
        let path_to_resources = config.path.join("resources");
        let path_to_properties = config.path.join("server.properties");

        Ok(Instance {
            auto_start: Arc::new(Mutex::new(config.auto_start)),
            restart_on_crash: Arc::new(Mutex::new(config.restart_on_crash)),
            timeout_last_left: Arc::new(config.timeout_last_left.map(|x| AtomicI32::new(x))),
            timeout_no_activity: Arc::new(config.timeout_no_activity.map(|x| AtomicI32::new(x))),
            start_on_connection: Arc::new(AtomicBool::new(config.start_on_connection)),
            backup_period: Arc::new(config.backup_period.map(|x| AtomicI32::new(x))),
            config,
            path_to_config,
            path_to_eula,
            path_to_properties,
            path_to_macros,
            path_to_resources,
        })
    }
}