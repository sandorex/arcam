use crate::common::prelude::*;
use assert_cmd::prelude::*;
use rexpect::session::{spawn_command, PtyReplSession};
use std::process::Command;
use arcam::engine::Podman;

#[test]
#[ignore = "requires podman"]
fn cmd_shell_podman() -> Result<()> {
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

    let mut c = Command::cargo_bin(arcam::APP_NAME)?;
    c.args(["shell", &container]);
    c.current_dir(tempdir.path());

    let mut pty = spawn_command(c, Some(5_000)).and_then(|p| {
        let mut session = PtyReplSession::new(p, "$".to_owned())
            .echo_on(true)
            .quit_command(Some("exit".to_owned()));

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
