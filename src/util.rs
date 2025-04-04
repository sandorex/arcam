use crate::command_ext::command_extensions::*;

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
