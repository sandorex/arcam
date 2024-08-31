use crate::util::command_extensions::*;
use crate::{cli, ExitResult};

pub fn print_logs(cli_args: &cli::CmdLogsArgs) -> ExitResult {
    println!("The logs may be empty if the container name is not valid");

    let mut cmd = Command::new("journalctl");
    cmd.args(["-t", &cli_args.container]);

    if cli_args.follow {
        // follow the output
        cmd.arg("--follow");
    }

    cmd.status()
        .expect("Failed to execute journalctl")
        .to_exitcode()
}
