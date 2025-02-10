//! Engine specific abstraction

pub mod podman;

use crate::prelude::*;
use crate::util::command_extensions::*;

/// Data returned from container engine on inspection of a container
pub trait ContainerInfo {
    /// Get container name
    fn get_name(&self) -> &str;

    /// Get container label
    fn get_label(&self, name: &str) -> Option<&str>;

    /// Check if container label exists
    fn has_label(&self, name: &str) -> bool {
        self.get_label(name).is_some()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Engine {
    Podman
}

// pub trait Engine {
//     /// Returns lowercase name of the engine
//     fn name() -> &'static str;
//
//     /// Execute command as root inside a container
//     fn exec<T: AsRef<std::ffi::OsStr>>(&self, container: &str, command: &[T]) -> Result<String>;
//
//     /// Creates `std::process::Command` with program_name being the engine path
//     fn command(&self) -> std::process::Command;
//
//     /// Returns list of all containers filtered by label, and optionally value
//     fn get_containers(&self, label: &str, value: Option<&str>) -> Result<Vec<String>>;
//
//     /// Inspects container and returns the data associated
//     fn inspect_container(&self, container: &str) -> Result<impl ContainerInfo>;
//
//     /// Check if container exists, should be faster than `inspect_container`
//     fn container_exists(&self, container: &str) -> Result<bool>;
//
//     /// Gently shutdown a container, after a timeout kill it forcefully if still running
//     fn stop_container(&self, container: &str) -> Result<()>;
// }

