use std::process::Command;

// use env!("GIT_DESCRIBE") to access it!
// TODO does not work in packages as there is no git, so it will be empty

fn git_describe() -> Option<String> {
    // basically generate ' (v0.1.1-4-gb9461f0d8e-dirty)' if dirty and 4 commits after v0.1.1 tag
    match Command::new("git").args(["describe", "--tags", "--abbrev=10", "--dirty"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Some(format!(" ({})", stdout.trim()))
        },
        Err(_) => None,
    }
}

// basically just get commit hash if not tagged and add it as env var
fn main() {
    println!("cargo::rerun-if-changed=.git");

    let mut git_describe_env: String = " ".into();

    // try to set it properly, if it fails meh
    if let Some(x) = git_describe() {
        git_describe_env = x;
    }

    println!("cargo::rustc-env=GIT_DESCRIBE={}", git_describe_env);
}
