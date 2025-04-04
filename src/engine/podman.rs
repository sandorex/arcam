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

// NOTE: this is so i do not have to have all the properties at Option<T> if they are null
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
            .log_output(log::Level::Debug)?;

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

        let output = cmd.log_output(log::Level::Debug)?;

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
            .log_output(log::Level::Debug)?;

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
            .log_output(log::Level::Debug)?;

        match output.get_code() {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(anyhow!("Error checking if container {container:?} exists")),
        }
    }

    #[cfg(test)]
    fn start_dummy_container(
        &self,
        image: &str,
        args: Option<Vec<&str>>,
    ) -> Result<crate::tests_prelude::Container> {
        assert!(!image.is_empty());

        let mut cmd = self.command();
        cmd.args(["run", "--rm", "-d", "-it"]);

        if let Some(args) = args {
            cmd.args(args);
        }

        // image goes last
        cmd.arg(image);

        let output = cmd.log_output(log::Level::Debug)?;

        Ok(crate::tests_prelude::Container {
            container: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            engine: Box::new(*self),
        })
    }

    #[cfg(test)]
    fn stop_container(&self, container: &str) -> Result<()> {
        assert!(!container.is_empty());

        // gentle shutdown, terminates by default after 10s
        self.command()
            .args(["container", "stop", container])
            .log_status(log::Level::Debug)?;

        Ok(())
    }
}

impl Display for Podman {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: this is truncated output from `podman inspect`, removed few labels cause of laziness
    const INSPECT_OUTPUT: &str = include_str!("podman_inspect.json");

    #[test]
    #[ignore]
    fn engine_inspect_podman() -> Result<()> {
        let obj = serde_json::from_str::<Vec<PodmanContainerInfo>>(INSPECT_OUTPUT)?;
        assert_eq!(
            obj.first().take().unwrap(),
            &PodmanContainerInfo {
                name: "wrathful-arcam".to_string(),
                config: PodmanContainerInfoConfig {
                    labels: HashMap::from([
                        ("arcam".to_string(), "0.1.10".to_string()),
                        (
                            "com.github.containers.toolbox".to_string(),
                            "true".to_string()
                        ),
                        (
                            "container_dir".to_string(),
                            "/home/sandorex/ws/arcam".to_string()
                        ),
                        ("default_shell".to_string(), "/bin/fish".to_string()),
                        (
                            "host_dir".to_string(),
                            "/mnt/slowmf/ws/projects/arcam".to_string()
                        ),
                    ]),
                },
            }
        );

        let container = Podman.start_dummy_container("debian:trixie", None)?;

        // ensure some data is extracted
        assert!(!Podman.inspect_containers(vec![&container])?.is_empty());

        Ok(())
    }

    #[test]
    #[ignore]
    fn engine_exists_podman() -> Result<()> {
        let container = Podman.start_dummy_container("debian:trixie", None)?;

        assert!(Podman.container_exists(&container)?);

        let inspected = Podman.inspect_containers(vec![&container])?;
        assert!(!inspected.is_empty());

        Ok(())
    }
}
