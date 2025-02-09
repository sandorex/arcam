use assert_cmd::Command;
use crate::util::tests::prelude::*;

#[test]
fn test_cmd_start_podman() -> Result<()> {
    let tempdir = test_temp_dir::test_temp_dir!();

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

    // try to start another container in same directory
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["start", "debian:trixie"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .failure()
        .stderr(format!("Error: There are containers running in current directory: {}\n", container_name));

    Command::new("podman")
        .args(["container", "exists", container_name])
        .assert()
        .success();

    Ok(())
}
