mod util;
mod cli;
mod commands;
mod config;
mod context;
mod vars;

use clap::Parser;
use cli::CliCommands;
use cli::cli_config::ConfigCommands;
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

    // init does not need context
    if let CliCommands::Init(x) = args.cmd {
        if !util::is_in_container() {
            return Err(anyhow!("Running init outside a container is dangerous, qutting.."));
        }

        return commands::container_init(x);
    }

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

    let ctx = Context::new(args.dry_run, engine)?;

    match args.cmd {
        CliCommands::Start(x) => commands::start_container(ctx, x)?,
        CliCommands::Shell(x) => commands::open_shell(ctx, x)?,
        CliCommands::Exec(x) => commands::container_exec(ctx, x)?,
        CliCommands::Exists(x) => commands::container_exists(ctx, x)?,
        CliCommands::Config(subcmd) => match subcmd {
            ConfigCommands::Extract(x) => commands::extract_config(ctx, x)?,
            ConfigCommands::Inspect(x) => commands::inspect_config(x)?,
            ConfigCommands::Options => commands::show_config_options(ctx),
        },
        CliCommands::List(x) => commands::print_containers(ctx, x)?,
        CliCommands::Logs(x) => commands::print_logs(ctx, x)?,
        CliCommands::Kill(x) => commands::kill_container(ctx, x)?,
        CliCommands::Init(_) => unreachable!(),
    };

    Ok(())
}
