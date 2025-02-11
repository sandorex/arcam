use assert_cmd::Command;
use crate::engine::Engine;
use super::prelude::*;

// TODO move test into cmd_start.rs
#[test]
fn cmd_start_podman() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["start", "debian:trixie"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    let container = Container {
        engine: Engine::Podman,
        container: String::from_utf8_lossy(&cmd.get_output().stdout).trim().to_string(),
    };

    // try to start another container in same directory
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["start", "--name", &container, "debian:trixie"])
        .current_dir(tempdir.path())
        .assert()
        .failure()
        .stderr(format!("Error: There are containers running in current directory: {:?}\n", container));

    Ok(())
}
