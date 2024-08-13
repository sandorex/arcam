use crate::util::{self, get_container_ws, CommandOutputExt, Engine};
use crate::cli;
use std::process::{Command, ExitCode};

fn gen_container_exec_cmd(shell: bool, ws_dir: String, container_name: &str, command: &[String]) -> Vec<String> {
    let user = util::get_user();

    let mut args: Vec<String> = vec![
        "exec".into(),
        "-it".into(),
        "--workdir".into(), ws_dir,
        "--user".into(), user,
        format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        container_name.to_string(),
    ];

    if shell {
        // run the command as one big concatenated script
        args.extend(vec![
            "--env=SHELL=/usr/bin/bash".into(),
            "bash".into(), "-c".into(), command.join(" "),
        ]);
    } else {
        // just execute verbatim
        args.extend(command.iter().cloned());
    }

    args
}

pub fn container_exec(engine: Engine, dry_run: bool, cli_args: &cli::CmdExecArgs) -> ExitCode {
    // check if container is owned (ignore it if dry_run)
    if !dry_run && !util::is_box_container(&engine, &cli_args.name) {
        eprintln!("Container {} is not owned by box or does not exist", &cli_args.name);
        return ExitCode::FAILURE;
    }

    let ws_dir = match get_container_ws(&engine, &cli_args.name) {
        Some(x) => x,
        None if dry_run => "/ws/dry_run".to_string(), // NOTE just so it does not fail in dry_run
        None => {
            eprintln!("Could not get container workspace from container {:#?}", &cli_args.name);

            return ExitCode::FAILURE;
        },
    };

    let args = gen_container_exec_cmd(cli_args.shell, ws_dir, &cli_args.name, &cli_args.command);

    if dry_run {
        util::print_cmd_dry_run(&engine, args);

        ExitCode::SUCCESS
    } else {
        Command::new(&engine.path)
            .args(args)
            .status()
            .expect("Could not execute engine")
            .to_exitcode()
    }
}

