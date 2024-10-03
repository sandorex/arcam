use std::{fmt::Display, process::Command};

#[derive(Debug, Clone)]
pub enum EngineKind {
    Podman,
    Docker,
}

impl TryFrom<String> for EngineKind {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "podman" => Ok(Self::Podman),
            "docker" => Ok(Self::Docker),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl Engine {
    /// Detect which engine it is by executing `<engine> --version`
    ///
    /// If it is stupid but it works, it isn't stupid.
    /// - Mercedes Lackey
    pub fn detect(engine: &str) -> Option<Self> {
        // output from `<engine> --version`
        // docker: Docker version 27.1.1, build 6312585
        // podman: podman version 5.1.2

        let cmd = Command::new(engine)
            .args(["--version"])
            .output()
            .expect("Could not execute engine");

        // NOTE its important to make it lowercase
        let stdout = String::from_utf8_lossy(&cmd.stdout).to_lowercase();

        // convert first word into EngineKind, at least try to..
        let kind = EngineKind::try_from(
            stdout.split(" ")
            .nth(0)
            .unwrap_or("")
            .to_string()
        );
        match kind {
            Ok(x) => Some(Engine {
                path: engine.to_string(),
                kind: x,
            }),
            Err(_) => None,
        }
    }

    /// Finds first available engine, prioritizes podman!
    pub fn find_available_engine() -> Option<Self> {
        if executable_exists("podman") {
            return Some(
                Self {
                    path: "podman".into(),
                    kind: EngineKind::Podman,
                }
            );
        }

        if executable_exists("docker") {
            return Some(
                Self {
                    path: "docker".into(),
                    kind: EngineKind::Docker,
                }
            );
        }

        None
    }
}

/// Check whether executable exists in PATH
#[cfg(target_os = "linux")]
pub fn executable_exists(cmd: &str) -> bool {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("which {}", cmd))
        .output()
        .expect("Failed to execute 'which'");

    output.status.success()
}

