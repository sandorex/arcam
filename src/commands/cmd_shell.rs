use crate::util::{self, get_container_ws, CommandOutputExt, Engine};
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

fn gen_open_shell_cmd(engine: &Engine, shell: &Option<String>, ws_dir: String, container_name: &str) -> Vec<String>{
    let user = util::get_user();
    let user_shell = match shell {
        Some(x) => x,
        None => &get_user_shell(engine, container_name, user.as_str()),
    };

    vec![
        "exec".into(), "-it".into(),
        format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        format!("--env=HOME=/home/{}", user),
        format!("--env=SHELL={}", user_shell),
        "--workdir".into(), ws_dir,
        "--user".into(), user,
        container_name.to_string(),
        user_shell.to_string(), "-l".into(),
    ]
}

pub fn open_shell(engine: Engine, dry_run: bool, cli_args: &cli::CmdShellArgs) -> ExitCode {
    // check if container is owned
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

    let args = gen_open_shell_cmd(&engine, &cli_args.shell, ws_dir, &cli_args.name);

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

