use assert_cmd::Command;
use super::prelude::*;
use crate::engine::Engine;

// TODO move this into cmd_exists.rs
#[test]
fn cmd_exists_podman() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    // no cwd containers yet
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists"])
        .current_dir(tempdir.path())
        .assert()
        .failure()
        .code(1);

    // create the container
    let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["start", "debian:trixie"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    let container = Container {
        engine: Engine::Podman,
        container: String::from_utf8_lossy(&cmd.get_output().stdout).trim().to_string(),
    };

    assert!(!container.is_empty(), "Container name is empty");

    // test with explicitly set container_name
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists", &container])
        .assert()
        .success();

    // detect container from cwd
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    Ok(())
}
