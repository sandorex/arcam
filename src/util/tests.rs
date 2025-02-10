//! Contains helper functions for tests
use assert_cmd::Command;
use std::error::Error;

#[allow(unused_imports)]
pub mod prelude {
    pub use super::Result;
    pub use super::podman_cleanup;
}

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// RAII structure to kill container on drop
pub struct ContainerCleanup {
    name: String,
}

impl Drop for ContainerCleanup {
    fn drop(&mut self) {
        let cmd_name = "podman";

        let exists = Command::new(cmd_name)
            .args(["container", "exists", &self.name])
            .assert()
            .get_output()
            .status
            .success();

        // ignore it if the container does not exist
        if !exists {
            return;
        }

        let cmd = Command::new(cmd_name)
            .args(["container", "kill", &self.name])
            .assert();

        let cmd = cmd.get_output();

        if cmd.status.success() {
            println!("Container {:?} cleaned up successfully", self.name);
        } else {
            println!("Failed to clean up container {:?}", self.name);
        }
    }
}

pub fn podman_cleanup(name: &str) -> ContainerCleanup {
    ContainerCleanup {
        name: name.to_string(),
    }
}

