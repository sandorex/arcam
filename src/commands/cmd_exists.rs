use crate::cli;
use crate::util::{get_container_status, Engine};
use std::process::ExitCode;

pub fn container_exists(engine: Engine, cli_args: &cli::CmdExistsArgs) -> ExitCode {
    // whatever state it is, it exists
    match get_container_status(&engine, &cli_args.container) {
        Some(_) => ExitCode::SUCCESS,
        None => ExitCode::FAILURE,
    }
}
