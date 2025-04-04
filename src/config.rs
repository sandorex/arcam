//! Contains everything related to container configuration

use anyhow::{Context, Result};
use code_docs::{code_docs_struct, DocumentedStruct};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Whole config file
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    /// Version of the configuration
    pub version: u64,

    #[serde(flatten)]
    pub config: Config,
}

impl ConfigFile {
    // TODO handle different future versions of config
    pub fn config_from_str(input: &str) -> Result<Config> {
        Ok(toml::from_str::<ConfigFile>(input)?.config)
    }

    pub fn config_from_file(file: &Path) -> Result<Config> {
        let file_contents = std::fs::read_to_string(file)
            .with_context(|| format!("while reading config file {:?}", file))?;

        Self::config_from_str(&file_contents)
            .with_context(|| format!("while parsing config file {:?}", file))
    }
}

// save all the fields and docs so they can be printed as always up-to-date documentation
code_docs_struct! {
    /// Single configuration for a container, contains default settings and optional settings per
    /// engine that get applied over the default settings
    #[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Image used for the container
        pub image: String,

        /// Optional path to directory to use as /etc/skel (static dotfiles)
        ///
        /// Environ vars are expanded
        pub skel: Option<String>,

        /// Default user shell
        #[serde(default)]
        pub shell: Option<String>,

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

        /// Path to mount as a volume, basically shorthand for `--volume=<name>:<path>`
        #[serde(default)]
        pub persist: Vec<(String, String)>,

        /// Same as `persist` but the path is chowned as user on init
        #[serde(default)]
        pub persist_user: Vec<(String, String)>,

        /// Run command before all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_pre: Option<String>,

        /// Run command after all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_post: Option<String>,

        /// Script to run on start, all original arguments are passed verbatim, you have to run
        /// `arcam start` yourself or nothing will happen
        ///
        /// NOTE: the script is ran using "/bin/sh"
        pub host_pre_init: Option<String>,

        /// Pass through container port to host (both TCP and UDP)
        ///
        /// Not all ports are allowed with rootless podman
        #[serde(default)]
        pub ports: Vec<(u32, u32)>,

        /// Environment variables to set (name, value)
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub env: Vec<(String, String)>,

        /// Add capabilities, or drop them with by prefixing `!cap`
        ///
        /// For more details about capabilities read `man 7 capabilities`
        #[serde(default)]
        pub capabilities: Vec<String>,

        /// Args passed to the engine
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub engine_args: Vec<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        let cfg_text = r#"
version = 1
image = "fedora"
engine_args = [ "default" ]
"#;

        let result = ConfigFile::config_from_str(cfg_text);
        assert!(result.is_ok(), "result is err: {}", result.unwrap_err());
        let result_ok = result.unwrap();

        assert_eq!(
            result_ok,
            Config {
                image: "fedora".into(),
                engine_args: vec!["default".into()],

                ..Default::default()
            }
        );
    }
}
