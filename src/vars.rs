//! File containing constants

/// Prefix env var name with proper prefix
#[macro_export]
macro_rules! ENV_VAR_PREFIX {
    ($($args:literal),*) => {
        concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_", $($args),*)
    };
}

pub const ENGINE_ERR_MSG: &str = "Failed to execute engine";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FULL_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_GIT_BRANCH"),
    ")"
);

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_NAME_UPPERCASE: &str = env!("CARGO_PKG_NAME_UPPERCASE");

/// Container label used to detect if container is made by arcam
pub const CONTAINER_LABEL_APP: &str = env!("CARGO_PKG_NAME");

/// Container label used to specify the host directory where container was started
pub const CONTAINER_LABEL_HOST_DIR: &str = "host_dir";

/// Container label used to specify the path to main project in the container
pub const CONTAINER_LABEL_CONTAINER_DIR: &str = "container_dir";

/// Container label used to specify default shell
pub const CONTAINER_LABEL_USER_SHELL: &str = "default_shell";

/// Set log level from the environ
pub const ENV_LOG_LEVEL: &str = "LOG_LEVEL";

/// Wayland socket to pass through
pub const ENV_WAYLAND_DISPLAY: &str = ENV_VAR_PREFIX!("WAYLAND_DISPLAY");

/// Container name
pub const ENV_CONTAINER: &str = ENV_VAR_PREFIX!("CONTAINER");

/// Suffix added to each unnamed container created
pub const ENV_CONTAINER_SUFFIX: &str = ENV_VAR_PREFIX!("CONTAINER_SUFFIX");

/// Image or config to use by default
pub const ENV_IMAGE: &str = ENV_VAR_PREFIX!("IMAGE");

/// Directory where the app stores data
pub const ENV_APP_DIR: &str = ENV_VAR_PREFIX!("DIR");

/// Should start open the shell automatically
pub const ENV_ENTER_ON_START: &str = ENV_VAR_PREFIX!("ENTER_ON_START");

/// Stores path to arcam executable and prevents infinite loop with host_pre_init
pub const ENV_EXE_PATH: &str = ENV_VAR_PREFIX!("EXE_PATH");

/// Where scripts are executed from
pub const INIT_D_DIR: &str = "/init.d";

/// Path where all arcam related things should be
pub const ARCAM_DIR: &str = "/arcam";

/// Path where arcam binary is mounted
pub const ARCAM_EXE: &str = "/arcam/exe";

/// Path to optional config file distributed within the image
pub const ARCAM_CONFIG: &str = "/config.toml";

/// This file existing is a signal when container initialization is finished
pub const FLAG_FILE_INIT: &str = "/arcam/initialized";

/// This file existing is a signal start command has copied all data required in the container so
/// the initialization can begin
pub const FLAG_FILE_PRE_INIT: &str = "/arcam/preinit";
