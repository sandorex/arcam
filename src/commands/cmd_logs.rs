use crate::cli;
use crate::command_extensions::*;
use crate::prelude::*;

pub fn print_logs(ctx: Context, mut cli_args: cli::CmdLogsArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        let containers = ctx.get_cwd_containers()?;
        if containers.is_empty() {
            return Err(anyhow!(
                "Could not find a running container in current directory"
            ));
        }

        cli_args.name = containers.first().unwrap().clone();
    }

    println!("The logs may be empty if the container name is not valid");

    let mut cmd = Command::new("journalctl");
    cmd.args(["-t", &cli_args.name]);

    if cli_args.follow {
        // follow the output
        cmd.arg("--follow");
    }

    cmd.log_status(log::Level::Debug)?;

    Ok(())
}
