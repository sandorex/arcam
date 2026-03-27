use crate::common::prelude::*;
use std::collections::HashMap;
use arcam::engine::{Podman, PodmanContainerInfo, PodmanContainerInfoConfig};

// NOTE: this is truncated output from `podman inspect`, removed few labels cause of laziness
const INSPECT_OUTPUT: &str = include_str!("podman_inspect.json");

#[test]
#[ignore = "requires podman"]
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

    let container = Podman.start_dummy_container(DEBIAN_IMAGE, None)?;

    // ensure some data is extracted
    assert!(!Podman.inspect_containers(vec![&container])?.is_empty());

    Ok(())
}

#[test]
#[ignore = "requires podman"]
fn engine_exists_podman() -> Result<()> {
    let container = Podman.start_dummy_container(DEBIAN_IMAGE, None)?;

    assert!(Podman.container_exists(&container)?);

    let inspected = Podman.inspect_containers(vec![&container])?;
    assert!(!inspected.is_empty());

    Ok(())
}
