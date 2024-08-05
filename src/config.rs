/// Contains everything related to container configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    Message(String),
    Generic(Box<dyn Error>),
    File(String, Box<dyn Error>),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{}", msg),
            Self::Generic(x) => x.fmt(f),
            Self::File(file, err) => {
                write!(f, "Config error in file {}: ", file)?;
                err.fmt(f)?;

                Ok(())
            },
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Message(_) => None,
            Self::Generic(x) => Some(x.as_ref()),
            Self::File(_, x) => Some(x.as_ref()),
        }
    }
}

/// Whole config file
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ConfigFile {
    /// Version of the configuration
    pub version: Option<u64>,

    /// All container configs
    pub config: Option<Vec<Config>>,
}

impl ConfigFile {
    /// Loads config from str, path is just for error message and can be anything
    pub fn load_from_str(text: &str) -> Result<Self, ConfigError> {
        let obj = toml::from_str::<ConfigFile>(text)
            .map_err(|err| ConfigError::Generic(Box::new(err)) )?;

        let version = obj.version.unwrap_or(1);
        if version != 1 {
            return Err(
                ConfigError::Message(format!("Invalid schema version {}", version)),
            )
        }

        Ok(obj)
    }

    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let file_contents = std::fs::read_to_string(path)
            .map_err(|err| ConfigError::File(path.to_string(), Box::new(err)))?;

        Self::load_from_str(&file_contents)
            .map_err(|err| ConfigError::File(path.to_string(), Box::new(err)))
    }
}

/// Single configuration for a container, contains default settings and optional settings per
/// engine that get applied over the default settings
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Config {
    // TODO figure out rules for local containers that need to be built
    /// Name of the configuration
    pub name: String,

    /// Image used for the container
    pub image: String,

    /// Optional name to set for the container, otherwise randomly generated
    pub container_name: Option<String>,

    /// Dotfiles directory to use as /etc/skel
    pub dotfiles: Option<String>,

    /// Should the container have access to internet
    #[serde(default)]
    pub network: bool,

    /// Default setting used regardless of the engine
    #[serde(default)]
    pub default: EngineConfig,

    /// Override default settings if the engine is podman
    #[serde(default)]
    pub podman: EngineConfig,

    /// Override default settings if the engine is docker
    #[serde(default)]
    pub docker: EngineConfig,
}

impl Config {
    pub fn get_engine_config(&self, engine: &crate::util::Engine) -> &EngineConfig {
        match &engine.kind {
            crate::util::EngineKind::Podman => &self.podman,
            crate::util::EngineKind::Docker => &self.docker,
        }
    }
}

// TODO create conversion between cli args and this, so one could generate it from cmd args
/// Container arguments for specific engine
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct EngineConfig {
    // NOTE keep it simple, do not add unecessary wrappers for arguments

    /// Arguments to pass to the engine, some $VARS are expanded
    #[serde(default)]
    pub engine_args: Vec<String>,

    /// Capabilties to add / remove for the container
    ///
    /// For example `!cap_net_broadcast` disables the capability
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>
}

/// Load and merge configs from directory (loads *.toml file)
pub fn load_from_dir(path: &str) -> Result<HashMap<String, Config>, ConfigError> {
    let mut configs: HashMap<String, Config> = HashMap::new();

    let toml_files: Vec<std::path::PathBuf> = std::path::Path::new(path)
        .read_dir()
        .map_err(|err| ConfigError::Message(format!("Error reading config directory {}: {}", path, err)))?
        .map(|x| x.unwrap().path() )
        .filter(|x| x.extension().unwrap_or_default() == "toml")
        .collect();

    for file in toml_files {
        let config_file = ConfigFile::load_from_file(file.display().to_string().as_str())?;

        for config in config_file.config.unwrap_or_default() {
            // ignore any duplicates, let the user handle it if they wish
            if configs.contains_key(&config.name) {
                eprintln!("Ignoring duplicate config {} in {}", &config.name, file.display());
                continue;
            }

            configs.insert(config.name.clone(), config);
        }
    }

    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        let cfg_text = r#"
[[config]]
name = "first"
image = "fedora"

[config.default]
engine_args = [ "default" ]

[config.podman]
engine_args = [ "podman" ]

[config.docker]
engine_args = [ "docker" ]
"#;

        let result = ConfigFile::load_from_str(cfg_text);
        assert!(result.is_ok(), "result is err: {}", result.unwrap_err());
        let result_ok = result.unwrap();

        assert_eq!(result_ok, ConfigFile {
            version: None,
            config: Some(vec![
                Config {
                    name: "first".into(),
                    image: "fedora".into(),

                    default: EngineConfig {
                        engine_args: vec!["default".into()],
                        ..Default::default()
                    },

                    podman: EngineConfig {
                        engine_args: vec!["podman".into()],
                        ..Default::default()
                    },

                    docker: EngineConfig {
                        engine_args: vec!["docker".into()],
                        ..Default::default()
                    },

                    ..Default::default()
                },
            ]),
        });
    }
}

