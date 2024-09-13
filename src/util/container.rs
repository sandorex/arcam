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
        .expect(crate::ENGINE_ERR_MSG);

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

/// Check if running inside a container
pub fn is_in_container() -> bool {
    use std::path::Path;
    use std::env;

    return Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
        || env::var("container").is_ok()
}

pub fn container_exists(engine: &Engine, container: &str) -> bool {
    Command::new(&engine.path)
        .args(["container", "exists", container])
        .output()
        .expect(crate::ENGINE_ERR_MSG)
        .status.success()
}

pub fn get_container_ws(engine: &Engine, container: &str) -> Option<String> {
    let key = format!(".Config.Labels.{}", crate::APP_NAME);

    // this looks like a mess as i need to escape curly braces
    //
    // basically return key if it exists
    let format = format!("{{{{ if {0} }}}}{{{{ {0} }}}}{{{{ end }}}}", key);

    // {{if .. }} is added so that the stdout is empty if ws is none
    let cmd = Command::new(&engine.path)
        .args(["inspect", container, "--format", format.as_str()])
        .output()
        .expect(crate::ENGINE_ERR_MSG);

    if cmd.status.success() {
        let stdout = String::from_utf8(cmd.stdout).unwrap();
        let trimmed = stdout.trim();

        // check if stdout is empty (will have some whitespace though!)
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        None
    }
}

/// Returns name of container that was started in same host directory
pub fn find_containers_by_cwd(engine: &Engine) -> Option<Vec<String>> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let cmd = Command::new(&engine.path)
        .args(["container", "ls", "--format", "{{.Names}}", "--sort", "created"])
        .args(["--filter".into(), format!("label=host_dir={}", &cwd.to_string_lossy())])
        .output()
        .expect(crate::ENGINE_ERR_MSG);

    if cmd.status.success() {
        let stdout = String::from_utf8(cmd.stdout).unwrap();
        let trimmed = stdout.trim();

        // check if stdout is empty
        if trimmed.is_empty() {
            // return empty vec to signify none were found
            Some(vec![])
        } else {
            // collect the lines
            // NOTE reversing to get youngest container first
            Some(trimmed.lines().rev().map(|x| x.to_string()).collect())
        }
    } else {
        // could not get the containers for some reason
        None
    }
}

