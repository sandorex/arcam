//! Engine specific abstraction

mod podman;

pub use podman::*;

use crate::command_extensions::*;
use crate::prelude::*;
use std::collections::HashMap;
use std::fmt::Display;

pub struct ContainerInfo {
    pub name: String,
    pub labels: HashMap<String, String>,
}

pub trait Engine: Display {
    /// Returns formatted name of the engine
    fn name(&self) -> &str;

    /// Creates `std::process::Command` with program_name being the engine path
    fn command(&self) -> Command {
        Command::new(self.name())
    }

    /// Execute command as root inside a container
    fn exec(&self, container: &str, cmd: &[&str]) -> Result<String>;

    /// Returns list of all containers filtered by label, and optionally value
    fn get_containers(&self, labels: Vec<(&str, Option<&str>)>) -> Result<Vec<String>>;

    /// Inspects containers and returns the data associated
    fn inspect_containers(&self, containers: Vec<&str>) -> Result<Vec<ContainerInfo>>;

    /// Check if container exists, should be faster than `inspect_container`
    fn container_exists(&self, container: &str) -> Result<bool>;

    #[cfg(test)]
    fn start_dummy_container(
        &self,
        image: &str,
        args: Option<Vec<&str>>,
    ) -> Result<crate::tests_prelude::Container>;

    /// Gently shutdown a container, after a timeout kill it forcefully if still running
    #[cfg(test)]
    fn stop_container(&self, container: &str) -> Result<()>;
}
