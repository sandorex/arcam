mod test_start;
mod test_exists;
mod test_kill;
mod test_exec;
mod test_shell;
mod test_permissions;

// NOTE: This test is not useless, it prevents running tests on outdated main binary
#[test]
fn test_sanity() -> Result<(), Box<dyn std::error::Error>> {
    assert_cmd::Command::cargo_bin(env!("CARGO_BIN_NAME"))?
        .args(["--version"])
        .assert()
        .success()
        .stdout(format!("arcam {}\n", crate::FULL_VERSION));

    Ok(())
}
