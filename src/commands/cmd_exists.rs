use crate::cli;
use crate::prelude::*;

pub fn container_exists(ctx: Context, mut cli_args: cli::CmdExistsArgs) -> Result<()> {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        // TODO this whole thing could be a function its used at least 3 times!
        if let Some(containers) = ctx.get_cwd_container() {
            if containers.is_empty() {
                return Err(anyhow!("Could not find a running container in current directory"));
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    }

    use std::process::exit;

    // TODO i am not sure if there is any problems with using exit here
    match ctx.engine_container_exists(&cli_args.name) {
        // give different exit code if the container exists but is not owned
        true => match ctx.get_container_label(&cli_args.name, crate::CONTAINER_LABEL_CONTAINER_DIR) {
            Some(_) => Ok(()),
            None => exit(2),
        },
        false => exit(1),
    }
}
