mod test_permissions;

use crate::engine::Engine;
use anyhow::Result;
use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

// NOTE: This test is not useless, it prevents running tests on outdated main binary
#[test]
fn test_sanity() -> Result<()> {
    assert_cmd::Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["--version"])
        .assert()
        .success()
        .stdout(format!("arcam {}\n", crate::FULL_VERSION));

    Ok(())
}

#[allow(unused)]
pub mod prelude {
    pub use super::Container;
    pub use crate::engine::Engine;
    pub use anyhow::Result;
}

/// RAII guard to stop running containers
pub struct Container {
    pub engine: Box<dyn Engine>,
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
