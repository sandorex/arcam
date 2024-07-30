use crate::{get_user, get_user_shell};
use crate::cli;

// use crate::CustomResult;

use std::process::Command;

pub fn open_shell(engine: &str, cli_args: &cli::CmdShellArgs) -> u8 {
    let user = get_user();

    let _ = Command::new(engine)
        .args(&[
            "exec", "-it",
            "--user", user.as_str(),
            "--env", format!("TERM={}", std::env::var("TERM").unwrap_or("xterm".into())).as_str(),
            "--workdir", "/ws",
            &cli_args.name,
            get_user_shell(engine, &cli_args.name, user.as_str()).as_str(), "-l",
        ])
        .status()
        .expect("Could not execute engine");

    0
}

