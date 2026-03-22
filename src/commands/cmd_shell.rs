use crate::cli;
use crate::command_extensions::*;
use crate::prelude::*;

pub fn open_shell(ctx: Context, mut cli_args: cli::CmdShellArgs) -> Result<()> {
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

    let Some(ws_dir) = container_info
        .labels
        .get(crate::CONTAINER_LABEL_CONTAINER_DIR)
    else {
        return Err(anyhow!(
            "Container {:?} is not owned by {}",
            cli_args.name,
            crate::APP_NAME
        ));
    };

    let mut cmd = ctx.engine.command();

    let Some(user_shell) = container_info.labels.get(crate::CONTAINER_LABEL_USER_SHELL) else {
        return Err(anyhow!(
            "Container {:?} does not have label {:?}",
            cli_args.name,
            crate::CONTAINER_LABEL_USER_SHELL
        ));
    };

    // TODO share the env with exec command so its consistent
    cmd.args([
        "exec".into(),
        "-it".into(),
        format!(
            "--env=TERM={}",
            std::env::var("TERM").unwrap_or("xterm".into())
        ),
        format!("--env=HOME=/home/{}", ctx.user),
        format!("--env=SHELL={}", user_shell),
        "--workdir".into(),
        ws_dir.to_string(),
        "--user".into(),
        ctx.user.clone(),
        cli_args.name.clone(),
        // NOTE: workaround to always source ~/.profile, even if shell is a script or non posix
        // like fish shell or nushell
        "sh".into(),
        "-l".into(),
        "-c".into(),
        format!("exec {}", user_shell),
    ]);

    if ctx.dry_run {
        cmd.log();
    } else {
        cmd.log_status_anyhow()?;
    }

    Ok(())
}
