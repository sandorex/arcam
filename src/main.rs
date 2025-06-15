mod cli;
mod command_ext;
mod commands;
mod config;
mod context;
mod engine;
mod util;
mod vars;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::CliCommands;

pub use command_ext::command_extensions;
pub use context::Context;
pub use util::*;
pub use vars::*;

#[cfg(test)]
pub use tests::prelude as tests_prelude;

pub mod prelude {
    // NOTE: anyhow context is renamed cause it clashes with Context
    pub use crate::context::Context;
    pub use anyhow::{anyhow, Context as AnyhowContext, Result};
}

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    simple_logger::init_with_level(args.log_level)?;

    let get_ctx = || {
        if !util::executable_in_path("podman") {
            return Err(anyhow!("Could not find podman in PATH"));
        }

        Context::new(args.dry_run, Box::new(engine::Podman))
    };

    match args.cmd {
        CliCommands::Start(x) => commands::start_container(get_ctx()?, x)?,
        CliCommands::Shell(x) => commands::open_shell(get_ctx()?, x)?,
        CliCommands::Exec(x) => commands::container_exec(get_ctx()?, x)?,
        CliCommands::Exists(x) => commands::container_exists(get_ctx()?, x)?,
        CliCommands::Config(x) => commands::config_command(get_ctx()?, x)?,
        CliCommands::List(x) => commands::print_containers(get_ctx()?, x)?,
        CliCommands::Logs(x) => commands::print_logs(get_ctx()?, x)?,
        CliCommands::Kill(x) => commands::kill_container(get_ctx()?, x)?,
        CliCommands::Completion(x) => {
            if x.complete.is_some() {
                commands::shell_completion_helper(get_ctx()?, x)?
            } else {
                commands::shell_completion_generation(x)?
            }
        }
        CliCommands::Wrap(x) => {
            // TODO move this to its own file
            let ctx = get_ctx()?;
            use anyhow::Context as AnyhowContext;
            use command_extensions::*;
            use std::process::Stdio;

            let container_exists = |container: &str| -> Result<bool> {
                log::trace!("Testing for existance of container {container:?}");

                let cmd = ctx
                    .engine
                    .command()
                    .args(["container", "exists", container])
                    .log_output()
                    .expect(crate::ENGINE_ERR_MSG);

                match cmd.get_code() {
                    0 => Ok(true),
                    1 => Ok(false),

                    // this really should not happen unless something breaks
                    x => Err(anyhow!(
                        "Unknown error during container initialization ({x})"
                    )),
                }
            };

            if !container_exists(&x.container)? {
                return Err(anyhow!("Container {:?} does not exist", x.container));
            }

            assert!(x.command.len() > 0);

            // start the process
            let mut command = std::process::Command::new(&x.command[0]);
            command.args(&x.command[1..]);
            command.stdin(Stdio::null());

            // if provided pipe stdout into the file
            if let Some(file) = x.stdout_file {
                let file_handle = std::fs::File::create(&file)
                    .with_context(|| anyhow!("error creating file at {:?}", file))?;

                command.stdout(file_handle);
            } else {
                command.stdout(Stdio::null());
            }

            // if provided pipe stderr into the file
            if let Some(file) = x.stderr_file {
                let file_handle = std::fs::File::create(&file)
                    .with_context(|| anyhow!("error creating file at {:?}", file))?;

                command.stderr(file_handle);
            } else {
                command.stderr(Stdio::null());
            }

            let mut command = command.log_spawn_anyhow()?;

            // wait for container to quit
            while container_exists(&x.container)? {
                std::thread::sleep(std::time::Duration::from_secs(x.interval.into()));
            }

            // signal the process to stop or kill it if it takes too long
            let _ = std::process::Command::new("kill")
                .arg("--verbose")
                .arg("--signal")
                .arg(format!("{}", &x.signal))
                .arg("--timeout")
                .arg(format!("{}", x.timeout))
                .arg("KILL")
                .arg(format!("{}", command.id()))
                .log_output_anyhow()?;

            let _ = command.wait();
        }
        CliCommands::Init => {
            if !util::is_in_container() {
                return Err(anyhow!(
                    "Running init outside a container is dangerous, qutting.."
                ));
            }

            commands::container_init()?
        }
    };

    Ok(())
}
