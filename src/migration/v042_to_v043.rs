use std::path::Path;

use color_eyre::eyre::Context;
use serde_json::json;

use crate::{error::Error, implementations::minecraft_java::RestoreConfig};

use super::RestoreConfigV042;

impl From<RestoreConfigV042> for RestoreConfig {
    fn from(config: RestoreConfigV042) -> Self {
        Self {
            name: config.name,
            version: config.version,
            flavour: config.flavour,
            description: config.description,
            cmd_args: config.cmd_args,
            port: config.port,
            min_ram: config.min_ram,
            max_ram: config.max_ram,
            auto_start: config.auto_start,
            restart_on_crash: config.restart_on_crash,
            backup_period: config.backup_period,
            jre_major_version: config.jre_major_version,
            has_started: config.has_started,
            java_cmd: None,
        }
    }
}

pub async fn migrate_v042_to_v043(
    mut old_dot_lodestone_config: serde_json::Value,
    path_to_instance: &Path,
) -> Result<(), Error> {
    if let Some("fabric") = old_dot_lodestone_config["flavour"].as_str() {
        old_dot_lodestone_config["flavour"] =
            json!({ "fabric": { "loader_version": null, "installer_version": null } });
    } else if let Some("paper") = old_dot_lodestone_config["flavour"].as_str() {
        old_dot_lodestone_config["flavour"] = json!({ "paper": { "build_version": null } });
    }

    let path_to_dot_lodestone_config = path_to_instance.join(".lodestone_config");
    let path_to_dot_lodestone_minecraft_config =
        path_to_instance.join(".lodestone_minecraft_config.json");
    let dot_lodestone_config: RestoreConfigV042 = serde_json::from_value(old_dot_lodestone_config)
        .context("Failed to deserialize old config file. This is likely a bug in Lodestone.")?;

    let dot_lodestone_config_new: crate::types::DotLodestoneConfig =
        dot_lodestone_config.clone().into();
    let dot_lodestone_config_new = serde_json::to_string_pretty(&dot_lodestone_config_new).unwrap();
    tokio::fs::write(&path_to_dot_lodestone_config, dot_lodestone_config_new)
        .await
        .context(format!(
            "Failed to write config file at {}",
            &path_to_dot_lodestone_config.display()
        ))?;

    let dot_lodestone_minecraft_config: RestoreConfig = dot_lodestone_config.into();
    let dot_lodestone_minecraft_config =
        serde_json::to_string_pretty(&dot_lodestone_minecraft_config).unwrap();
    tokio::fs::write(
        &path_to_dot_lodestone_minecraft_config,
        dot_lodestone_minecraft_config,
    )
    .await
    .context(format!(
        "Failed to write config file at {}",
        &path_to_dot_lodestone_minecraft_config.display()
    ))?;
    Ok(())
}
