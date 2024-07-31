use crate::get_user;
use crate::cli;
use std::process::Command;

/// Extracts default shell for user from /etc/passwd inside a container
fn get_user_shell(engine: &str, container: &str, user: &str) -> String {
    let cmd_result = Command::new(engine)
        .args(&["exec", "--user", "root", "-it", &container, "getent", "passwd", user])
        .output()
        .expect("Could not execute engine");

    const ERR: &'static str = "Failed to extract default shell from /etc/passwd";
    let stdout = String::from_utf8_lossy(&cmd_result.stdout);
    if ! cmd_result.status.success() || stdout.is_empty() {
        panic!("{}", ERR);
    }

    // i do not want to rely on external tools like awk so im extracting manually
    stdout.trim().split(':').last().map(|x| x.to_string())
        .expect(ERR)
}

pub fn open_shell(engine: &str, dry_run: bool, cli_args: &cli::CmdShellArgs) -> u8 {
    let user = get_user();
    let user_shell = get_user_shell(engine, &cli_args.name, user.as_str());

    let cmd = crate::engine_cmd_status(engine, dry_run, vec![
        "exec".into(), "-it".into(),
        "--user".into(), user,
        // propagete TERM but default to xterm
        "--env".into(), format!("TERM={}", std::env::var("TERM").unwrap_or("xterm".into())),
        "--workdir".into(), "/ws".into(),
        user_shell, "-l".into(),
    ]);

    // propagate the exit code even though it does not matter much
    match cmd {
        Ok(_) => 0,
        Err(x) => x,
    }
}

