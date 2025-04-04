use crate::cli;
use crate::prelude::*;
use std::process::exit;

pub fn container_exists(ctx: Context, cli_args: cli::CmdExistsArgs) -> Result<()> {
    if cli_args.name.is_empty() {
        // cwd containers are always owned
        match ctx.get_cwd_containers() {
            Ok(containers) if !containers.is_empty() => exit(0),
            _ => exit(1),
        }
    } else if ctx.engine.container_exists(&cli_args.name)? {
        exit(0);
    } else {
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::Podman;
    use crate::tests_prelude::*;
    use assert_cmd::Command;

    #[test]
    #[ignore]
    fn cmd_exists_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        // no cwd containers yet
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["exists"])
            .current_dir(tempdir.path())
            .assert()
            .failure()
            .code(1);

        // create the container
        let cmd = Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "debian:trixie"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        let container = Container {
            engine: Box::new(Podman),
            container: String::from_utf8_lossy(&cmd.get_output().stdout)
                .trim()
                .to_string(),
        };

        assert!(!container.is_empty(), "Container name is empty");

        // test with explicitly set container_name
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["exists", &container])
            .assert()
            .success();

        // detect container from cwd
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["exists"])
            .current_dir(tempdir.path())
            .assert()
            .success();

        Ok(())
    }
}
