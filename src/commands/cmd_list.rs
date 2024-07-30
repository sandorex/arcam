use std::process::Command;

pub fn print_containers(engine: &str) -> u8 {
    let cmd = Command::new(engine)
        .args(&["container", "ls", "--filter", "label=box"])
        .status()
        .expect("Could not execute engine");

    cmd.code().unwrap_or(1).try_into().unwrap()
}
