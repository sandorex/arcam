use crate::cli;
use std::process::Command;

/// Check if container is owned by box, will return false if container does not exist
fn is_box_container(engine: &str, name: &str) -> bool {
    let cmd = Command::new(engine)
        .args(&["container", "inspect", name, "--format", "{{if .Config.Labels.box}}{{.Config.Labels.box}}{{end}}"])
        .output()
        .expect("Could not execute engine");

    cmd.status.success() && !String::from_utf8_lossy(&cmd.stdout).is_empty()
}

pub fn kill_container(engine: &str, cli_args: &cli::CmdKillArgs) -> u8 {
    if ! is_box_container(engine, &cli_args.container) {
        eprintln!("Container '{}' is not owned by box or does not exist", &cli_args.container);
        return 1;
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
            _ => return 0,
        }
    }

    let cmd = Command::new(engine)
        .args(&["container", "stop", "--time", &cli_args.timeout.to_string(), &cli_args.container])
        .status()
        .expect("Could not execute engine");

    cmd.code().unwrap_or(1).try_into().unwrap()
}
