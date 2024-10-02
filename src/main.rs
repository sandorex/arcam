mod util;
mod cli;
mod commands;
mod config;

use clap::Parser;
use cli::CliCommands;
use cli::cli_config::ConfigCommands;
use std::process::ExitCode;
use util::Engine;

pub const ENGINE_ERR_MSG: &str = "Failed to execute engine";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FULL_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_DESCRIBE"));

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_NAME_UPPERCASE: &str = env!("CARGO_PKG_NAME_UPPERCASE");

/// Prefix env var name with proper prefix
#[macro_export]
macro_rules! ENV_VAR_PREFIX {
    ($($args:literal),*) => {
        concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_", $($args),*)
    };
}

pub use util::ExitResult;

fn get_engine(args: &cli::Cli) -> Result<Engine, ExitCode> {
    // find and detect engine
    let engine: Engine = if let Some(chosen) = &args.engine {
        // test if engine exists in PATH or as a literal path
        if !(util::executable_exists(chosen) || std::path::Path::new(chosen).exists()) {
            eprintln!("Engine '{}' not found in PATH or filesystem", chosen);
            return Err(ExitCode::FAILURE);
        }

        Engine::detect(chosen).expect("Failed to detect engine kind")
    } else if let Some(found) = Engine::find_available_engine() {
        found
    } else {
        eprintln!("No compatible container engine found in PATH");
        return Err(ExitCode::FAILURE);
    };

    // prevent running with docker for now
    if let util::EngineKind::Docker = engine.kind {
        eprintln!("Docker is not supported at the moment");
        return Err(ExitCode::FAILURE);
    }

    Ok(engine)
}

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    match command(&args) {
        Ok(_) => ExitCode::SUCCESS,
        Err(x) => x,
    }
}

fn command(args: &cli::Cli) -> Result<(), ExitCode> {
    // TODO there is a bit too much cloning here, not that it matters much
    match &args.cmd {
        CliCommands::Start(x) => commands::start_container(get_engine(&args)?, args.dry_run, x.clone()),
        CliCommands::Shell(x) => commands::open_shell(get_engine(&args)?, args.dry_run, x.clone()),
        CliCommands::Exec(x) => commands::container_exec(get_engine(&args)?, args.dry_run, x.clone()),
        CliCommands::Exists(x) => commands::container_exists(get_engine(&args)?, x.clone()),
        CliCommands::Config(subcmd) => match subcmd {
            ConfigCommands::Extract(x) => commands::extract_config(get_engine(&args)?, args.dry_run, &x),
            ConfigCommands::Inspect(x) => commands::inspect_config(&x),
            ConfigCommands::Options => { commands::show_config_options(); Ok(()) },
        },
        CliCommands::List => commands::print_containers(get_engine(&args)?, args.dry_run),
        CliCommands::Logs(x) => commands::print_logs(get_engine(&args)?, x.clone()),
        CliCommands::Kill(x) => commands::kill_container(get_engine(&args)?, args.dry_run, x.clone()),
        CliCommands::Init(x) => {
            if !util::is_in_container() {
                eprintln!("Running init outside a container is dangerous, qutting..");
                return Err(ExitCode::FAILURE);
            }

            commands::container_init(x.clone())
        },
    }.map_err(|x| ExitCode::from(x))
}
