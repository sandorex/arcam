use crate::cli;
use crate::util::{self, CommandOutputExt, Engine};
use std::process::{Command, ExitCode};

pub fn kill_container(engine: Engine, dry_run: bool, cli_args: &cli::CmdKillArgs) -> ExitCode {
    if ! util::is_box_container(&engine, &cli_args.container) {
        eprintln!("Container {} is not owned by box or does not exist", &cli_args.container);
        return ExitCode::FAILURE;
    }

    // simple shitty prompt
    // if not yes then yes, but if yes then no yes
    if ! cli_args.yes {
        use std::io::Write;
        let mut s = String::new();

        print!("Are you sure you want to kill container {:?} ? [y/N] ", &cli_args.container);

        let _ = std::io::stdout().flush();

        std::io::stdin().read_line(&mut s).expect("Could not read stdin");
        s = s.trim().to_string();

        match s.to_lowercase().as_str() {
            "y"|"yes" => {},
            _ => return ExitCode::FAILURE,
        }
    }

    let args: Vec<String> = vec![
        "container".into(), "stop".into(), "--time".into(), cli_args.timeout.to_string(), cli_args.container.clone(),
    ];

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
