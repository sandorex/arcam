use serde::Deserialize;

use super::{Engine, ContainerInfo};
use crate::prelude::*;
use crate::util::command_extensions::*;
use std::collections::HashMap;

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
}

impl Engine {
    pub fn name(&self) -> &'static str {
        "podman"
    }

    pub fn exec<T: AsRef<std::ffi::OsStr>>(&self, container: &str, command: &[T]) -> Result<String> {
        let output = self.command()
            .args(["exec", "--user", "root", container])
            .args(command)
            .run_get_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn command(&self) -> Command {
        Command::new(self.name())
    }

    pub fn get_containers(&self, label: &str, value: Option<&str>) -> Result<Vec<String>> {
        let mut cmd = self.command();

        // just print names of the containers
        cmd.args(["container", "ls", "--format", "{{ .Names }}"]);

        if let Some(value) = value {
            cmd.arg(format!("--filter={label}={value}"));
        } else {
            cmd.arg(format!("--filter={label}"));
        }

        let output = cmd.run_get_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).lines().map(|x| x.to_string()).collect())
    }

    pub fn inspect_container(&self, container: &str) -> Result<impl ContainerInfo> {
        let output = self.command()
            .args(["inspect", container])
            .run_get_output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(
            serde_json::from_str::<PodmanContainerInfo>(&stdout)
                .with_context(|| "Error parsing output from \"podman inspect\"")?
        )
    }

    pub fn container_exists(&self, container: &str) -> Result<bool> {
        // TODO log this with log::trace!
        let output = self.command()
            .args(["container", "exists", container])
            .output()?;

        match output.get_code() {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(anyhow!("Error checking if container {:?} exists", container)),
        }
    }

    pub fn stop_container(&self, container: &str) -> Result<()> {
        // gentle shutdown, terminates by default after 10s
        self.command()
            .args(["container", "stop", container])
            .run_interactive()?;

        Ok(())
    }
}

// #[derive(Debug)]
// pub struct Podman {
//     pub path: String,
// }
//
// impl Engine for Podman {
//     fn name() -> &'static str {
//         "podman"
//     }
//
//     fn exec<T: AsRef<std::ffi::OsStr>>(&self, container: &str, command: &[T]) -> Result<String> {
//         let output = self.command()
//             .args(["exec", "--user", "root", container])
//             .args(command)
//             .run_get_output()?;
//
//         Ok(String::from_utf8_lossy(&output.stdout).to_string())
//     }
//
//     fn command(&self) -> Command {
//         Command::new(&self.path)
//     }
//
//     fn get_containers(&self, label: &str, value: Option<&str>) -> Result<Vec<String>> {
//         let mut cmd = self.command();
//
//         // just print names of the containers
//         cmd.args(["container", "ls", "--format", "{{ .Names }}"]);
//
//         if let Some(value) = value {
//             cmd.arg(format!("--filter={label}={value}"));
//         } else {
//             cmd.arg(format!("--filter={label}"));
//         }
//
//         let output = cmd.run_get_output()?;
//
//         Ok(String::from_utf8_lossy(&output.stdout).lines().map(|x| x.to_string()).collect())
//     }
//
//     fn inspect_container(&self, container: &str) -> Result<impl ContainerInfo> {
//         let output = self.command()
//             .args(["inspect", container])
//             .run_get_output()?;
//
//         let stdout = String::from_utf8_lossy(&output.stdout);
//
//         Ok(
//             serde_json::from_str::<PodmanContainerInfo>(&stdout)
//                 .with_context(|| "Error parsing output from \"podman inspect\"")?
//         )
//     }
//
//     fn container_exists(&self, container: &str) -> Result<bool> {
//         let output = self.command()
//             .args(["exists", container])
//             .output()?;
//
//         match output.get_code() {
//             0 => Ok(true),
//             1 => Ok(false),
//             _ => Err(anyhow!("Error checking if container {:?} exists", container)),
//         }
//     }
//
//     fn stop_container(&self, container: &str) -> Result<()> {
//         // gentle shutdown, terminates by default after 10s
//         self.command()
//             .args(["container", "stop", container])
//             .run_interactive()?;
//
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: this is truncated output from `podman inspect`, removed few labels cause of laziness
    const INSPECT_OUTPUT: &str = include_str!("podman_inspect.json");

    #[test]
    fn test_podman_inspect_parsing() {
        let obj = serde_json::from_str::<Vec<PodmanContainerInfo>>(INSPECT_OUTPUT);
        assert!(obj.is_ok(), "Error parsing: {:?}", obj);
        assert_eq!(
            obj.unwrap().first().take().unwrap(),
            &PodmanContainerInfo {
                name: "wrathful-arcam".to_string(),
                config: PodmanContainerInfoConfig {
                    labels: HashMap::from([
                        ("arcam".to_string(), "0.1.10".to_string()),
                        ("com.github.containers.toolbox".to_string(), "true".to_string()),
                        ("container_dir".to_string(), "/home/sandorex/ws/arcam".to_string()),
                        ("default_shell".to_string(), "/bin/fish".to_string()),
                        ("host_dir".to_string(), "/mnt/slowmf/ws/projects/arcam".to_string()),
                    ]),
                },
            }
        );
    }
}
