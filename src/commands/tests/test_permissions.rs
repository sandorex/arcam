use assert_cmd::Command;
use users::{get_current_gid, get_current_uid};
use crate::util::tests::prelude::*;

fn run(container_name: &str, command: &[&str]) -> Result<Command> {
    let mut cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?;
    cmd.args(["exec", container_name, "--"]);
    cmd.args(command);

    Ok(cmd)
}

#[test]
fn test_permissions_podman() -> Result<()> {
    let tempdir = test_temp_dir::test_temp_dir!();

    // create the container
    let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["start", "debian:trixie"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    let container_name = String::from_utf8_lossy(&cmd.get_output().stdout);
    let container_name = container_name.trim();

    println!("Container {:?}", container_name);

    // kill container on drop
    let _container = podman_cleanup(container_name);

    // check if uid/gid are the same
    run(container_name, &["stat", "-c", "%u %g", "."])?
        .assert()
        .stdout(format!("{} {}\r\n", get_current_uid(), get_current_gid()));

    Ok(())
}
