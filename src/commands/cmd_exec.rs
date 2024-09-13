use crate::util::{self, Engine};
use crate::cli;
use crate::util::command_extensions::*;
use crate::ExitResult;

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
            eprintln!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME);

            return Err(1);
        }
    };

    let user = std::env::var("USER").expect("Unable to get USER from env var");

    let mut cmd = Command::new(&engine.path);
    cmd.args(["exec", "-it"]);
    cmd.args([
        format!("--workdir={}", ws_dir),
        format!("--user={}", user),
        format!("--env=TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
    ]);

    if let Some(shell) = &cli_args.shell {
        cmd.arg(format!("--env=SHELL={}", shell));
        cmd.args([
            &cli_args.name,
            shell, "-c"
        ]);

        // run the command as one big concatenated string
        cmd.arg(cli_args.command.join(" "));
    } else {
        cmd.arg(&cli_args.name);

        // just execute verbatim
        cmd.args(&cli_args.command);
    }

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}

