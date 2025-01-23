use crate::prelude::*;
use crate::cli;
use crate::util::command_extensions::*;

/// Extracts default shell for user from /etc/passwd inside a container
fn get_user_shell(ctx: &Context, container: &str) -> Result<String> {
    // try to get default shell from symlink
    if let Ok(shell) = ctx.engine_exec_root(container, vec!["readlink", "/shell"]) {
        return Ok(shell.trim().to_string());
    }

    // fallback to user shell
    let output = ctx.engine_exec_root(container, vec!["getent", "passwd", &ctx.user])?;

    Ok(
        output
            .trim()
            .rsplit_once(':')
            .ok_or(anyhow!("Error parsing user from passwd (output was {:?})", output))?
            .1
            .to_string()
    )
}

pub fn open_shell(ctx: Context, mut cli_args: cli::CmdShellArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = ctx.get_cwd_container() {
            if containers.is_empty() {
                return Err(anyhow!("Could not find a running container in current directory"));
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    } else if !ctx.dry_run && !ctx.get_container_status(&cli_args.name).is_some() {
        return Err(anyhow!("Container {:?} does not exist", &cli_args.name));
    }

    // check if container is owned
    let ws_dir = match ctx.get_container_label(&cli_args.name, crate::CONTAINER_LABEL_CONTAINER_DIR) {
        Some(x) => x,
        // allow dry_run to work
        None if ctx.dry_run => "/ws/dry_run".to_string(),
        None => return Err(anyhow!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME)),
    };

    let args = {
        let user_shell = match &cli_args.shell {
            Some(x) => x,
            None => &get_user_shell(&ctx, &cli_args.name)?,
        };

        // TODO share the env with exec command so its consistent
        vec![
            "exec".into(), "-it".into(),
            format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
            format!("--env=HOME=/home/{}", ctx.user),
            format!("--env=SHELL={}", user_shell),
            "--workdir".into(), ws_dir,
            "--user".into(), ctx.user.clone(),
            cli_args.name.clone(),
            // hacky way to always source ~/.profile even with fish shell
            "sh".into(), "-l".into(), "-c".into(), format!("exec {}", user_shell),
        ]
    };

    let mut cmd = ctx.engine_command();
    cmd.args(args);

    if ctx.dry_run {
        cmd.print_escaped_cmd();

        Ok(())
    } else {
        let cmd = cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG);

        let code = cmd.get_code();

        // TODO redo this so it does not clear screen each time its exited
        // code 137 usually means SIGKILL and that messes up the terminal so reset it afterwards
        if code == 137 {
            // i do not care if it failed
            let _ = Command::new("reset").status();
            return Ok(());
        }

        if cmd.success() {
            Ok(())
        } else {
            Err(anyhow!("Shell exited with error code {}", cmd))
        }
    }
}
