use std::process::{Command, ExitCode};

/// Simple extension trait to avoid duplicating code, allow easy conversion to `ExitCode`
pub trait CommandOutputExt {
    /// Convert into `std::process::ExitCode` easily consistantly
    ///
    /// Equal to `ExitCode::from(1)` in case of signal termination (or any exit code larger than 255)
    fn to_exitcode(&self) -> ExitCode;
}

impl CommandOutputExt for std::process::ExitStatus {
    fn to_exitcode(&self) -> ExitCode {
        // the unwrap_or(1) s are cause even if conversion fails it still failed just termination
        // by signal is larger than 255 that u8 exit code on unix allows
        ExitCode::from(TryInto::<u8>::try_into(self.code().unwrap_or(1)).unwrap_or(1))
    }
}

impl CommandOutputExt for std::process::Output {
    fn to_exitcode(&self) -> ExitCode {
        self.status.to_exitcode()
    }
}

/// Basically carries path to the engine and which engine it actually is
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Engine {
    Podman(String),
    Docker(String),
}

#[allow(dead_code)]
impl Engine {
    /// Create enum from named string
    pub fn from_name(name: &str, path: &str) -> Option<Self> {
        match name {
            "podman" => Some(Self::Podman(path.to_string())),
            "docker" => Some(Self::Docker(path.to_string())),
            _ => None,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Podman(_) => "podman",
            Self::Docker(_) => "docker",
        }
    }

    pub fn get_path(&self) -> &String {
        match self {
            Self::Podman(x) => &x,
            Self::Docker(x) => &x,
        }
    }

    /// Detect which engine it is by executing it
    ///
    /// Do not know how reliable it is though
    pub fn detect(engine: &str) -> Option<Self> {
        // outputs:
        // docker: Docker version 27.1.1, build 6312585
        // podman: podman version 5.1.2

        let cmd = Command::new(engine)
            .args(&["--version"])
            .output()
            .expect("Could not execute engine");

        // just in case i am making it lowercase
        let stdout = String::from_utf8_lossy(&cmd.stdout).to_lowercase();

        // decide based on the first word
        Self::from_name(stdout.split(" ").nth(0).unwrap_or(""), engine)
    }
}

/// Possible status of a container
#[derive(Debug)]
pub enum ContainerStatus {
    Created,
    Exited,
    Paused,
    Running,
    Unknown,
}

/// Get container status if it exists
pub fn get_container_status(engine: &Engine, container: &str) -> Option<ContainerStatus> {
    let cmd = Command::new(engine.get_path())
        .args(&["container", "inspect", container, "--format", "{{.State.Status}}"])
        .output()
        .expect("Could not execute engine");

    // the container does not exist
    if ! cmd.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&cmd.stdout).to_string();
    Some(match stdout.as_str() {
        "created" => ContainerStatus::Created,
        "exited" => ContainerStatus::Exited,
        "paused" => ContainerStatus::Paused,
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Unknown,
    })
}

/// Check if container is owned by box, will return false if container does not exist
pub fn is_box_container(engine: &Engine, name: &str) -> bool {
    let cmd = Command::new(engine.get_path())
        .args(&["container", "inspect", name, "--format", "{{if .Config.Labels.box}}{{.Config.Labels.box}}{{end}}"])
        .output()
        .expect("Could not execute engine");

    cmd.status.success() && !String::from_utf8_lossy(&cmd.stdout).is_empty()
}

/// Check whether executable exists in PATH
#[cfg(target_os = "linux")]
pub fn executable_exists(cmd: &str) -> bool {
    let output = Command::new("sh")
        .arg("-c").arg(format!("which {}", cmd))
        .output()
        .expect("Failed to execute 'which'");

    output.status.success()
}

/// Finds first available engine, prioritizes podman!
pub fn find_available_engine() -> Option<Engine> {
    if executable_exists("podman") {
        return Some(Engine::Podman("podman".to_string()));
    }

    if executable_exists("docker") {
        return Some(Engine::Docker("docker".to_string()));
    }

    None
}

/// Helper to get hostname using `hostname` utility which should be available on most linux systems
#[cfg(target_os = "linux")]
pub fn get_hostname() -> String {
    let cmd = Command::new("hostname").output().expect("Could not call hostname");
    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if ! cmd.status.success() || hostname.is_empty() {
        panic!("Unable to get hostname from host");
    }

    hostname.trim().into()
}

/// Check if running inside a container
pub fn is_in_container() -> bool {
    return std::path::Path::new("/run/.containerenv").exists()
        || std::path::Path::new("/.dockerenv").exists()
        || std::env::var("container").is_ok()
}

/// Generates random name using adjectives list
pub fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &'static str = include_str!("adjectives.txt");

    // NOTE: pseudo-random without crates!
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos: usize = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        .try_into()
        .unwrap();

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.iter().nth(nanos % adjectives.len()).unwrap();

    return format!("{}-box", adjective);
}

pub fn get_user() -> String { std::env::var("USER").expect("Unable to get USER from env var") }

/// Prints command which would've been ran, pretty ugly but should properly quote things, keyword
/// being SHOULD
pub fn print_cmd_dry_run(engine: &Engine, args: Vec<String>) {
    print!("(CMD) {}", engine.get_path());
    for i in args {
        print!(" '{}'", i);
    }
    println!();
}

