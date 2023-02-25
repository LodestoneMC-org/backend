use std::collections::BTreeMap;
pub use std::path::PathBuf;

use color_eyre::eyre::eyre;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
use ts_rs::TS;

use crate::error::Error;
use crate::error::ErrorKind;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub enum ConfigurableValue {
    String(String),
    Integer(i32),
    UnsignedInteger(u32),
    Float(f32),
    Boolean(bool),
    Enum(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigurableValueType {
    String(Option<String>), // regex
    Integer { min: Option<i32>, max: Option<i32> },
    UnsignedInteger { min: Option<u32>, max: Option<u32> },
    Float { min: Option<f32>, max: Option<f32> },
    Boolean,
    Enum { options: Vec<String> },
}

impl ToString for ConfigurableValueType {
    fn to_string(&self) -> String {
        match self {
            ConfigurableValueType::String(_) => "string".to_string(),
            ConfigurableValueType::Integer { .. } => "integer".to_string(),
            ConfigurableValueType::UnsignedInteger { .. } => "unsigned integer".to_string(),
            ConfigurableValueType::Float { .. } => "float".to_string(),
            ConfigurableValueType::Boolean => "boolean".to_string(),
            ConfigurableValueType::Enum { .. } => "enum".to_string(),
        }
    }
}

impl ConfigurableValueType {
    pub fn type_check(&self, value: &ConfigurableValue) -> Result<(), Error> {
        match (self, value) {
            (ConfigurableValueType::String(regex), ConfigurableValue::String(value)) => {
                if let Some(regex) = regex {
                    if let Ok(regex) = fancy_regex::Regex::new(regex) {
                        if let Ok(true) = regex.is_match(value) {
                            Ok(())
                        } else {
                            Err(Error {
                                kind: ErrorKind::BadRequest,
                                source: eyre!("Value does not match regex"),
                            })
                        }
                    } else {
                        Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Invalid regex"),
                        })
                    }
                } else {
                    Ok(())
                }
            }
            (ConfigurableValueType::Integer { min, max }, ConfigurableValue::Integer(value)) => {
                if let Some(min) = min {
                    if value < min {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too small"),
                        });
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too large"),
                        });
                    }
                }
                Ok(())
            }
            (
                ConfigurableValueType::UnsignedInteger { min, max },
                ConfigurableValue::UnsignedInteger(value),
            ) => {
                if let Some(min) = min {
                    if value < min {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too small"),
                        });
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too large"),
                        });
                    }
                }
                Ok(())
            }
            (ConfigurableValueType::Float { min, max }, ConfigurableValue::Float(value)) => {
                if let Some(min) = min {
                    if value < min {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too small"),
                        });
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        return Err(Error {
                            kind: ErrorKind::BadRequest,
                            source: eyre!("Value is too large"),
                        });
                    }
                }
                Ok(())
            }
            (ConfigurableValueType::Boolean, ConfigurableValue::Boolean(_)) => Ok(()),
            (ConfigurableValueType::Enum { options }, ConfigurableValue::Enum(value)) => {
                if options.contains(value) {
                    Ok(())
                } else {
                    Err(Error {
                        kind: ErrorKind::BadRequest,
                        source: eyre!("Value is not in enum"),
                    })
                }
            }
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Type mismatch"),
            }),
        }
    }
}

impl ToString for ConfigurableValue {
    fn to_string(&self) -> String {
        match self {
            ConfigurableValue::String(value) => value.to_string(),
            ConfigurableValue::Integer(value) => value.to_string(),
            ConfigurableValue::UnsignedInteger(value) => value.to_string(),
            ConfigurableValue::Float(value) => value.to_string(),
            ConfigurableValue::Boolean(value) => value.to_string(),
            ConfigurableValue::Enum(value) => value.to_string(),
        }
    }
}

impl ConfigurableValue {
    pub fn infer_type(&self) -> ConfigurableValueType {
        match self {
            ConfigurableValue::String(_) => ConfigurableValueType::String(None),
            ConfigurableValue::Integer(_) => ConfigurableValueType::Integer {
                min: None,
                max: None,
            },
            ConfigurableValue::UnsignedInteger(_) => ConfigurableValueType::UnsignedInteger {
                min: None,
                max: None,
            },
            ConfigurableValue::Float(_) => ConfigurableValueType::Float {
                min: None,
                max: None,
            },
            ConfigurableValue::Boolean(_) => ConfigurableValueType::Boolean,
            ConfigurableValue::Enum(_) => ConfigurableValueType::Enum { options: vec![] },
        }
    }

    pub fn try_as_integer(&self) -> Result<i32, Error> {
        match self {
            ConfigurableValue::Integer(value) => Ok(*value),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Expected integer, found {}", self.infer_type().to_string()),
            }),
        }
    }

    pub fn try_as_unsigned_integer(&self) -> Result<u32, Error> {
        match self {
            ConfigurableValue::UnsignedInteger(value) => Ok(*value),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!(
                    "Expected unsigned integer, found {}",
                    self.infer_type().to_string()
                ),
            }),
        }
    }

    pub fn try_as_float(&self) -> Result<f32, Error> {
        match self {
            ConfigurableValue::Float(value) => Ok(*value),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Expected float, found {}", self.infer_type().to_string()),
            }),
        }
    }

    pub fn try_as_enum(&self) -> Result<&String, Error> {
        match self {
            ConfigurableValue::Enum(value) => Ok(value),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Expected enum, found {}", self.infer_type().to_string()),
            }),
        }
    }

    pub fn try_as_boolean(&self) -> Result<bool, Error> {
        match self {
            ConfigurableValue::Boolean(b) => Ok(*b),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Expected boolean, found {}", self.infer_type().to_string()),
            }),
        }
    }

    pub fn try_as_string(&self) -> Result<&String, Error> {
        match self {
            ConfigurableValue::String(s) => Ok(s),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Expected string, found {}", self.infer_type().to_string()),
            }),
        }
    }
}

// A SettingManifest contains a unique identifier, a name and a description
// and a value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingManifest {
    pub setting_id: String, // static, cannot change at runtime
    pub name: String,
    pub description: String,
    pub value: Option<ConfigurableValue>,
    pub value_type: ConfigurableValueType,
    pub default_value: Option<ConfigurableValue>, // static, cannot change at runtime
    pub is_secret: bool,                          // ??
    pub is_required: bool,                        // ??
    pub is_mutable: bool,                         // CAN change at runtime
}

impl SettingManifest {
    pub fn get_value(&self) -> Option<&ConfigurableValue> {
        self.value.as_ref()
    }
    pub fn get_identifier(&self) -> &String {
        &self.setting_id
    }
    /// # WARNING
    /// Will infer the type of the value from the value itself
    ///
    /// A number will be unbounded
    ///
    /// A string will have no regex
    ///
    /// An enum will have no options
    pub fn new_required_value(
        setting_id: String,
        name: String,
        description: String,
        value: ConfigurableValue,
        default_value: Option<ConfigurableValue>,
        is_secret: bool,
        is_mutable: bool,
    ) -> Self {
        Self {
            setting_id,
            name,
            description,
            value: Some(value.clone()),
            value_type: value.infer_type(),
            default_value,
            is_secret,
            is_required: true,
            is_mutable,
        }
    }
    pub fn new_optional_value(
        setting_id: String,
        name: String,
        description: String,
        value: Option<ConfigurableValue>,
        value_type: ConfigurableValueType,
        default_value: Option<ConfigurableValue>,
        is_secret: bool,
        is_mutable: bool,
    ) -> Self {
        Self {
            setting_id,
            name,
            description,
            value,
            value_type,
            default_value,
            is_secret,
            is_required: false,
            is_mutable,
        }
    }

    pub fn new_value_with_type(
        setting_id: String,
        name: String,
        description: String,
        value: Option<ConfigurableValue>,
        value_type: ConfigurableValueType,
        default_value: Option<ConfigurableValue>,
        is_secret: bool,
        is_mutable: bool,
    ) -> Result<Self, Error> {
        if let Some(value) = value {
            value_type.type_check(&value)?;
            Ok(Self {
                setting_id,
                name,
                description,
                value: Some(value),
                value_type,
                default_value,
                is_secret,
                is_required: true,
                is_mutable,
            })
        } else {
            Ok(Self {
                setting_id,
                name,
                description,
                is_required: false,
                value,
                value_type,
                default_value,
                is_secret,
                is_mutable,
            })
        }
        
    }

    fn set_value_type_safe(&mut self, value: ConfigurableValue) -> Result<(), Error> {
        self.value_type
            .type_check(&value)
            .map_err(|e| Error {
                kind: ErrorKind::BadRequest,
                source: eyre!(e),
            })
            .map(|_| {
                self.value = Some(value);
            })
    }

    pub fn set_value(&mut self, value: ConfigurableValue) -> Result<(), Error> {
        if self.is_mutable {
            self.set_value_type_safe(value)
        } else {
            Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Setting is not mutable"),
            })
        }
    }

    pub fn set_optional_value(&mut self, value: Option<ConfigurableValue>) -> Result<(), Error> {
        if self.is_mutable {
            if value.is_none() && self.is_required {
                Err(Error {
                    kind: ErrorKind::BadRequest,
                    source: eyre!("Setting is required"),
                })
            } else {
                self.value = value;
                Ok(())
            }
        } else {
            Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Setting is not mutable"),
            })
        }
    }
}

// A Setting section contains a name and a description (for UI)
// A Setting section contains a list of InstanceSetting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionManifest {
    pub(super) section_id: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) settings: BTreeMap<String, SettingManifest>,
}

impl SectionManifest {
    pub fn new(
        section_id: String,
        name: String,
        description: String,
        settings: BTreeMap<String, SettingManifest>,
    ) -> Self {
        Self {
            section_id,
            name,
            description,
            settings,
        }
    }

    pub fn get_setting(&self, setting_id: &str) -> Option<&SettingManifest> {
        self.settings.get(setting_id)
    }

    pub fn add_setting(&mut self, setting: SettingManifest) -> Result<(), Error> {
        if self.settings.contains_key(setting.get_identifier()) {
            Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Setting already exists"),
            })
        } else {
            self.settings
                .insert(setting.get_identifier().clone(), setting);
            Ok(())
        }
    }

    pub fn set_setting(&mut self, setting: SettingManifest) -> Result<(), Error> {
        if self.settings.contains_key(setting.get_identifier()) {
            self.settings
                .insert(setting.get_identifier().clone(), setting);
            Ok(())
        } else {
            Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Setting does not exist"),
            })
        }
    }

    pub fn insert_setting(&mut self, setting: SettingManifest) {
        self.settings
            .insert(setting.get_identifier().clone(), setting);
    }

    pub fn update_setting(
        &mut self,
        setting_id: &str,
        value: ConfigurableValue,
    ) -> Result<(), Error> {
        if let Some(setting) = self.settings.get_mut(setting_id) {
            setting.set_value(value)
        } else {
            Err(Error {
                kind: ErrorKind::NotFound,
                source: eyre!("Setting does not exist"),
            })
        }
    }
}

// A setting manifest indicates if the instance has implemented functionalities for smart, lodestone controlled feature
// A setting manifest has an ordered list of Setting Section
pub struct ConfigurableManifest {
    auto_start: bool,
    restart_on_crash: bool,
    start_on_connection: bool,
    timeout_last_left: bool,
    setting_sections: BTreeMap<String, SectionManifest>,
}

impl ConfigurableManifest {
    pub fn new(
        auto_start: bool,
        restart_on_crash: bool,
        start_on_connection: bool,
        timeout_last_left: bool,
        setting_sections: BTreeMap<String, SectionManifest>,
    ) -> Self {
        Self {
            auto_start,
            restart_on_crash,
            start_on_connection,
            timeout_last_left,
            setting_sections,
        }
    }

    pub fn get_setting(&self, section_id: &str, setting_id: &str) -> Option<&SettingManifest> {
        if let Some(section) = self.setting_sections.get(section_id) {
            section.settings.get(setting_id)
        } else {
            None
        }
    }

    fn get_setting_mut(
        &mut self,
        section_id: &str,
        setting_id: &str,
    ) -> Option<&mut SettingManifest> {
        if let Some(section) = self.setting_sections.get_mut(section_id) {
            section.settings.get_mut(setting_id)
        } else {
            None
        }
    }

    pub fn get_section(&self, section_id: &str) -> Option<&SectionManifest> {
        self.setting_sections.get(section_id)
    }

    pub fn get_all_sections(&self) -> BTreeMap<String, SectionManifest> {
        self.setting_sections.clone()
    }

    pub fn set_setting_value(
        &mut self,
        section_id: &str,
        setting_id: &str,
        value: Option<ConfigurableValue>,
    ) -> Result<(), Error> {
        if let Some(setting) = self.get_setting_mut(section_id, setting_id) {
            setting.set_optional_value(value)
        } else {
            Err(Error {
                kind: ErrorKind::NotFound,
                source: eyre!("Setting not found"),
            })
        }
    }

    pub fn set_setting(
        &mut self,
        section_id: &str,
        setting_id: &str,
        setting: SettingManifest,
    ) -> Result<(), Error> {
        if let Some(section) = self.setting_sections.get_mut(section_id) {
            section.set_setting(setting)
        } else {
            Err(Error {
                kind: ErrorKind::NotFound,
                source: eyre!("Section not found"),
            })
        }
    }

    pub fn set_setting_mut(
        &mut self,
        section_id: &str,
        setting_id: &str,
        is_mutable: bool,
    ) -> Option<bool> {
        if let Some(setting) = self.get_setting_mut(section_id, setting_id) {
            let ret = setting.is_mutable;
            setting.is_mutable = is_mutable;
            Some(ret)
        } else {
            None
        }
    }

    pub fn update_setting_value(
        &mut self,
        section_id: &str,
        setting_id: &str,
        value: ConfigurableValue,
    ) -> Result<(), Error> {
        if let Some(setting) = self.get_setting_mut(section_id, setting_id) {
            setting.set_value(value)
        } else {
            Err(Error {
                kind: ErrorKind::NotFound,
                source: eyre!("Setting not found"),
            })
        }
    }
}

#[derive(Clone)]
pub struct SettingManifestValue {
    pub(super) value: Option<ConfigurableValue>,
}

#[derive(Clone)]
pub struct SectionManifestValue {
    pub(super) settings: BTreeMap<String, SettingManifestValue>,
}

impl SectionManifestValue {
    pub fn get_setting(&self, setting_id: &str) -> Option<&SettingManifestValue> {
        self.settings.get(setting_id)
    }
}

#[derive(Clone)]
pub struct ManifestValue {
    pub(super) setting_sections: BTreeMap<String, SectionManifestValue>,
}

impl ManifestValue {
    pub fn get_setting(&self, section_id: &str, setting_id: &str) -> Option<&SettingManifestValue> {
        if let Some(section) = self.setting_sections.get(section_id) {
            section.settings.get(setting_id)
        } else {
            None
        }
    }

    pub fn get_section(&self, section_id: &str) -> Option<&SectionManifestValue> {
        self.setting_sections.get(section_id)
    }

    pub fn get_all_sections(&self) -> BTreeMap<String, SectionManifestValue> {
        self.setting_sections.clone()
    }
}

impl SettingManifest {
    pub fn validate_setting(&self, value: &Option<ConfigurableValue>) -> Result<(), Error> {
        if let Some(value) = value {
            self.value_type.type_check(value)
        } else if self.is_required {
            Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Setting is required"),
            })
        } else {
            Ok(())
        }
    }
}

impl SectionManifest {
    pub fn validate_section(&self, value: &SectionManifestValue) -> Result<(), Error> {
        for (setting_id, setting_value) in value.settings.iter() {
            if let Some(setting) = self.settings.get(setting_id) {
                setting.validate_setting(&setting_value.value)?;
            } else {
                return Err(Error {
                    kind: ErrorKind::BadRequest,
                    source: eyre!("Setting not found"),
                });
            }
        }
        Ok(())
    }
}

impl ConfigurableManifest {
    pub fn validate_manifest(&self, value: &ManifestValue) -> Result<(), Error> {
        for (section_id, section_value) in value.setting_sections.iter() {
            if let Some(section) = self.setting_sections.get(section_id) {
                section.validate_section(section_value)?;
            } else {
                return Err(Error {
                    kind: ErrorKind::BadRequest,
                    source: eyre!("Section not found"),
                });
            }
        }
        Ok(())
    }
}