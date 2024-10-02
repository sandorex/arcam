mod container;
mod engine;
mod command;

pub use container::*;
pub use engine::*;
pub use command::*;

use std::path::PathBuf;
use std::collections::HashMap;

pub type ExitResult = Result<(), u8>;

/// Get app configuration directory
pub fn app_dir() -> PathBuf {
    // prefer custom path from environment
    match std::env::var(crate::ENV_VAR_PREFIX!("DIR")) {
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

            // use bin name for dir name
            PathBuf::from(xdg_config_home).join(crate::APP_NAME)
        },
    }
}

/// Get container configuration directory
pub fn config_dir() -> PathBuf {
    app_dir().join("configs")
}

/// Loads all configs while also handling all errors
pub fn load_configs() -> Result<HashMap<String, crate::config::Config>, u8> {
    use crate::config;

    match config::load_from_dir(config_dir().to_str().unwrap()) {
        Ok(x) => Ok(x),
        Err(err) => {
            eprintln!("{}\n", err);

            Err(1)
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
    unsafe {
        (
            geteuid(),
            getegid(),
        )
    }
}

/// Generate random number using `/dev/urandom`
pub fn rand() -> u32 {
    use std::io::Read;

    const ERR_MSG: &str = "Error reading /dev/urandom";

    let mut rng = std::fs::File::open("/dev/urandom")
        .expect(ERR_MSG);

    let mut buffer = [0u8; 4];
    rng.read_exact(&mut buffer)
        .expect(ERR_MSG);

    u32::from_be_bytes(buffer)
}

/// Simple yes/no prompt
pub fn prompt(prompt: &str) -> bool {
    use std::io::Write;
    let mut s = String::new();

    // if not yes then yes, but if yes then no yes
    print!("{} [y/N] ", prompt);

    let _ = std::io::stdout().flush();

    std::io::stdin().read_line(&mut s).expect("Could not read stdin");
    s = s.trim().to_string();

    matches!(s.to_lowercase().as_str(), "y"|"yes")
}

#[cfg(test)]
mod tests {
    use super::rand;

    #[test]
    fn test_rand() {
        // just a sanity test
        assert_ne!(rand(), rand());
    }
}

