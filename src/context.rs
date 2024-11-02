use std::path::PathBuf;
use crate::prelude::*;
use users::os::unix::UserExt;
use crate::util::Engine;
use std::collections::HashMap;
use crate::util::command_extensions::*;

/// Possible status of a container
#[derive(Debug)]
pub enum ContainerStatus {
    Created,
    Exited,
    Paused,
    Running,
    Unknown,
}

/// Context used throughout the application
#[derive(Debug)]
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

    /// Which container engine to use
    pub engine: Engine,
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

                    PathBuf::from(home).join(".config").to_str().unwrap().to_string()
                },
            };

            // use bin name for dir name
            PathBuf::from(xdg_config_home).join(crate::APP_NAME)
        },
    }
}

impl Context {
    /// Construct new context with current user
    pub fn new(dry_run: bool, engine: Engine) -> Result<Self> {
        use users::{get_current_uid, get_user_by_uid};
        let uid = get_current_uid();
        let user = get_user_by_uid(uid)
            .ok_or(anyhow::anyhow!("Unable to find user by id {}", uid))?;

        Ok(Self {
            user: user.name().to_string_lossy().to_string(),
            user_home: user.home_dir().to_path_buf(),
            user_id: uid,
            user_gid: user.primary_group_id(),

            cwd: std::env::current_dir()
                .with_context(|| "Failed to get current directory")?,
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
        }.join(crate::APP_NAME)
    }

    /// Get container configuration directory
    pub fn config_dir(&self) -> PathBuf {
        self.app_dir.join("configs")
    }

    /// Get path to this executable
    pub fn get_executable_path(&self) -> Result<PathBuf> {
        std::env::current_exe()
            .with_context(|| "Failed to get executable path")
    }

    /// Get containers running in cwd
    pub fn get_cwd_container(&self) -> Option<Vec<String>> {
        let cmd = self.engine_command()
            .args(["container", "ls", "--format", "{{.Names}}", "--sort", "created"])
            .args(["--filter".into(), format!("label={}={}", crate::CONTAINER_LABEL_HOST_DIR, self.cwd.to_string_lossy())])
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if cmd.status.success() {
            let stdout = String::from_utf8_lossy(&cmd.stdout);
            let trimmed = stdout.trim();

            // check if stdout is empty
            if trimmed.is_empty() {
                // return empty vec to signify none were found
                Some(vec![])
            } else {
                // collect the lines
                // NOTE reversing to get youngest container first
                Some(trimmed.lines().rev().map(|x| x.to_string()).collect())
            }
        } else {
            // could not get the containers for some reason
            None
        }
    }

    /// Creates `std::process::Command` with engine
    pub fn engine_command(&self) -> std::process::Command {
        std::process::Command::new(&self.engine.path)
    }

    /// Execute engine command and get output back, the user is root!
    pub fn engine_exec_root(&self, container: &str, cmd: Vec<&str>) -> Result<String> {
        let cmd_result = self.engine_command()
            .args(["exec", "--user", "root", "-it", container])
            .args(&cmd)
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        let stdout = String::from_utf8_lossy(&cmd_result.stdout);
        if !cmd_result.status.success() {
            return Err(anyhow!("Engine command {:?} exited with code {}", cmd, cmd_result.get_code()));
        }

        Ok(stdout.to_string())
    }

    pub fn engine_container_exists(&self, container: &str) -> bool {
        self.engine_command()
            .args(["container", "exists", container])
            .output()
            .expect(crate::ENGINE_ERR_MSG)
            .status.success()
    }

    /// Gets value of label on a container if it is defined
    pub fn get_container_label(&self, container: &str, label: &str) -> Option<String> {
        let key = format!(".Config.Labels.{}", label);

        // this looks like a mess as i need to escape curly braces
        //
        // basically return key if it exists
        // {{if .. }} is added so that the stdout is empty if ws is none
        let format = format!("{{{{ if {0} }}}}{{{{ {0} }}}}{{{{ end }}}}", key);

        let cmd = self.engine_command()
            .args(["inspect", container, "--format", format.as_str()])
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if cmd.status.success() {
            let stdout = String::from_utf8_lossy(&cmd.stdout);
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


    /// Get container status if it exists
    pub fn get_container_status(&self, container: &str) -> Option<ContainerStatus> {
        let cmd = self.engine_command()
            .args(["container", "inspect", container, "--format", "{{.State.Status}}"])
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        // the container does not exist
        if !cmd.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&cmd.stdout);
        Some(match stdout.trim() {
            "created" => ContainerStatus::Created,
            "exited" => ContainerStatus::Exited,
            "paused" => ContainerStatus::Paused,
            "running" => ContainerStatus::Running,
            _ => ContainerStatus::Unknown,
        })
    }

    /// Loads all configs while also handling all errors
    pub fn load_configs(&self) -> Result<HashMap<String, crate::config::Config>> {
        crate::config::load_from_dir(self.config_dir().as_path())
            .map_err(|err| anyhow!("{}", err))
    }
}
