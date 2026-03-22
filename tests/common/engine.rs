use anyhow::Result;
use arcam::command_extensions::*;
use arcam::engine::{self, Engine};
use super::Container;

/// Contains testing utilities for each engine
pub trait EngineExt: Engine {
    fn start_dummy_container(
        &self,
        image: &str,
        args: Option<Vec<&str>>,
    ) -> Result<Container>;

    /// Gently shutdown a container, after a timeout kill it forcefully if still running
    fn stop_container(&self, container: &str) -> Result<()>;
}

impl EngineExt for engine::Podman {
    fn start_dummy_container(
            &self,
            image: &str,
            args: Option<Vec<&str>>,
    ) -> Result<super::Container> {
        assert!(!image.is_empty());

        let mut cmd = self.command();
        cmd.args(["run", "--rm", "-d", "-it"]);

        if let Some(args) = args {
            cmd.args(args);
        }

        // image goes last
        cmd.arg(image);

        let output = cmd.log_output()?;

        Ok(Container {
            container: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            engine: Box::new(*self),
        })
    }

    fn stop_container(&self, container: &str) -> Result<()> {
        assert!(!container.is_empty());

        // gentle shutdown, terminates by default after 10s
        self.command()
            .args(["container", "stop", container])
            .log_status()?;

        Ok(())
    }
}
