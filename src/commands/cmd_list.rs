use std::process::{Command, ExitCode};
use crate::util::{CommandOutputExt, Engine, print_cmd_dry_run};

pub fn print_containers(engine: Engine, dry_run: bool) -> ExitCode {
    let args: Vec<String> = vec![
        "container".into(), "ls".into(), "--filter".into(), "label=box".into()
    ];

    if dry_run {
        print_cmd_dry_run(&engine, args);

        ExitCode::SUCCESS
    } else {
        Command::new(&engine.path)
            .args(args)
            .status()
            .expect("Could not execute engine")
            .to_exitcode()
    }
}
