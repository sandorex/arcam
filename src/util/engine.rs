use std::fmt::Display;
use super::executable_in_path;

#[derive(Debug, Clone)]
pub enum EngineKind {
    Podman,
}

impl TryFrom<String> for EngineKind {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "podman" => Ok(Self::Podman),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Engine {
    /// Path to the engine, can also be name in PATH
    pub path: String,

    /// See `EngineKind`
    pub kind: EngineKind,
}

impl Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self.kind).to_lowercase())
    }
}

impl Engine {
    /// Finds first available engine, prioritizes podman!
    pub fn find_available_engine() -> Option<Self> {
        if executable_in_path("podman") {
            return Some(
                Self {
                    path: "podman".into(),
                    kind: EngineKind::Podman,
                }
            );
        }

        None
    }
}
