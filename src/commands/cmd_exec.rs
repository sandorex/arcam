use crate::cli;
use crate::command_extensions::*;
use crate::prelude::*;

pub fn container_exec(ctx: Context, mut cli_args: cli::CmdExecArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        let containers = ctx.get_cwd_containers()?;
        if containers.is_empty() {
            return Err(anyhow!(
                "Could not find a running container in current directory"
            ));
        }

        cli_args.name = containers.first().unwrap().clone();
    } else if !ctx.dry_run && !ctx.engine.container_exists(&cli_args.name)? {
        return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
    }

    let container_info = ctx.engine.inspect_containers(vec![&cli_args.name])?;
    let container_info = container_info.first().unwrap();

    // check if container is owned
    let Some(ws_dir) = container_info
        .labels
        .get(crate::CONTAINER_LABEL_CONTAINER_DIR)
    else {
        return Err(anyhow!(anyhow!(
            "Container {:?} is not owned by {}",
            &cli_args.name,
            crate::APP_NAME
        )));
    };

    let mut cmd = ctx.engine.command();
    cmd.args(["exec", "-it"]);
    cmd.args([
        format!("--workdir={}", ws_dir),
        format!("--user={}", ctx.user),
        format!(
            "--env=TERM={}",
            std::env::var("TERM").unwrap_or("xterm".into())
        ),
    ]);

    if let Some(shell) = &cli_args.shell {
        cmd.arg(format!("--env=SHELL={}", shell));
        cmd.args([&cli_args.name, shell]);

        // add -l and hope for the best
        if cli_args.login {
            // use sh -l -c 'SHELL -c ..' here
            cmd.arg("-l");
        }

        cmd.arg("-c");

        // run the command as one big concatenated string
        cmd.arg(cli_args.command.join(" "));
    } else {
        cmd.arg(&cli_args.name);

        // just execute verbatim
        cmd.args(&cli_args.command);
    }

    if ctx.dry_run {
        cmd.log(log::Level::Error);
    } else {
        cmd.log_status_anyhow(log::Level::Debug)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::engine::Podman;
    use crate::tests_prelude::*;
    use assert_cmd::Command;

    #[test]
    #[ignore]
    fn cmd_exec_podman() -> Result<()> {
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

        // create file in cwd
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["exec", &container, "--", "touch", "file.txt"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        assert!(tempdir.path().join("file.txt").exists(), "File not created");

        Ok(())
    }

    // TODO test environment passing and --shell
}
