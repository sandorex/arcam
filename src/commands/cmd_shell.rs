use crate::engine::ContainerInfo;
use crate::prelude::*;
use crate::cli;
use crate::command_extensions::*;

pub fn open_shell(ctx: Context, mut cli_args: cli::CmdShellArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        let containers = ctx.get_cwd_containers()?;
        if containers.is_empty() {
            return Err(anyhow!("Could not find a running container in current directory"));
        }

        cli_args.name = containers.first().unwrap().clone();
    } else if !ctx.dry_run && ctx.engine.container_exists(&cli_args.name)? {
        return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
    }

    let container_info = ctx.engine.inspect_containers(vec![&cli_args.name])?;
    let container_info = container_info.first().unwrap();

    let Some(ws_dir) = container_info.get_label(crate::CONTAINER_LABEL_CONTAINER_DIR) else {
        return Err(anyhow!("Container {:?} is not owned by {}", cli_args.name, crate::APP_NAME));
    };

    let args = {
        let Some(user_shell) = container_info.get_label(crate::CONTAINER_LABEL_USER_SHELL) else {
            return Err(anyhow!("Container {:?} does not have label {:?}", cli_args.name, crate::CONTAINER_LABEL_USER_SHELL));
        };

        // TODO share the env with exec command so its consistent
        vec![
            "exec".into(), "-it".into(),
            format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
            format!("--env=HOME=/home/{}", ctx.user),
            format!("--env=SHELL={}", user_shell),
            "--workdir".into(), ws_dir.to_string(),
            "--user".into(), ctx.user.clone(),
            cli_args.name.clone(),
            // NOTE: workaround to always source ~/.profile, even if shell is a script or non posix
            // like fish shell or nushell
            "sh".into(), "-l".into(), "-c".into(), format!("exec {}", user_shell),
        ]
    };

    let mut cmd = ctx.engine.command();
    cmd.args(args);

    if ctx.dry_run {
        cmd.print_escaped_cmd();

        Ok(())
    } else {
        let cmd = cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG);

        if cmd.success() {
            Ok(())
        } else {
            Err(anyhow!("Shell exited with error code {}", cmd))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use assert_cmd::prelude::*;
    use crate::tests_prelude::*;
    use rexpect::session::{PtyReplSession, spawn_command};

    #[test]
    fn cmd_shell_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        let _container = Container {
            engine: Engine::Podman,
            container: String::from_utf8_lossy(&cmd.get_output().stdout).trim().to_string(),
        };

        let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        c.args(["shell"]);
        c.current_dir(tempdir.path());

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
}
