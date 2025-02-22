use std::hash::Hash;
use crate::command_ext::CommandExt;

/// Generate random number using `/dev/urandom`
pub fn rand() -> u32 {
    use std::io::Read;

    const ERR_MSG: &str = "Error reading /dev/urandom";

    let mut rng = std::fs::File::open("/dev/urandom").expect(ERR_MSG);

    let mut buffer = [0u8; 4];
    rng.read_exact(&mut buffer).expect(ERR_MSG);

    u32::from_be_bytes(buffer)
}

/// Simple yes/no prompt
pub fn prompt(prompt: &str) -> bool {
    use std::io::Write;
    let mut s = String::new();

    // if not yes then yes, but if yes then no yes
    print!("{} [y/N] ", prompt);

    let _ = std::io::stdout().flush();

    std::io::stdin()
        .read_line(&mut s)
        .expect("Could not read stdin");
    s = s.trim().to_string();

    matches!(s.to_lowercase().as_str(), "y" | "yes")
}

/// Check whether executable exists in PATH
pub fn executable_in_path(cmd: &str) -> bool {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("which {}", cmd))
        .log_output(log::Level::Debug)
        .expect("Failed to execute 'which'");

    output.status.success()
}

/// Check if running inside a container
pub fn is_in_container() -> bool {
    use std::env;
    use std::path::Path;

    Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
        || env::var("container").is_ok()
}

pub trait Graph {
    /// Get dependencies
    fn graph_dependencies(&self) -> Vec<&Self>;

    fn graph_walk(&self) -> Vec<&Self> {
        let mut result = vec![];

        let deps = self.graph_dependencies();
        if !deps.is_empty() {
            // save all from next layer
            result.extend(deps.iter());

            // iterate and do their dependencies
            for dep in deps {
                result.extend(dep.graph_walk().into_iter());
            }
        }

        result
    }
}

/// Simplistic topological sort
pub fn tsort<T: Graph>(graph: Vec<&T>) -> Vec<&T> {
    let mut result: Vec<&T> = vec![];

    // add all the root graph nodes
    result.extend(graph.iter());

    // iterate over the root graph nodes
    for g in &graph {
        result.extend(g.graph_walk().into_iter());
    }

    result.reverse();

    result
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use super::*;

    #[derive(PartialEq, Eq, Hash)]
    pub struct Sortable<'a>(pub &'a str, pub Vec<&'a Self>);

    impl Graph for Sortable<'_> {
        fn graph_dependencies(&self) -> Vec<&Self> {
            self.1.clone()
        }
    }

    impl Debug for Sortable<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.1.is_empty() {
                write!(f, "{:?}", self.0)
            } else {
                write!(f, "{:?} {:?}", self.0, self.1)
            }
        }
    }

