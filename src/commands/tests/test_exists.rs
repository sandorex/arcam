use assert_cmd::Command;
use crate::util::tests::prelude::*;

#[test]
fn test_cmd_exists_podman() -> Result<()> {
    let tempdir = test_temp_dir::test_temp_dir!();

    // it should not exist yet
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .failure()
        .code(1);

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

    // it should exist now
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    // now test with the container name
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists", container_name])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    Ok(())
}
