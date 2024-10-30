use crate::util::{self, Engine};
use crate::{cli, ExitResult};
use crate::util::command_extensions::*;

/// Extracts default shell for user from /etc/passwd inside a container
fn get_user_shell(engine: &Engine, container: &str, user: &str) -> String {
    let cmd_result = Command::new(&engine.path)
        .args(["exec", "--user", "root", "-i", container, "getent", "passwd", user])
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

// pub fn touch_process(engine: &Engine, container: &str, pid: &str) -> ExitResult {
//     match std::fs::OpenOptions::new().create(true).write(true).open() {
//         Ok(_) => Ok(()),
//         Err(e) => Err(e),
// }

pub fn touch_file(name: &str) {
    use std::fs::File;
    use std::time::SystemTime;

    let file = File::create(name).unwrap();
    file.set_modified(SystemTime::now()).unwrap();
}

// TODO check if this could be merged into exec command as its mostly duplicate code
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
            eprintln!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME);

            return Err(1);
        }
    };

    let unique_id = std::process::id().to_string();

    let user = std::env::var("USER").expect("Unable to get USER from env var");
    let args = {
        let user_shell = match &cli_args.shell {
            Some(x) => x,
            None => &get_user_shell(&engine, &cli_args.name, user.as_str()),
        };

        vec![
            "exec".into(), "-it".into(),
            format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
            format!("--env=HOME=/home/{}", user),
            format!("--env=SHELL={}", user_shell),
            "--workdir".into(), ws_dir,
            "--user".into(), user,
            cli_args.name.clone(),
            // TODO this could be automated using this binary
            // execute shell and save the pid into the file
            "/bin/sh".into(), "-c".into(),

            // then replace process with the shell
            format!("[ -d /run/arcam/ ] && echo $$ > /run/arcam/{}; exec {} {}", unique_id, user_shell, "-l"),
        ]
    };

    let mut cmd = Command::new(&engine.path);
    cmd.args(args);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        // touch the file
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5000));
            touch_file(format!("/run/user/1000/arcam/{}", unique_id).as_str());
        });

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
