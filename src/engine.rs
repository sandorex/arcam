//! Engine specific abstraction

use crate::command_extensions::*;
use crate::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;

/// Engine agnostic container info
pub trait ContainerInfo {
    /// Get container name
    fn get_name(&self) -> &str;

    /// Get container label
    fn get_label(&self, name: &str) -> Option<&str>;

    /// Check if container label exists
    fn has_label(&self, name: &str) -> bool {
        self.get_label(name).is_some()
    }

    /// Get all labels as a hashmap
    fn labels(&self) -> &HashMap<String, String>;
}

#[derive(Debug, Clone, Copy)]
pub enum Engine {
    Podman,
}

impl Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

// NOTE these should be expanded as needed, i do not need all the data
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfoConfig {
    labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfo {
    name: String,
    config: PodmanContainerInfoConfig,
}

impl ContainerInfo for PodmanContainerInfo {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_label(&self, name: &str) -> Option<&str> {
        self.config.labels.get(name).map(|x| x.as_str())
    }

    fn labels(&self) -> &HashMap<String, String> {
        &self.config.labels
    }
}

impl Engine {
    /// Returns lowercase name of the engine
    pub fn name(&self) -> &'static str {
        "podman"
    }

    /// Execute command as root inside a container
    pub fn exec<T: AsRef<std::ffi::OsStr>>(
        &self,
        container: &str,
        command: &[T],
    ) -> Result<String> {
        assert!(!container.is_empty());
        assert!(!command.is_empty());

        let output = self
            .command()
            .args(["exec", "--user", "root", container])
            .args(command)
            .log_output(log::Level::Debug)?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Creates `std::process::Command` with program_name being the engine path
    pub fn command(&self) -> Command {
        Command::new(self.name())
    }

    /// Returns list of all containers filtered by label, and optionally value
    pub fn get_containers(&self, labels: Vec<(&str, Option<&str>)>) -> Result<Vec<String>> {
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

    /// Inspects containers and returns the data associated
    pub fn inspect_containers(&self, containers: Vec<&str>) -> Result<Vec<impl ContainerInfo>> {
        assert!(!containers.is_empty());

        let output = self
            .command()
            .args(["container", "inspect"])
            .args(containers)
            .log_output(log::Level::Debug)?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::from_str::<Vec<PodmanContainerInfo>>(&stdout)
            .with_context(|| "Error parsing output from \"podman inspect\"")?)
    }

    /// Check if container exists, should be faster than `inspect_container`
    pub fn container_exists(&self, container: &str) -> Result<bool> {
        assert!(!container.is_empty());

        // TODO log this with log::trace!
        let output = self
            .command()
            .args(["container", "exists", container])
            .output()?;

        match output.get_code() {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(anyhow!(
                "Error checking if container {:?} exists",
                container
            )),
        }
    }

    // TODO return crate::commands::tests::Container::Podman
    #[cfg(test)]
    pub fn start_dummy_container(&self, image: &str, args: Option<Vec<&str>>) -> Result<String> {
        assert!(!image.is_empty());

        let mut cmd = self.command();
        cmd.args(["run", "--rm", "-d", "-it"]);

        if let Some(args) = args {
            cmd.args(args);
        }

        // image goes last
        cmd.arg(image);

        let output = cmd.log_output(log::Level::Debug)?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Gently shutdown a container, after a timeout kill it forcefully if still running
    pub fn stop_container(&self, container: &str) -> Result<()> {
        assert!(!container.is_empty());

        // gentle shutdown, terminates by default after 10s
        self.command()
            .args(["container", "stop", container])
            .log_status(log::Level::Debug)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: this is truncated output from `podman inspect`, removed few labels cause of laziness
    const INSPECT_OUTPUT: &str = include_str!("engine/podman_inspect.json");

    #[test]
    fn podman_inspect_parsing() -> Result<()> {
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

        Ok(())
    }
}
