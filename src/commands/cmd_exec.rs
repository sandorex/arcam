use crate::cli;
use crate::util::command_extensions::*;
use crate::prelude::*;
use std::path::Path;

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
        // this updates the flag files so auto-shutdown works
        std::thread::spawn(move || {
            // TODO encode these settings into a label using base64
            // return early if there is no autoshutdown label
            if ctx.get_container_label(&cli_args.name, "autoshutdown").is_none() {
                return;
            }

            loop {
                let _ = ctx.engine_exec_root(&cli_args.name, vec![
                    "touch",
                    &Path::new(crate::HEALTH_DIR).join(std::process::id().to_string()).to_string_lossy()
                ]);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        cmd.run_interactive()?;
    }

    Ok(())
}
