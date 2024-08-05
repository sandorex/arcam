use crate::util::{self, CommandOutputExt, Engine};
use crate::cli;
use std::process::{Command, ExitCode};

/// Extracts default shell for user from /etc/passwd inside a container
fn get_user_shell(engine: &Engine, container: &str, user: &str) -> String {
    let cmd_result = Command::new(&engine.path)
        .args(["exec", "--user", "root", "-it", container, "getent", "passwd", user])
        .output()
        .expect("Could not execute engine");

    const ERR: &str = "Failed to extract default shell from /etc/passwd";
    let stdout = String::from_utf8_lossy(&cmd_result.stdout);
    if ! cmd_result.status.success() || stdout.is_empty() {
        panic!("{}", ERR);
    }

    // i do not want to rely on external tools like awk so im extracting manually
    stdout.trim()
        .split(':')
        .last()
        .map(|x| x.to_string())
        .expect(ERR)
}

pub fn open_shell(engine: Engine, dry_run: bool, cli_args: &cli::CmdShellArgs) -> ExitCode {
    // check if container is owned
    if ! util::is_box_container(&engine, &cli_args.name) {
        eprintln!("Container {} is not owned by box or does not exist", &cli_args.name);
        return ExitCode::FAILURE;
    }

    let user = util::get_user();
    let user_shell = match &cli_args.shell {
        Some(x) => x,
        None => &get_user_shell(&engine, &cli_args.name, user.as_str()),
    };

    let args: Vec<String> = vec![
        "exec".into(), "-it".into(),
        "--user".into(), user,
        // propagete TERM but default to xterm
        "--env".into(), format!("TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        "--workdir".into(), "/ws".into(),
        cli_args.name.clone(),
        user_shell.to_string(), "-l".into(),
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

