use std::process::{Command, ExitCode};
use crate::util::{CommandOutputExt, Engine};

pub fn print_containers(engine: Engine) -> ExitCode {
    Command::new(engine.get_path())
        .args(&["container", "ls", "--filter", "label=box"])
        .status()
        .expect("Could not execute engine")
        .to_exitcode()
}
