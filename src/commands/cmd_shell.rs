use crate::{get_user, get_user_shell};
use crate::cli;

pub fn open_shell(engine: &str, dry_run: bool, cli_args: &cli::CmdShellArgs) -> u8 {
    let user = get_user();
    let user_shell = get_user_shell(engine, &cli_args.name, user.as_str());

    let cmd = crate::engine_cmd_status(engine, dry_run, vec![
        "exec".into(), "-it".into(),
        "--user".into(), user,
        // propagete TERM but default to xterm
        "--env".into(), format!("TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        "--workdir".into(), "/ws".into(),
        user_shell, "-l".into(),
    ]);

    // propagate the exit code even though it does not matter much
    match cmd {
        Ok(_) => 0,
        Err(x) => x,
    }
}

