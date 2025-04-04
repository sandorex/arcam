use crate::cli;
use crate::command_extensions::*;
use crate::prelude::*;

pub fn kill_container(ctx: Context, mut cli_args: cli::CmdKillArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        let containers = ctx.get_cwd_containers()?;
        if containers.is_empty() {
            return Err(anyhow!(
                "Could not find a running container in current directory"
            ));
        }

        cli_args.name = containers.first().unwrap().clone();
    } else if !ctx.dry_run {
        if !ctx.engine.container_exists(&cli_args.name)? {
            return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
        }

        let container_info = ctx.engine.inspect_containers(vec![&cli_args.name])?;
        let container_info = container_info.first().unwrap();

        // check if container is owned
        if !container_info
            .labels
            .contains_key(crate::CONTAINER_LABEL_APP)
        {
            return Err(anyhow!(
                "Container {:?} is not owned by {}",
                &cli_args.name,
                crate::APP_NAME
            ));
        }
    }

    // prompt user
    if !cli_args.yes
        && !crate::prompt(
            format!(
                "Are you sure you want to kill container {:?} ?",
                &cli_args.name
            )
            .as_str(),
        )
    {
        return Err(anyhow!("Cancelled by user."));
    }

    let timeout = cli_args.timeout.to_string();
    let mut cmd = ctx.engine.command();
    cmd.args(["container", "stop", "--time", &timeout, &cli_args.name]);

    if ctx.dry_run {
        cmd.log(log::Level::Error);
    } else {
        cmd.log_output_anyhow(log::Level::Debug)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::engine::Podman;
    use crate::tests_prelude::*;
    use assert_cmd::prelude::*;
    use rexpect::session::spawn_command;
    use std::process::Command;

    #[test]
    #[ignore]
    fn cmd_kill_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        // create the container
        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
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
    #[ignore]
    fn cmd_kill_interactive_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
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
            let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
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
            let mut c = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
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
}
