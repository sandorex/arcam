use crate::cli;
use crate::prelude::*;
use crate::util::command_extensions::*;
use crate::util;

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

    if !cli_args.yes && !util::prompt(format!("Are you sure you want to kill container {:?} ?", &cli_args.name).as_str()) {
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
