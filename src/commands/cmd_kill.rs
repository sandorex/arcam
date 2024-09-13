use crate::{ExitResult, cli};
use crate::util::command_extensions::*;
use crate::util::{self, Engine};

pub fn kill_container(engine: Engine, dry_run: bool, mut cli_args: cli::CmdKillArgs) -> ExitResult {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = util::find_containers_by_cwd(&engine) {
            if containers.is_empty() {
                eprintln!("Could not find a running container in current directory");
                return Err(1);
            }

            cli_args.name = containers.first().unwrap().clone();
        }

        // i do not need to test container if it was found by cwd
    } else if !dry_run {
        if !util::container_exists(&engine, &cli_args.name) {
            eprintln!("Container {:?} does not exist", &cli_args.name);
            return Err(1);
        }

        // check if container is owned
        if util::get_container_ws(&engine, &cli_args.name).is_none() {
            eprintln!("Container {:?} is not owned by {}", &cli_args.name, crate::APP_NAME);

            return Err(1);
        }
    }

    if !cli_args.yes && !util::prompt(format!("Are you sure you want to kill container {:?} ?", &cli_args.name).as_str()) {
        return Err(1);
    }

    let timeout = cli_args.timeout.to_string();

    let mut cmd = Command::new(&engine.path);
    cmd.args(["container", "stop", "--time", &timeout, &cli_args.name]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}
