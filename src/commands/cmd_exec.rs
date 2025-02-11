use crate::cli;
use crate::util::command_extensions::*;
use crate::prelude::*;

pub fn container_exec(ctx: Context, mut cli_args: cli::CmdExecArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = ctx.get_cwd_container() {
            if containers.is_empty() {
                return Err(anyhow!("Could not find a running container in current directory"));
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    } else if !ctx.dry_run && !ctx.engine_container_exists(&cli_args.name) {
        return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
    }

    // check if container is owned
    let ws_dir = match ctx.get_container_label(&cli_args.name, crate::CONTAINER_LABEL_CONTAINER_DIR) {
        Some(x) => x,
        // allow dry_run to work
        None if ctx.dry_run => "/ws/dry_run".to_string(),
        None => return Err(anyhow!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME)),
    };

    let mut cmd = ctx.engine_command();
    cmd.args(["exec", "-it"]);
    cmd.args([
        format!("--workdir={}", ws_dir),
        format!("--user={}", ctx.user),
        format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
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
        cmd.print_escaped_cmd();
    } else {
        cmd.run_interactive()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use crate::engine::Engine;
    use super::super::test_utils::prelude::*;

    #[test]
    fn cmd_exec_podman() -> Result<()> {
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
