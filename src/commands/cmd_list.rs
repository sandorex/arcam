use std::process::{Command, ExitCode};
use crate::util::CommandOutputExt;

pub fn print_containers(engine: &str) -> ExitCode {
    Command::new(engine)
        .args(&["container", "ls", "--filter", "label=box"])
        .status()
        .expect("Could not execute engine")
        .to_exitcode()
}
