/// Contains all code that should run inside the container as the init

use std::process::ExitCode;
use crate::FULL_VERSION;

pub const INIT_SCRIPT: &'static str = include_str!("box-init.sh");
pub const DATA_VOLUME_NAME: &'static str = "box-data"; // TODO move the vol name to main

pub fn container_init() -> ExitCode {
    println!("box {}", FULL_VERSION);

    use std::io::Write;
    use std::process::{Command, Stdio};

    // TODO listen to TERM signals and shutdown properly, currently container has to be killed with
    // SIGKILL
    let mut exec_child = Command::new("bash")
        .args(&["-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("Could not execute engine");

    {
        let stdin = exec_child.stdin.as_mut().unwrap();
        stdin.write_all(INIT_SCRIPT.as_bytes()).unwrap();
    }

    let result = exec_child.wait_with_output().unwrap();

    // just return the code
    ExitCode::from(TryInto::<u8>::try_into(result.status.code().unwrap_or(1)).unwrap())
}
