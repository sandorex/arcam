use crate::util::{self, Engine};
use crate::{cli, ExitResult};
use crate::util::command_extensions::*;

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

fn gen_open_shell_cmd(engine: &Engine, shell: &Option<String>, ws_dir: String, container_name: &str) -> Vec<String> {
    let user = std::env::var("USER").expect("Unable to get USER from env var");
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

pub fn open_shell(engine: Engine, dry_run: bool, mut cli_args: cli::CmdShellArgs) -> ExitResult {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = util::find_containers_by_cwd(&engine) {
            if containers.is_empty() {
                eprintln!("Could not find a running container in current directory");
                return Err(1);
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    } else if !dry_run && !util::container_exists(&engine, &cli_args.name) {
        eprintln!("Container {:?} does not exist", &cli_args.name);
        return Err(1);
    }

    // check if container is owned
    let ws_dir = match util::get_container_ws(&engine, &cli_args.name) {
        Some(x) => x,
        // allow dry_run to work
        None if dry_run => "/ws/dry_run".to_string(),
        None => {
            eprintln!("Container {:?} is not owned by {}", &cli_args.name, crate::BIN_NAME);

            return Err(1);
        }
    };

    let args = gen_open_shell_cmd(&engine, &cli_args.shell, ws_dir, &cli_args.name);
    let mut cmd = Command::new(&engine.path);
    cmd.args(args);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        let cmd = cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG);

        // code 137 usually means SIGKILL and that messes up the terminal so reset it afterwards
        if cmd.code().is_some_and(|x| x == 137) {
            // i do not care if it failed
            let _ = Command::new("reset").status();
        }

        cmd.to_exitcode()
    }
}

