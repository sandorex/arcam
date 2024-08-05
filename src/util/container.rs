use std::process::Command;
use super::Engine;

/// Possible status of a container
#[derive(Debug)]
pub enum ContainerStatus {
    Created,
    Exited,
    Paused,
    Running,
    Unknown,
}

/// Get container status if it exists
pub fn get_container_status(engine: &Engine, container: &str) -> Option<ContainerStatus> {
    let cmd = Command::new(&engine.path)
        .args(["container", "inspect", container, "--format", "{{.State.Status}}"])
        .output()
        .expect("Could not execute engine");

    // the container does not exist
    if ! cmd.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&cmd.stdout).to_string();
    Some(match stdout.as_str() {
        "created" => ContainerStatus::Created,
        "exited" => ContainerStatus::Exited,
        "paused" => ContainerStatus::Paused,
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Unknown,
    })
}

/// Check if container is owned by box, will return false if container does not exist
pub fn is_box_container(engine: &Engine, name: &str) -> bool {
    let cmd = Command::new(&engine.path)
        .args(["container", "inspect", name, "--format", "{{if .Config.Labels.box}}{{.Config.Labels.box}}{{end}}"])
        .output()
        .expect("Could not execute engine");

    cmd.status.success() && !String::from_utf8_lossy(&cmd.stdout).is_empty()
}

/// Check if running inside a container
pub fn is_in_container() -> bool {
    use std::path::Path;
    use std::env;

    return Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
        || env::var("container").is_ok()
}

