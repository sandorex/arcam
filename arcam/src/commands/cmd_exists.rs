use crate::cli;
use crate::prelude::*;
use std::process::exit;

pub fn container_exists(ctx: Context, cli_args: cli::CmdExistsArgs) -> Result<()> {
    if cli_args.name.is_empty() {
        // cwd containers are always owned
        match ctx.get_cwd_containers() {
            Ok(containers) if !containers.is_empty() => exit(0),
            _ => exit(1),
        }
    } else if ctx.engine.container_exists(&cli_args.name)? {
        exit(0);
    } else {
        exit(1);
    }
}
