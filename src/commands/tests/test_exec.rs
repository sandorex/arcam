use assert_cmd::Command;
use crate::util::tests::prelude::*;

#[test]
fn test_cmd_exec_podman() -> Result<()> {
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

    // create file in cwd
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exec", container_name, "--", "touch", "file.txt"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    assert!(tempdir.as_path_untracked().join("file.txt").exists(), "File not created");

    Ok(())
}

// TODO test environment passing and --shell
