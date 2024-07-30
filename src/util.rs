use std::process::Command;

/// Check whether executable exists in PATH
#[cfg(target_os = "linux")]
pub fn executable_exists(cmd: &str) -> bool {
    let output = Command::new("sh")
        .arg("-c").arg(format!("which {}", cmd))
        .output()
        .expect("failed to execute 'which'");

    output.status.success()
}

/// Finds first available engine, prioritizes podman!
pub fn find_available_engine() -> Option<String> {
    if executable_exists("podman") {
        return Some("podman".to_string());
    }

    if executable_exists("docker") {
        return Some("docker".to_string());
    }

    None
}

/// Helper to get hostname using `hostname` utility which should be available on most linux systems
#[cfg(target_os = "linux")]
pub fn get_hostname() -> String {
    let cmd = Command::new("hostname").output().expect("Could not call hostname");
    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if ! cmd.status.success() || hostname.is_empty() {
        panic!("Unable to get hostname from host");
    }

    hostname.trim().into()
}

