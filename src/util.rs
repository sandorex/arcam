mod container;
mod engine;
mod command;

pub use container::*;
pub use engine::*;
pub use command::*;

use std::process::ExitCode;
use std::path::PathBuf;
use std::collections::HashMap;

pub trait ExitResultExt {
    fn to_exitcode(&self) -> ExitCode;
}

pub type ExitResult = Result<(), u8>;
impl ExitResultExt for ExitResult {
    fn to_exitcode(&self) -> ExitCode {
        // convert u8 to ExitCode
        match self {
            Ok(_) => ExitCode::SUCCESS,
            Err(x) => ExitCode::from(*x),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_user() -> String { std::env::var("USER").expect("Unable to get USER from env var") }

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

