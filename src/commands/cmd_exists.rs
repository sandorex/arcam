use crate::cli;
use crate::util;
use std::process::ExitCode;

pub fn container_exists(engine: &str, cli_args: &cli::CmdExistsArgs) -> ExitCode {
    // whatever state it is, it exists
    match util::get_container_status(engine, &cli_args.container) {
        Some(_) => ExitCode::SUCCESS,
        None => ExitCode::FAILURE,
    }
}
