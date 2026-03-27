use crate::common::prelude::*;
use anyhow::Result;
use assert_cmd::Command;
use arcam::engine::Podman;

fn start_podman(gvisor: bool) -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let mut cmd = Command::cargo_bin("arcam")?;
    cmd.current_dir(tempdir.path());
    cmd.args(["start"]);

    if gvisor {
        cmd.arg("--gvisor");
    }

    cmd.arg(DEBIAN_IMAGE);

    let cmd = cmd
        .assert()
        .success();

    let container = Container {
        engine: Box::new(Podman),
        container: String::from_utf8_lossy(&cmd.get_output().stdout)
            .trim()
            .to_string(),
    };

    // make sure the returned container name is proper, to prevent issues
    // like with interactive downloading of images
    let re = regex::Regex::new(r"[A-Za-z0-9-]").unwrap();
    assert!(
        re.is_match(&container),
        "Container name from start command is invalid"
    );

    // try to start another container in same directory
    Command::cargo_bin("arcam")?
        .args(["start", "--name", &container, DEBIAN_IMAGE])
        .current_dir(tempdir.path())
        .assert()
        .failure()
        .stderr(format!(
            "Error: There are containers running in current directory: {:?}\n",
            container
        ));

    Ok(())
}

#[test]
#[ignore = "requires podman"]
fn cmd_start_podman() -> Result<()> {
    start_podman(false)
}

#[test]
#[ignore = "requires podman and gvisor"]
fn cmd_start_podman_gvisor() -> Result<()> {
    start_podman(true)
}
