//! Contains helper functions for tests
use std::error::Error;
use crate::engine::Engine;

#[allow(unused_imports)]
pub mod prelude {
    pub use super::Result;
    pub use super::Container;
}

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub enum Container<'a> {
    Podman(&'a str),
}

impl Drop for Container<'_> {
    fn drop(&mut self) {
        match self {
            Self::Podman(container) => {
                // ignore it if the container does not exist
                if !Engine::Podman.container_exists(container).unwrap() {
                    return;
                }

                match Engine::Podman.stop_container(container) {
                    Ok(_) => println!("Container {container:?} cleaned up successfully"),
                    Err(_) => println!("Failed to clean up container {container:?}"),
                }
            }
        }
    }
}

