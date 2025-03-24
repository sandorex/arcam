use crate::config::Config;
use crate::engine::Engine;
use crate::prelude::*;
use std::path::PathBuf;
use users::os::unix::UserExt;

/// Context used throughout the application
pub struct Context {
    pub user: String,
    pub user_home: PathBuf,
    pub user_id: u32,
    pub user_gid: u32,

    /// Current working directory
    pub cwd: PathBuf,

    /// Meant mostly for debugging, print commands instead of executing them
    pub dry_run: bool,

    /// Directory where app related configuration files reside
    pub app_dir: PathBuf,

    /// Engine to use
    pub engine: Box<dyn Engine>,
}

/// Get app configuration directory
fn get_app_dir() -> PathBuf {
    // prefer custom path from environment
    match std::env::var(crate::ENV_APP_DIR) {
        Ok(x) => PathBuf::from(x),
        Err(_) => {
            // respect XDG standard
            let xdg_config_home = match std::env::var("XDG_CONFIG_HOME") {
                Ok(x) => x,
                // fallback to ~/.config
                Err(_) => {
                    let home = std::env::var("HOME").expect("Failed to get HOME dir from env var");

                    PathBuf::from(home)
                        .join(".config")
                        .to_str()
                        .unwrap()
                        .to_string()
                }
            };

            // use bin name for dir name
            PathBuf::from(xdg_config_home).join(crate::APP_NAME)
        }
    }
}

impl Context {
    /// Construct new context with current user
    pub fn new(dry_run: bool, engine: Box<dyn Engine>) -> Result<Self> {
        use users::{get_current_uid, get_user_by_uid};

        let uid = get_current_uid();
        let user =
            get_user_by_uid(uid).ok_or(anyhow::anyhow!("Unable to find user by id {}", uid))?;

        Ok(Self {
            user: user.name().to_string_lossy().to_string(),
            user_home: user.home_dir().to_path_buf(),
            user_id: uid,
            user_gid: user.primary_group_id(),

            cwd: std::env::current_dir().with_context(|| "Failed to get current directory")?,
            dry_run,
            app_dir: get_app_dir(),
            engine,
        })
    }

    /// State directory for this app, respects XDG_STATE_HOME env var but
    /// defaults to `~/.local/state/` when undefined
    pub fn get_local_state_dir(&self) -> PathBuf {
        // respect XDG standard
        match std::env::var("XDG_STATE_HOME") {
            Ok(x) => PathBuf::from(x),
            // fallback to ~/.local/state/
            Err(_) => PathBuf::from("/home/")
                .join(&self.user)
                .join(".local")
                .join("state"),
        }
        .join(crate::APP_NAME)
    }

    /// Get container configuration directory
    pub fn config_dir(&self) -> PathBuf {
        self.app_dir.join("configs")
    }

    /// Get path to this executable
    pub fn get_executable_path(&self) -> Result<PathBuf> {
        std::env::current_exe().with_context(|| "Failed to get executable path")
    }

    /// Get all owned containers in this directory
    pub fn get_cwd_containers(&self) -> Result<Vec<String>> {
        self.engine.get_containers(vec![(
            crate::CONTAINER_LABEL_HOST_DIR,
            Some(&self.cwd.to_string_lossy()),
        )])
    }

    /// Tries to find config by the name
    pub fn find_config(&self, name: &str) -> Result<Config> {
        let path = self.config_dir().as_path().join(format!("{}.toml", name));
        crate::config::ConfigFile::config_from_file(&path)
    }
}
