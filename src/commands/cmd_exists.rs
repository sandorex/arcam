use crate::cli;
use crate::util::{self, Engine};
use crate::ExitResult;

pub fn container_exists(engine: Engine, mut cli_args: cli::CmdExistsArgs) -> ExitResult {
    // try to find container in current directory
    if cli_args.name.is_empty() {
        if let Some(containers) = util::find_containers_by_cwd(&engine) {
            if containers.is_empty() {
                eprintln!("Could not find a running container in current directory");
                return Err(1);
            }

            cli_args.name = containers.first().unwrap().clone();
        }
    }

    match util::container_exists(&engine,&cli_args.name) {
        // give different exit code if the container exists but is not owned
        true => match util::get_container_ws(&engine, &cli_args.name) {
            Some(_) => Ok(()),
            None => Err(2),
        },
        false => Err(1),
    }
}
