//! Contains everything related to container configuration

use code_docs::{code_docs_struct, DocumentedStruct};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use crate::util::Engine;

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
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    /// Version of the configuration
    #[serde(default = "default_version")]
    pub version: u64,

    /// All container configs
    pub config: Option<Vec<Config>>,
}

// version 1 is gonna be default for now
const fn default_version() -> u64 { 1 }

impl ConfigFile {
    /// Loads config from str, path is just for error message and can be anything
    pub fn load_from_str(text: &str) -> Result<Self, ConfigError> {
        // TODO load a table first and get the version then try parsing appropriate struct

        let obj = toml::from_str::<ConfigFile>(text)
            .map_err(|err| ConfigError::Generic(Box::new(err)) )?;

        match obj.version {
            1 => Ok(obj),
            _ => Err(ConfigError::Message(format!("Invalid schema version {}", obj.version))),
        }
    }

    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let file_contents = std::fs::read_to_string(path)
            .map_err(|err| ConfigError::File(path.to_string(), Box::new(err)))?;

        Self::load_from_str(&file_contents)
            .map_err(|err| ConfigError::File(path.to_string(), Box::new(err)))
    }
}

// save all the fields and docs so they can be printed as always up-to-date documentation
code_docs_struct! {
    /// Single configuration for a container, contains default settings and optional settings per
    /// engine that get applied over the default settings
    #[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        // TODO redo these comments so they are easy to understand even for non-rust programmers
        /// Name of the configuration
        pub name: String,

        /// Image used for the container
        pub image: String,

        /// Optional name to set for the container, otherwise randomly generated
        pub container_name: Option<String>,

        /// Optional path to directory to use as /etc/skel (static dotfiles)
        ///
        /// Environ vars are expanded
        pub skel: Option<String>,

        /// Set network access
        #[serde(default)]
        pub network: bool,

        /// Passthrough pulseaudio, security impact is unknown
        #[serde(default)]
        pub audio: bool,

        /// Passthrough wayland compositor socket, high security impact, allows clipboard access
        #[serde(default)]
        pub wayland: bool,

        /// Passthrough ssh-agent socket, security impact is unknown
        #[serde(default)]
        pub ssh_agent: bool,

        /// Passthrough D-BUS session bus, maximum security impact allows arbitrary code execution
        #[serde(default)]
        pub session_bus: bool,

        /// Run command before all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_pre: Vec<String>,

        /// Run command after all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_post: Vec<String>,

        /// Environment variables to set
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub env: HashMap<String, String>,

        /// Args passed to the engine
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub engine_args: Vec<String>,

        /// Args passed to the engine, if its podman
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub engine_args_podman: Vec<String>,

        /// Args passed to the engine, if its docker
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub engine_args_docker: Vec<String>,
    }
}

impl Config {
    /// Get engine args for specific engine
    pub fn get_engine_args(&self, engine: &Engine) -> &Vec<String> {
        match engine.kind {
            crate::util::EngineKind::Podman => &self.engine_args_podman,
            crate::util::EngineKind::Docker => &self.engine_args_docker,
        }
    }
}

/// Load and merge configs from directory (loads *.toml file)
pub fn load_from_dir(path: &str) -> Result<HashMap<String, Config>, ConfigError> {
    let mut configs: HashMap<String, Config> = HashMap::new();

    // the directory does not exist just exit quietly
    if !std::path::Path::new(path).exists() {
        return Ok(configs);
    }

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
        // TODO write all the keys here to preserve compatability in the future
        let cfg_text = r#"
[[config]]
name = "first"
image = "fedora"
engine_args = [ "default" ]
engine_args_podman = [ "podman" ]
engine_args_docker = [ "docker" ]
"#;

        let result = ConfigFile::load_from_str(cfg_text);
        assert!(result.is_ok(), "result is err: {}", result.unwrap_err());
        let result_ok = result.unwrap();

        assert_eq!(result_ok, ConfigFile {
            version: default_version(),
            config: Some(vec![
                Config {
                    name: "first".into(),
                    image: "fedora".into(),
                    engine_args: vec!["default".into()],
                    engine_args_podman: vec!["podman".into()],
                    engine_args_docker: vec!["docker".into()],

                    ..Default::default()
                },
            ]),
        });
    }
}

