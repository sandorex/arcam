use std::process::{Command, ExitCode};

use crate::{cli, util::CommandOutputExt};

pub fn print_logs(cli_args: &cli::CmdLogsArgs) -> ExitCode {
    println!("The logs may be empty if the container name is not valid");

    let mut args = vec!["-t", &cli_args.container];

    if cli_args.follow {
        // follow the output
        args.push("--follow");
    }

    Command::new("journalctl")
        .args(args)
        .status()
        .expect("Failed to execute journalctl")
        .to_exitcode()
}
