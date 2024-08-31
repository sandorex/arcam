use crate::cli;
use crate::util::{get_container_status, Engine};
use crate::ExitResult;

pub fn container_exists(engine: Engine, cli_args: &cli::CmdExistsArgs) -> ExitResult {
    // whatever state it is, it exists
    match get_container_status(&engine, &cli_args.container) {
        Some(_) => Ok(()),
        None => Err(1),
    }
}
