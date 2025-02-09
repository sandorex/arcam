use std::process::Command;
use assert_cmd::prelude::*;
use crate::util::tests::prelude::*;
use rexpect::session::spawn_command;

#[test]
fn test_cmd_kill_podman() -> Result<()> {
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

    // it should exist now
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["exists"])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    // test with --yes
    Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["kill", "-y", container_name])
        .current_dir(tempdir.as_path_untracked())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_cmd_kill_interactive_podman() -> Result<()> {
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

    // try to kill to get the prompt
    let mut pty = {
        let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        c.args(["kill", &container_name]);

        spawn_command(c, Some(5_000))
    }?;

    let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
    assert!(matched.contains(container_name), "Wrong container name in prompt?");

    // send enter? so i check the default action
    pty.send_line("")?;
    pty.exp_eof()?;

    // check if container is still running, as it should be
    Command::new("podman")
        .args(["container", "exists", &container_name])
        .assert()
        .success();

    // run again and answer y
    let mut pty = {
        let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        c.args(["kill", &container_name]);

        spawn_command(c, Some(5_000))
    }?;

    let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
    assert!(matched.contains(container_name), "Wrong container name in prompt?");

    // send enter? so i check the default action
    pty.send_line("y")?;
    pty.exp_eof()?;

    // check if container is still running, it should not be at this point
    Command::new("podman")
        .args(["container", "exists", &container_name])
        .assert()
        .failure();

    Ok(())
}
