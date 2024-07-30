use crate::get_user;
use crate::cli;

use std::process::Command;

pub fn container_exec(engine: &str, cli_args: &cli::CmdExecArgs) -> u8 {
    let args: Vec<String>;
    if cli_args.shell {
        // run the command as one big concatenated script
        args = vec![
            "bash".into(), "-c".into(), cli_args.command.join(" "),
        ]
    } else {
        // just execute verbatim
        args = cli_args.command.clone();
    }

    let cmd = Command::new(engine)
        .args(&["exec", "-it", "--user", &get_user(), &cli_args.name])
        .args(args)
        .status()
        .expect("Could not execute engine");

    if cmd.success() {
        0
    } else {
        cmd.code().unwrap_or(1).try_into().unwrap()
    }
}

