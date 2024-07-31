mod util;
mod cli;
mod commands;

use std::path::Path;
use std::process::ExitCode;
use clap::Parser;
use std::process::Command;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const FULL_VERSION: &'static str = concat!(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
pub const DATA_VOLUME_NAME: &'static str = "box-data";

/// Check if running inside a container
fn is_in_container() -> bool {
    return Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
        || std::env::var("container").is_ok()
}

/// Generates random name using adjectives list
fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &'static str = include_str!("adjectives.txt");

    use rand::seq::SliceRandom;

    let mut rng = rand::thread_rng();

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.choose(&mut rng)
        .expect("Random adjective is empty")
        .to_string();

    return format!("{}-box", adjective);
}

fn get_user() -> String {
    std::env::var("USER").expect("Unable to get USER from env var")
}

// TODO remove this function as its quite pointless
/// Run engine command and save the output, cannot be run dry
fn engine_cmd_output(engine: &str, args: Vec<String>) -> Result<std::process::Output, std::process::Output> {
    let cmd = Command::new(engine)
        .args(args)
        .output()
        .expect("Could not execute engine");

    // basically wrap it based on the success so i can use .expect and such functions
    if cmd.status.success() {
        Ok(cmd)
    } else {
        Err(cmd)
    }
}

// TODO remove this and add print_cmd() for dry_run
/// Run engine command but keep the stdin/stdout the same so it will be printed
fn engine_cmd_status(engine: &str, dry_run: bool, args: Vec<String>) -> Result<u8, u8> {
    if dry_run {
        println!("(CMD) {} {:?}", engine, args);

        Ok(0)
    } else {
        let cmd = Command::new(engine)
            .args(args)
            .status()
            .expect("Could not execute engine");

        if cmd.success() {
            Ok(0)
        } else {
            Err(cmd.code().unwrap_or(1).try_into().unwrap())
        }
    }
}

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    // init does not need engine, just get it from environment if needed
    if let CliCommands::Init = args.cmd {
        if !is_in_container() {
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
    ExitCode::from(match args.cmd {
        CliCommands::Start(x) => commands::start_container(&engine, args.dry_run, &x),
        CliCommands::Shell(x) => commands::open_shell(&engine, args.dry_run, &x),
        CliCommands::Exec(x) => commands::container_exec(&engine, args.dry_run, &x),
        CliCommands::Exists(x) => commands::container_exists(&engine, &x),
        CliCommands::List => commands::print_containers(&engine),
        CliCommands::Kill(x) => commands::kill_container(&engine, args.dry_run, &x),
        CliCommands::Init => unreachable!(),
    })
}
