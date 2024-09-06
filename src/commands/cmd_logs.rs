use crate::util::{self, command_extensions::*, Engine};
use crate::{cli, ExitResult};

pub fn print_logs(engine: Engine, mut cli_args: cli::CmdLogsArgs) -> ExitResult {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = util::find_containers_by_cwd(&engine) {
            if containers.is_empty() {
                eprintln!("Could not find a running container in current directory");
                return Err(1);
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    }

    println!("The logs may be empty if the container name is not valid");

    let mut cmd = Command::new("journalctl");
    cmd.args(["-t", &cli_args.name]);

    if cli_args.follow {
        // follow the output
        cmd.arg("--follow");
    }

    cmd.status()
        .expect("Failed to execute journalctl")
        .to_exitcode()
}
