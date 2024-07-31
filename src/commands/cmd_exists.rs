use crate::cli;
use std::process::Command;

pub enum ContainerStatus {
    Created,
    Exited,
    Paused,
    Running,
    Unknown,
}

pub fn get_container_status(engine: &str, container: &str) -> Result<ContainerStatus, ()> {
    let cmd = Command::new(engine)
        .args(&["container", "inspect", container, "--format", "{{.State.Status}}"])
        .output()
        .expect("Could not execute engine");

    // the container does not exist
    if ! cmd.status.success() {
        return Err(());
    }

    let stdout = String::from_utf8_lossy(&cmd.stdout).to_string();
    Ok(match stdout.as_str() {
        "created" => ContainerStatus::Created,
        "exited" => ContainerStatus::Exited,
        "paused" => ContainerStatus::Paused,
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Unknown,
    })
}

pub fn container_exists(engine: &str, cli_args: &cli::CmdExistsArgs) -> u8 {
    // whatever state it is, it exists
    match get_container_status(engine, &cli_args.container) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
