mod util;
mod cli;
mod commands;

use clap::Parser;
use std::process::ExitCode;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const FULL_VERSION: &'static str = concat!(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
pub const DATA_VOLUME_NAME: &'static str = "box-data";

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    // init does not need engine, just get it from environment if needed
    if let CliCommands::Init = args.cmd {
        if !util::is_in_container() {
            eprintln!("Running init outside a container is dangerous, qutting..");
            return ExitCode::FAILURE;
        }

        return commands::container_init()
    }

    // prefer the one in argument or ENV then try to find one automatically
    let engine = {
        if let Some(chosen) = args.engine {
            chosen
        } else {
            if let Some(found) = util::find_available_engine() {
                found
            } else {
                eprintln!("No compatible container engine found in PATH");
                return ExitCode::FAILURE;
            }
        }
    };

    // test if engine exists in PATH or as a literal path
    if !(util::executable_exists(&engine) || std::path::Path::new(&engine).exists()) {
        eprintln!("Engine '{}' not found in PATH or filesystem", engine);
        return ExitCode::FAILURE;
    }

    use cli::CliCommands;
    match args.cmd {
        CliCommands::Start(x) => commands::start_container(&engine, args.dry_run, &x),
        CliCommands::Shell(x) => commands::open_shell(&engine, args.dry_run, &x),
        CliCommands::Exec(x) => commands::container_exec(&engine, args.dry_run, &x),
        CliCommands::Exists(x) => commands::container_exists(&engine, &x),
        CliCommands::List => commands::print_containers(&engine),
        CliCommands::Kill(x) => commands::kill_container(&engine, args.dry_run, &x),
        CliCommands::Init => unreachable!(),
    }
}
