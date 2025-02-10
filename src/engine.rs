//! Engine specific abstraction

pub mod podman;

use crate::prelude::*;

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

// TODO add kill_container fn for tests
pub trait Engine {
    /// Returns lowercase name of the engine
    fn name(&self) -> &str;

    /// Execute command as root inside a container
    fn exec<T: AsRef<std::ffi::OsStr>>(&self, container: &str, command: &[T]) -> Result<String>;

    /// Creates `std::process::Command` with program_name being the engine path
    fn command(&self) -> std::process::Command;

    /// Returns list of all containers filtered by label, and optionally value
    fn get_containers(&self, label: &str, value: Option<&str>) -> Result<Vec<String>>;

    /// Inspects container and returns the data associated
    fn inspect_container(&self, container: &str) -> Result<impl ContainerInfo>;
}

