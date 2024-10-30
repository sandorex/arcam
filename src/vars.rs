//! Contains all environment vars

use crate::ENV_VAR_PREFIX;

/// Wayland socket to pass through
pub const WAYLAND_DISPLAY: &str = ENV_VAR_PREFIX!("WAYLAND_DISPLAY");

/// Container name
pub const CONTAINER: &str = ENV_VAR_PREFIX!("CONTAINER");

/// Suffix added to each unnamed container created
pub const CONTAINER_SUFFIX: &str = ENV_VAR_PREFIX!("CONTAINER_SUFFIX");

/// Image or config to use by default
pub const IMAGE: &str = ENV_VAR_PREFIX!("IMAGE");

/// Directory where the app stores data
pub const APP_DIR: &str = ENV_VAR_PREFIX!("DIR");
