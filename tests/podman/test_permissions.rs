use crate::common::prelude::*;
use arcam::engine::Podman;
use users::{get_current_gid, get_current_uid};

fn run(container_name: &str, command: &[&str]) -> Result<Command> {
    let mut cmd = Command::cargo_bin(arcam::APP_NAME)?;
    cmd.args(["exec", container_name, "--"]);
    cmd.args(command);

    Ok(cmd)
}

#[test]
#[ignore = "requires podman"]
fn test_permissions_podman() -> Result<()> {
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

    // check if uid/gid are the same
    run(&container, &["stat", "-c", "%u %g", "."])?
        .assert()
        .stdout(format!("{} {}\r\n", get_current_uid(), get_current_gid()));

    Ok(())
}
