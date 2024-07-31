use std::process::Command;

// use env!("GIT_HASH") to access it!

// basically just get commit hash if not tagged and add it as env var
fn main() {
    let is_tagged: bool = match Command::new("git").args(&["tag", "--points-at", "HEAD"]).output() {
        Ok(output) => !String::from_utf8(output.stdout).unwrap().is_empty(),
        // assume not tagged even though git may not exist
        Err(_) => false,
    };

    let git_hash;
    if is_tagged {
        // nothing when its a release
        git_hash = "".to_string();
    } else {
        git_hash = match Command::new("git").args(&["rev-parse", "--short=10", "HEAD"]).output() {
            Ok(output) => format!("-{}", String::from_utf8(output.stdout).unwrap()),
            Err(_) => "-???".to_string(),
        };
    }

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
