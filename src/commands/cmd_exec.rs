use crate::get_user;
use crate::cli;

pub fn container_exec(engine: &str, dry_run: bool, cli_args: &cli::CmdExecArgs) -> u8 {
    let mut args = vec![
        "exec".into(), "-it".into(), "--user".into(), get_user(), cli_args.name.clone()
    ];

    if cli_args.shell {
        // run the command as one big concatenated script
        args.extend(vec![
            "bash".into(), "-c".into(), format!("'{}'", cli_args.command.join(" ")),
        ]);
    } else {
        // just execute verbatim
        args.extend(cli_args.command.clone());
    }

    let cmd = crate::engine_cmd_status(engine, false, args);

    match cmd {
        Ok(_) => 0,
        Err(x) => x,
    }
}

