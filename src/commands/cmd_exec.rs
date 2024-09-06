use crate::util::{self, Engine};
use crate::cli;
use crate::util::command_extensions::*;
use crate::ExitResult;

fn gen_container_exec_cmd(shell: bool, ws_dir: String, container_name: &str, command: &[String]) -> Vec<String> {
    let user = std::env::var("USER").expect("Unable to get USER from env var");

    let mut args: Vec<String> = vec![
        "exec".into(),
        "-it".into(),
        "--workdir".into(), ws_dir,
        "--user".into(), user,
        format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        container_name.to_string(),
    ];

    if shell {
        // run the command as one big concatenated script
        args.extend(vec![
            "--env=SHELL=/usr/bin/bash".into(),
            "bash".into(), "-c".into(), command.join(" "),
        ]);
    } else {
        // just execute verbatim
        args.extend(command.iter().cloned());
    }

    args
}

pub fn container_exec(engine: Engine, dry_run: bool, mut cli_args: cli::CmdExecArgs) -> ExitResult {
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

    let args = gen_container_exec_cmd(cli_args.shell, ws_dir, &cli_args.name, &cli_args.command);
    let mut cmd = Command::new(&engine.path);
    cmd.args(args);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}

