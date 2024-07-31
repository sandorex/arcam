mod util;
mod cli;
mod commands;

pub use std::process::ExitCode;

use clap::Parser;
use std::process::Command;

pub const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");
pub const INIT_SCRIPT: &'static str = include_str!("box-init.sh");
pub const DATA_VOLUME_NAME: &'static str = "box-data";

/// Sets required constants inside the init script
fn template_init_script(user: &str) -> String {
    INIT_SCRIPT.to_string()
        .replace("@BOX_VERSION@", VERSION)
        .replace("@BOX_USER@", user)
}

fn get_user() -> String {
    std::env::var("USER").expect("Unable to get USER from env var")
}

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

/// Run engine command but keep the stdin/stdout the same so it will be printed
fn engine_cmd_status(engine: &str, dry_run: bool, args: Vec<String>) -> Result<u8, u8> {
    if dry_run {
        println!("(CMD) {} {}", engine, args.join(" "));

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

/// Extracts default shell for user from /etc/passwd inside a container
fn get_user_shell(engine: &str, container: &str, user: &str) -> String {
    let cmd_result = Command::new(engine)
        .args(&["exec", "--user", "root", "-it", &container, "getent", "passwd", user])
        .output()
        .expect("Could not execute engine");

    const ERR: &'static str = "Failed to extract default shell from /etc/passwd";
    let stdout = String::from_utf8_lossy(&cmd_result.stdout);
    if ! cmd_result.status.success() || stdout.is_empty() {
        panic!("{}", ERR);
    }

    // i do not want to rely on external tools like awk so im extracting manually
    stdout.trim().split(':').last().map(|x| x.to_string())
        .expect(ERR)
}

fn main() -> ExitCode {
    let args = cli::Cli::parse();

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
        CliCommands::Shell(x) => commands::open_shell(&engine, &x),
        CliCommands::Exec(x) => commands::container_exec(&engine, &x),
        CliCommands::List => commands::print_containers(&engine),
        CliCommands::Kill(x) => commands::kill_container(&engine, &x),
    })
}
