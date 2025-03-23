use std::path::Path;
use crate::command_ext::command_extensions::*;

/// Clones git repository using external git, tag is either a branch or a tag
pub fn git_clone(path: &Path, repository: &str, tag: Option<&str>) -> anyhow::Result<()> {
    let mut command = Command::new("git");

    command.args(["clone", "--depth", "1"]);

    if let Some(tag) = tag {
        command.args(["--branch", tag]);
    }

    command.args(["--", repository]);
    command.arg(path);

    // allows easy debugging by printing stdout
    if log::log_enabled!(log::Level::Debug) {
        command.log_status_anyhow(log::Level::Debug)?;
    } else {
        command.log_output_anyhow(log::Level::Debug)?;
    }

    Ok(())
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

pub trait Graph: PartialEq {
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

    // removing duplicates, expensive
    let mut unique: Vec<&T> = vec![];
    for i in &result {
        if !unique.contains(i) {
            unique.push(i);
        }
    }

    unique
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use super::*;

    #[derive(PartialEq)]
    pub struct Sortable<'a>(pub &'a str, pub Vec<&'a Self>);

    impl Graph for Sortable<'_> {
        fn graph_dependencies(&self) -> Vec<&Self> {
            self.1.clone()
        }
    }

    impl Debug for Sortable<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}({})", self.0, self.1.len())
        }
    }

    #[test]
    fn tsort_basic() {
        // NOTE: im using output from `cargo graph` for this test

        let syn = Sortable("syn", vec![]);
        let quote = Sortable("quote", vec![]);
        let proc_macro2 = Sortable("proc_macro2", vec![]);
        let serde_derive = Sortable("serde_derive", vec![&proc_macro2, &quote, &syn]);
        let serde = Sortable("serde", vec![&serde_derive]);

        let ryu = Sortable("ryu", vec![]);
        let memchr = Sortable("memchr", vec![]);
        let itoa = Sortable("itoa", vec![]);
        let serde_json = Sortable("serde_json", vec![&itoa, &memchr, &ryu, &serde]);

        let serde_spanned = Sortable("serde_spanned", vec![&serde]);
        let winnow = Sortable("winnow", vec![]);
        let hashbrown = Sortable("hashbrown", vec![]);
        let equivalent = Sortable("equivalent", vec![]);
        let indexmap = Sortable("indexmap", vec![&equivalent, &hashbrown]);
        let toml_datetime = Sortable("toml_datetime", vec![&serde]);
        let toml_edit = Sortable("toml_edit", vec![&indexmap, &serde, &serde_spanned, &toml_datetime, &winnow]);
        let toml = Sortable("toml", vec![&serde, &serde_spanned, &toml_datetime, &toml_edit]);

        let sorted: Vec<&Sortable> = tsort(vec![&serde, &serde_json, &toml]);

        assert_eq!(
            sorted,
            vec![&syn, &quote, &proc_macro2, &serde_derive, &serde, &hashbrown, &equivalent, &winnow, &toml_datetime, &serde_spanned, &indexmap, &toml_edit, &ryu, &memchr, &itoa, &toml, &serde_json,]
        );

        // basic example
        let d = Sortable("D", vec![]);
        let c = Sortable("C", vec![&d]);
        let a = Sortable("A", vec![&c, &d]);
        let e = Sortable("E", vec![&c, &a]);
        let b = Sortable("B", vec![&e, &d]);

        let sorted: Vec<&Sortable> = tsort(vec![&d, &c, &a, &e, &b]);

        assert_eq!(
            sorted,
            vec![&d, &c, &a, &e, &b],
        );
    }
}

