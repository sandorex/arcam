mod util;
mod cli;
mod commands;
mod config;
mod context;
mod vars;

use clap::Parser;
use cli::CliCommands;
use util::Engine;
use anyhow::anyhow;

pub use vars::*;
pub use context::Context;

pub mod prelude {
    // NOTE: anyhow context is renamed cause it clashes with Context
    pub use anyhow::{anyhow, Context as AnyhowContext, Result};

    pub use crate::context::Context;
}

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    // init and healthcheck do not need context
    if let CliCommands::Init = args.cmd {
        if !util::is_in_container() {
            return Err(anyhow!("Running init outside a container is dangerous, qutting.."));
        }

        return commands::container_init();
    }

    let get_ctx = || {
        // find and detect engine
        let engine: Engine = if let Some(chosen) = &args.engine {
            // test if engine exists in PATH or as a literal path
            if !(util::executable_in_path(chosen) || std::path::Path::new(chosen).exists()) {
                return Err(anyhow!("Engine '{}' not found in PATH or filesystem", chosen));
            }

            Engine::detect(chosen).expect("Failed to detect engine kind")
        } else if let Some(found) = Engine::find_available_engine() {
            found
        } else {
            return Err(anyhow!("No compatible container engine found in PATH"));
        };

        Context::new(args.dry_run, engine)
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
        CliCommands::Completion(x) => if x.complete.is_some() {
            commands::shell_completion_helper(get_ctx()?, x)?
        } else {
            // i do not want to create a context when i dont need it here
            commands::shell_completion_generation(x)?
        },
        CliCommands::Init => unreachable!(),
    };

    Ok(())
}
