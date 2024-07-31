use crate::util::{self, CommandOutputExt};
use crate::cli;
use std::process::{Command, ExitCode};

pub fn container_exec(engine: &str, dry_run: bool, cli_args: &cli::CmdExecArgs) -> ExitCode {
    // check if container is owned
    if ! util::is_box_container(engine, &cli_args.name) {
        eprintln!("Container {} is not owned by box or does not exist", &cli_args.name);
        return ExitCode::FAILURE;
    }

    let mut args = vec![
        "exec".into(), "-it".into(), "--user".into(), util::get_user(), cli_args.name.clone()
    ];

    if cli_args.shell {
        // run the command as one big concatenated script
        args.extend(vec![
            "bash".into(), "-c".into(), cli_args.command.join(" "),
        ]);
    } else {
        // just execute verbatim
        args.extend(cli_args.command.clone());
    }

    if dry_run {
        util::print_cmd_dry_run(engine, args);

        ExitCode::SUCCESS
    } else {
        Command::new(engine)
            .args(args)
            .status()
            .expect("Could not execute engine")
            .to_exitcode()
    }
}

