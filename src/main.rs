mod util;
mod cli;
mod commands;

pub use std::process::ExitCode;

use clap::Parser;
use std::process::Command;

pub const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");
pub const INIT_SCRIPT: &'static str = include_str!("box-init.sh");
pub const DATA_VOLUME_NAME: &'static str = "box-data";

static mut DRY_RUN: bool = false;

/// Sets required constants inside the init script
fn template_init_script(user: &str) -> String {
    INIT_SCRIPT.to_string()
        .replace("@BOX_VERSION@", VERSION)
        .replace("@BOX_USER@", user)
}

fn get_user() -> String {
    std::env::var("USER").expect("Unable to get USER from env var")
}

// TODO also will allow easy implementation of dry_run
// fn run_engine_cmd(engine: &str, args: Vec<String>) -> Result<(), ()> {
//     Ok(())
// }

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

    // there is no threading in this application, and it is written to only here
    unsafe {
        DRY_RUN = args.dry_run;
    }

    // prefer the one in argument or ENV then try to find one automatically
    let engine = {
        if let Some(chosen) = args.engine {
            chosen
        } else {
            if let Some(found) = util::find_available_engine() {
                found
            } else {
                println!("No compatible container engine found in PATH");
                std::process::exit(1);
            }
        }
    };

    // TODO test if the engine exists at all

    use cli::CliCommands;
    ExitCode::from(match args.cmd {
        CliCommands::Start(x) => commands::start_container(&engine, &x),
        CliCommands::Shell(x) => commands::open_shell(&engine, &x),
        CliCommands::Exec(x) => commands::container_exec(&engine, &x),
        CliCommands::List => commands::print_containers(&engine),
        CliCommands::Kill(x) => commands::kill_container(&engine, &x),
    })
}
