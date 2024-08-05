use std::process::Command;

// use env!("GIT_HASH") to access it!

// basically just get commit hash if not tagged and add it as env var
fn main() {
    println!("cargo::rerun-if-changed=.git");

    // basically generate 'v0.1.1-4-gb9461f0d8e-dirty' if dirty and 4 commits after v0.1.1 tag
    let git_hash = match Command::new("git").args(["describe", "--tags", "--abbrev=10", "--dirty"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            format!(" ({})", stdout.trim())
        },
        // in case built without git, which shouldnt happen right?
        // well.. cargo package does not have access to the git repo
        Err(_) => {
            println!("cargo::warning='Git not found, could not describe git'");
            " ".to_string()
        },
    };

    println!("cargo::rustc-env=GIT_DESCRIBE={}", git_hash);
}
