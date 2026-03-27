#![allow(dead_code)]

mod engine;

use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

#[allow(unused)]
pub mod prelude {
    pub use super::Container;
    pub use arcam::engine::Engine;
    pub use super::engine::EngineExt;
    pub use anyhow::Result;
    pub use assert_cmd::Command;

    /// Debian image used across multiple tests
    pub const DEBIAN_IMAGE: &str = "debian:trixie";
}

/// RAII guard to stop running containers
pub struct Container {
    pub engine: Box<dyn engine::EngineExt>,
    pub container: String,
}

impl Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.container)
    }
}

impl Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.container)
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = self.engine.stop_container(&self.container);
    }
}

impl Deref for Container {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}
