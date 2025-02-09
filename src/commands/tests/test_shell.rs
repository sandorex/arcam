use std::process::Command;
use assert_cmd::prelude::*;
use crate::util::tests::prelude::*;
use rexpect::session::{PtyReplSession, spawn_command};

#[test]
fn test_cmd_shell_podman() -> Result<()> {
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

    let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    c.args(["shell"]);
    c.current_dir(tempdir.as_path_untracked());

    let mut pty = spawn_command(c, Some(5_000)).and_then(|p| {
        let mut session = PtyReplSession {
            prompt: "$".to_owned(),
            pty_session: p,
            quit_command: Some("exit".to_owned()),
            echo_on: true,
        };

        // wait until the prompt appears
        session.wait_for_prompt()?;

        // set prompt to something simple
        session.send_line("export PS1='$ '")?;

        // wait for prompt again
        session.wait_for_prompt()?;

        Ok(session)
    })?;

    // check if its running properly
    pty.send_line("echo $ARCAM_VERSION")?;
    pty.exp_string(env!("CARGO_PKG_VERSION"))?;
    pty.wait_for_prompt()?;

    Ok(())
}

#[test]
fn test_cmd_start_shell_podman() -> Result<()> {
    let tempdir = test_temp_dir::test_temp_dir!();

    // in this case i set name in advance so the container always gets killed
    let container_name = "test_arcam";

    // kill container on drop
    let _container = podman_cleanup("test_arcam");
    println!("Container {:?}", container_name);

    let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    c.args(["start", "--name", "test_arcam", "-E", "debian:trixie"]);
    c.current_dir(tempdir.as_path_untracked());

    let mut pty = spawn_command(c, Some(5_000)).and_then(|p| {
        let mut session = PtyReplSession {
            prompt: "$".to_owned(),
            pty_session: p,
            quit_command: Some("exit".to_owned()),
            echo_on: true,
        };

        // wait until the prompt appears
        session.wait_for_prompt()?;

        // set prompt to something simple
        session.send_line("export PS1='$ '")?;

        // wait for prompt again
        session.wait_for_prompt()?;

        Ok(session)
    })?;

    // check if its running properly
    pty.send_line("echo $ARCAM_VERSION")?;
    pty.exp_string(env!("CARGO_PKG_VERSION"))?;
    pty.wait_for_prompt()?;

    Ok(())
}
