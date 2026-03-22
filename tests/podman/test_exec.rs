use crate::common::prelude::*;
use assert_cmd::prelude::*;
use std::process::Command;
use arcam::engine::Podman;

// TODO test environment passing and --shell

#[test]
#[ignore = "requires podman"]
fn cmd_exec_podman() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

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

    // create file in cwd
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["exec", &container, "--", "touch", "file.txt"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    assert!(tempdir.path().join("file.txt").exists(), "File not created");

    Ok(())
}
