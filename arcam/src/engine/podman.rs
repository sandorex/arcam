use super::{ContainerInfo, Engine};
use crate::command_extensions::*;
use crate::prelude::*;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfoConfig {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfo {
    pub name: String,
    pub config: PodmanContainerInfoConfig,
}

impl From<PodmanContainerInfo> for ContainerInfo {
    fn from(value: PodmanContainerInfo) -> Self {
        Self {
            name: value.name,
            labels: value.config.labels,
        }
    }
}

// NOTE: this is so i do not have to have all the properties as Option<T> if they are null
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Implementation for podman engine manager
#[derive(Debug, Clone, Copy)]
pub struct Podman;

impl Engine for Podman {
    fn name(&self) -> &str {
        "podman"
    }

    fn exec(&self, container: &str, command: &[&str]) -> Result<String> {
        assert!(!container.is_empty());
        assert!(!command.is_empty());

        let output = self
            .command()
            .args(["exec", "--user", "root", container])
            .args(command)
            .log_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_containers(&self, labels: Vec<(&str, Option<&str>)>) -> Result<Vec<String>> {
        let mut cmd = self.command();

        // just print names of the containers
        cmd.args(["container", "ls", "--format", "{{ .Names }}"]);

        for (key, val) in labels {
            if let Some(val) = val {
                cmd.arg(format!("--filter=label={key}={val}"));
            } else {
                cmd.arg(format!("--filter=label={key}"));
            }
        }

        let output = cmd.log_output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|x| x.to_string())
            .collect())
    }

    fn inspect_containers(&self, containers: Vec<&str>) -> Result<Vec<ContainerInfo>> {
        assert!(!containers.is_empty());

        let output = self
            .command()
            .args(["container", "inspect"])
            .args(containers)
            .log_output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // deserialize into podman specific struct then convert into the generic one
        Ok(serde_json::from_str::<Vec<PodmanContainerInfo>>(&stdout)
            .with_context(|| "Error parsing output from \"podman inspect\"")?
            .into_iter()
            .map(Into::<ContainerInfo>::into)
            .collect())
    }

    fn container_exists(&self, container: &str) -> Result<bool> {
        assert!(!container.is_empty());

        let output = self
            .command()
            .args(["container", "exists", container])
            .log_output()?;

        match output.get_code() {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(anyhow!("Error checking if container {container:?} exists")),
        }
    }

    fn image_exists(&self, image: &str) -> Result<bool> {
        let cmd = self
            .command()
            .args(["image", "exists", image])
            .log_output()?;

        match cmd.get_code() {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(anyhow!("Error checking does image {image:?} exist")),
        }
    }

    fn image_pull(&self, image: &str, interactive: bool) -> Result<()> {
        let mut cmd = self.command();
        cmd.args(["image", "pull", image]);

        if interactive {
            // print to stderr to prevent issues with reading container name
            cmd.stdout(std::io::stderr());
            cmd.log_status_anyhow()?;
        } else {
            cmd.log_output_anyhow()?;
        }

        Ok(())
    }
}

impl Display for Podman {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

