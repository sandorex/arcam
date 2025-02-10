use crate::prelude::*;
use crate::cli;
use crate::util::command_extensions::*;

pub fn open_shell(ctx: Context, mut cli_args: cli::CmdShellArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = ctx.get_cwd_container() {
            if containers.is_empty() {
                return Err(anyhow!("Could not find a running container in current directory"));
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    } else if !ctx.dry_run && ctx.engine_container_exists(&cli_args.name) {
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
        let user_shell = ctx.get_container_label(&cli_args.name, crate::CONTAINER_LABEL_USER_SHELL).unwrap();

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

        if cmd.success() {
            Ok(())
        } else {
            Err(anyhow!("Shell exited with error code {}", cmd))
        }
    }
}
