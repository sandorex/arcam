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
        cmd.log();
    } else {
        cmd.log_output_anyhow()?;
    }

    Ok(())
}
