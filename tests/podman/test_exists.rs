use crate::common::prelude::*;
use assert_cmd::prelude::*;
use std::process::Command;
use arcam::engine::Podman;

#[test]
#[ignore = "requires podman"]
fn cmd_exists_podman() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    // no cwd containers yet
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["exists"])
        .current_dir(tempdir.path())
        .assert()
        .failure()
        .code(1);

    // create the container
    let cmd = Command::cargo_bin(arcam::APP_NAME)?
        .args(["start", DEBIAN_IMAGE])
        .current_dir(tempdir.path())
        .assert()
        .success();

    let container = Container {
        engine: Box::new(Podman),
        container: String::from_utf8_lossy(&cmd.get_output().stdout)
            .trim()
            .to_string(),
    };

    assert!(!container.is_empty(), "Container name is empty");

    // test with explicitly set container_name
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["exists", &container])
        .assert()
        .success();

    // detect container from cwd
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["exists"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    Ok(())
}
