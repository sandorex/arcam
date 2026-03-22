use crate::common::prelude::*;
use assert_cmd::prelude::*;
use rexpect::session::spawn_command;
use std::process::Command;
use arcam::engine::Podman;

#[test]
#[ignore = "requires podman"]
fn cmd_kill_podman() -> Result<()> {
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

    // it should exist now
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["exists"])
        .current_dir(tempdir.path())
        .assert()
        .success();

    // test with --yes
    Command::cargo_bin(arcam::APP_NAME)?
        .args(["kill", "-y", &container])
        .current_dir(tempdir.path())
        .assert()
        .success();

    Ok(())
}

#[test]
#[ignore = "requires podman"]
fn cmd_kill_interactive_podman() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

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

    // try to kill to get the prompt
    let mut pty = {
        let mut c = Command::cargo_bin(arcam::APP_NAME)?;
        c.args(["kill", &container]);

        spawn_command(c, Some(5_000))
    }?;

    let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
    assert!(
        matched.contains(&*container),
        "Wrong container name in prompt?"
    );

    // send enter? so i check the default action
    pty.send_line("")?;
    pty.exp_eof()?;

    // check if container is still running, as it should be
    Command::new("podman")
        .args(["container", "exists", &container])
        .assert()
        .success();

    // run again and answer y
    let mut pty = {
        let mut c = Command::cargo_bin(arcam::APP_NAME)?;
        c.args(["kill", &container]);

        spawn_command(c, Some(5_000))
    }?;

    let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
    assert!(
        matched.contains(&*container),
        "Wrong container name in prompt?"
    );

    // send enter? so i check the default action
    pty.send_line("y")?;
    pty.exp_eof()?;

    // check if container is still running, it should not be at this point
    Command::new("podman")
        .args(["container", "exists", &container])
        .assert()
        .failure();

    Ok(())
}
