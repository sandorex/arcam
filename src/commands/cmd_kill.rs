use crate::cli;
use crate::prelude::*;
use crate::command_extensions::*;

pub fn kill_container(ctx: Context, mut cli_args: cli::CmdKillArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = ctx.get_cwd_container() {
            if containers.is_empty() {
                return Err(anyhow!("Could not find a running container in current directory"));
            }

            cli_args.name = containers.first().unwrap().clone();
        }

        // i do not need to test container if it was found by cwd
    } else if !ctx.dry_run {
        if !ctx.engine_container_exists(&cli_args.name) {
            return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
        }

        // check if container is owned
        if ctx.get_container_label(&cli_args.name, crate::CONTAINER_LABEL_CONTAINER_DIR).is_none() {
            return Err(anyhow!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME));
        }
    }

    if !cli_args.yes && !crate::prompt(format!("Are you sure you want to kill container {:?} ?", &cli_args.name).as_str()) {
        return Err(anyhow!("Cancelled by user."));
    }

    let timeout = cli_args.timeout.to_string();
    let mut cmd = ctx.engine_command();
    cmd.args(["container", "stop", "--time", &timeout, &cli_args.name]);

    if ctx.dry_run {
        cmd.print_escaped_cmd();
    } else {
        cmd.run_get_output()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use assert_cmd::prelude::*;
    use crate::engine::Engine;
    use crate::tests_prelude::*;
    use rexpect::session::spawn_command;

    #[test]
    fn cmd_kill_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        // create the container
        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        let container = Container {
            engine: Engine::Podman,
            container: String::from_utf8_lossy(&cmd.get_output().stdout).trim().to_string(),
        };

        // it should exist now
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["exists"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        // test with --yes
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["kill", "-y", &container])
            .current_dir(tempdir.path())
            .assert()
            .success();

        Ok(())
    }

    #[test]
    fn cmd_kill_interactive_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        let container = Container {
            engine: Engine::Podman,
            container: String::from_utf8_lossy(&cmd.get_output().stdout).trim().to_string(),
        };

        // try to kill to get the prompt
        let mut pty = {
            let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
            c.args(["kill", &container]);

            spawn_command(c, Some(5_000))
        }?;

        let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
        assert!(matched.contains(&*container), "Wrong container name in prompt?");

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
            let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
            c.args(["kill", &container]);

            spawn_command(c, Some(5_000))
        }?;

        let (_, matched) = pty.exp_regex(r#"\"(.+)\".*\[y/N\]"#)?;
        assert!(matched.contains(&*container), "Wrong container name in prompt?");

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
}
