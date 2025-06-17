//! Contains everything related to container configuration

mod v25_06;
use v25_06::Config2506;

/// Latest config struct
pub type Config = Config2506;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "version")]
pub enum ConfigFile {
    #[serde(rename = "25.06")]
    V25_06(Config2506),
}

impl TryInto<Config> for ConfigFile {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<Config, Self::Error> {
        match self {
            Self::V25_06(x) => Ok(x.try_into()?),
        }
    }
}

impl ConfigFile {
    pub fn config_from_str(input: &str) -> Result<Config> {
        Ok(toml::from_str::<ConfigFile>(input)?.try_into()?)
    }

    pub fn config_from_file(file: &Path) -> Result<Config> {
        let file_contents = std::fs::read_to_string(file)
            .with_context(|| format!("while reading config file {:?}", file))?;

        Self::config_from_str(&file_contents)
            .with_context(|| format!("while parsing config file {:?}", file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        let cfg_text = r#"
version = "25.06"
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
