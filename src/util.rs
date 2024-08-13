mod container;
mod engine;

pub use container::*;
pub use engine::*;

use std::process::{Command, ExitCode};
use std::path::PathBuf;
use std::collections::HashMap;

// TODO clean this whole module up so its organized better

/// Simple extension trait to avoid duplicating code, allow easy conversion to `ExitCode`
pub trait CommandOutputExt {
    /// Convert into `std::process::ExitCode` easily consistantly
    ///
    /// Equal to `ExitCode::from(1)` in case of signal termination (or any exit code larger than 255)
    fn to_exitcode(&self) -> ExitCode;
}

impl CommandOutputExt for std::process::ExitStatus {
    fn to_exitcode(&self) -> ExitCode {
        // the unwrap_or(1) s are cause even if conversion fails it still failed just termination
        // by signal is larger than 255 that u8 exit code on unix allows
        ExitCode::from(TryInto::<u8>::try_into(self.code().unwrap_or(1)).unwrap_or(1))
    }
}

impl CommandOutputExt for std::process::Output {
    fn to_exitcode(&self) -> ExitCode {
        self.status.to_exitcode()
    }
}

/// Get hostname from system using `hostname` command
#[cfg(target_os = "linux")]
pub fn get_hostname() -> String {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        return env_hostname;
    }

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname").output().expect("Could not call hostname");
    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if ! cmd.status.success() || hostname.is_empty() {
        panic!("Unable to get hostname from host");
    }

    hostname.trim().into()
}

/// Generates random name using adjectives list
///
/// Uses system time so its not really random cause im stingy about dependencies
pub fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &str = include_str!("adjectives.txt");

    // NOTE: pseudo-random without crates!
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos: usize = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        .try_into()
        .unwrap();

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.get(nanos % adjectives.len()).unwrap();

    format!("{}-box", adjective)
}

#[cfg(target_os = "linux")]
pub fn get_user() -> String { std::env::var("USER").expect("Unable to get USER from env var") }

pub fn get_container_ws(engine: &Engine, container: &str) -> Option<String> {
    // {{if .. }} is added so that the stdout is empty if ws is none
    let cmd = Command::new(&engine.path)
        .args(["inspect", container, "--format", "{{if .Config.Labels.box_ws}}{{.Config.Labels.box_ws}}{{end}}"])
        .output()
        .expect("Could not execute engine");

    if cmd.status.success() {
        let stdout = String::from_utf8(cmd.stdout).unwrap();
        let trimmed = stdout.trim();

        // check if stdout is empty (will have some whitespace though!)
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        None
    }
}

/// Prints command which would've been ran, pretty ugly but should properly quote things, keyword
/// being SHOULD
pub fn print_cmd_dry_run(engine: &Engine, args: Vec<String>) {
    print!("(CMD) {}", &engine.path);
    for i in args {
        print!(" '{}'", i);
    }
    println!();
}

/// Get app configuration directory
pub fn app_dir() -> PathBuf {
    const BOX_DIR: &str = "box";

    // prefer custom path from environment
    match std::env::var("BOX_DIR") {
        Ok(x) => PathBuf::from(x),
        Err(_) => {
            // respect XDG standard
            let xdg_config_home = match std::env::var("XDG_CONFIG_HOME") {
                Ok(x) => x,
                // fallback to ~/.config
                Err(_) => {
                    let home = std::env::var("HOME").expect("Failed to get HOME dir from env var");

                    PathBuf::from(home).join(".config").to_str().unwrap().to_string()
                },
            };

            PathBuf::from(xdg_config_home).join(BOX_DIR)
        },
    }
}

/// Get container configuration directory
pub fn config_dir() -> PathBuf {
    app_dir().join("configs")
}

/// Loads all configs while also handling all errors
pub fn load_configs() -> Option<HashMap<String, crate::config::Config>> {
    use crate::config;

    match config::load_from_dir(config_dir().to_str().unwrap()) {
        Ok(x) => Some(x),
        Err(err) => {
            eprintln!("{}\n", err);

            None
        },
    }
}

#[link(name = "c")]
extern "C" {
    fn geteuid() -> u32;
    fn getegid() -> u32;
}

/// Get user UID and GID
pub fn get_user_uid_gid() -> (u32, u32) {
    // TODO SAFETY is this unsafe just cause or?
    unsafe {
        (
            geteuid(),
            getegid(),
        )
    }
}

