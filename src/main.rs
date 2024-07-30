mod util;
mod cli;

pub use anyhow::{Error, Result, Context};

use std::env;
use clap::Parser;
use std::process::Command;
use base64::prelude::*;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const INIT_SCRIPT: &'static str = include_str!("box-init.sh");
pub const DATA_VOLUME_NAME: &'static str = "box-data";

/// Sets required constants inside the init script
fn template_init_script(user: &str) -> String {
    INIT_SCRIPT.to_string()
        .replace("@BOX_VERSION@", VERSION)
        .replace("@BOX_USER@", user)
}

fn start_container(engine: &str, cli_args: &cli::CmdStartArgs) -> Result<()> {
    let templated_script = template_init_script(env::var("USER").with_context(|| "Unable to get USER from env var")?.as_str());

    // TODO set XDG_ env vars just in case
    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(), "--rm".into(),
        "--security-opt".into(), "label=disable".into(),
        "--user".into(), "root".into(),
        "--userns=keep-id".into(), // TODO podman only
        "--label=manager=box".into(),
        format!("--label=box={}", engine),
        "--env".into(), format!("BOX={}", engine),
        "--env".into(), format!("BOX_VERSION={}", VERSION),
        "--volume".into(), format!("{}:/ws:Z", std::env::current_dir().with_context(|| "Failed to get current directory")?.display()),
        "--hostname".into(), util::get_hostname().with_context(|| "Unable to get hostname")?,
    ];

    // add the env vars, TODO should this be checked for syntax?
    for e in &cli_args.env {
        args.extend(vec!["--env".into(), e.into()]);
    }

    // find all terminfo, they differ mostly on debian...
    {
        let mut existing: Vec<String> = vec![];
        for x in vec!["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
            if std::path::Path::new(x).exists() {
                args.extend(vec!["--volume".into(), format!("{0}:/host{0}:ro", x)]);
                existing.push(x.into());
            }
        }

        let mut terminfo_env = "".to_string();

        // add first the host ones as they are preferred
        for x in &existing {
            terminfo_env.push_str(format!("/host{}:", x).as_str());
        }

        // add container ones as fallback
        for x in &existing {
            terminfo_env.push_str(format!("{}:", x).as_str());
        }

        // remove leading ':'
        if terminfo_env.chars().last().unwrap_or(' ') == ':' {
            terminfo_env.pop();
        }

        // generate the env variable to find them all
        args.extend(vec!["--env".into(), format!("TERMINFO_DIRS={}", terminfo_env)]);
    }

    // TODO change this to data_volume so its not confusing with the negation
    if ! cli_args.no_data_volume {
        let inspect_cmd = Command::new(engine)
            .args(&["volume", "inspect", DATA_VOLUME_NAME])
            .status()
            .with_context(|| "Unable run inspect volume")?;

        // if it fails then volume is missing probably
        if ! inspect_cmd.success() {
            let create_vol_cmd = Command::new(engine)
                .args(&["volume", "create", DATA_VOLUME_NAME])
                .status()
                .with_context(|| "Unable to create volume")?;

            if ! create_vol_cmd.success() {
                return Err(Error::msg(format!("Could not create data volume")));
            }
        }

        args.extend(vec![
            "--volume".into(), format!("{}:/data:Z", DATA_VOLUME_NAME),
        ]);
    }

    // disable network if requested
    // TODO make it network and negate in --network/--no-network
    if cli_args.no_network {
        args.push("--network=none".into());
    }

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        args.extend(vec!["--volume".into(), format!("{}:/etc/skel:ro", dotfiles.display())]);
    }

    args.extend(vec![
        // use bash to decode the script
        "--entrypoint".into(), "/bin/bash".into(),

        // the container image
        cli_args.image.clone(),

        "-c".into(),
        format!("printf '{}' | base64 -d > /init; exec /init", BASE64_STANDARD.encode(templated_script)),
    ]);

    let cmd = Command::new(engine)
        .args(&args)
        .output()
        .with_context(|| "Unable to spawn engine")?;

    if ! cmd.status.success() {
        return Err(Error::msg(format!("Engine command failed: {:?}", &args)));
    }

    let id = String::from_utf8_lossy(&cmd.stdout);

    // wait for a bit so the container is actually started, important!
    std::thread::sleep(std::time::Duration::from_millis(300));

    // print the name of the container
    let _ = Command::new(engine)
        .args(&["container", "inspect", &id, "--format", "{{.Name}}"])
        .status()
        .with_context(|| "Unable to spawn engine")?;

    Ok(())
}

// TODO make this so other functions become bit more readable
/// Run command in container
// fn run_in_container(engine: &str, container: &str, cmd: Vec<String>) -> Result<(std::process::ExitStatus, String, String)> {
//     let cmd = Command::new(engine)
//         .args(&[
//             "exec",
//             "-it",
//             container,
//         ])
//         .args(cmd)
//         .output()
//         .with_context(|| "cannot execute engine")?;
//
//     Ok((cmd.status, String::from_utf8_lossy(&cmd.stdout).into(), String::from_utf8_lossy(&cmd.stderr).into()))
// }

fn open_shell(engine: &str, cli_args: &cli::CmdShellArgs) -> Result<()> {
    let user = env::var("USER").with_context(|| "Unable to get USER from env var")?;

    // extract default user shell from /etc/passwd
    let default_user_shell: String = {
        let cmd_result = Command::new(engine)
            .args(&["exec", "--user", "root", "-it", &cli_args.name, "bash", "-c", format!("getent passwd '{}'", user).as_str()])
            .output()
            .with_context(|| "Could not execute engine")?;

        let stdout = String::from_utf8_lossy(&cmd_result.stdout);
        if ! cmd_result.status.success() || stdout.is_empty() {
            return Err(Error::msg(format!("Error while extracting default shell from container")));
        }

        // i do not want to rely on external tools like awk so im extracting manually
        stdout.trim().split(':').last()
            .with_context(|| "Failed to extract default shell from passwd")?
            .to_string()
    };

    let _ = Command::new(engine)
        .args(&[
            "exec", "-it",
            "--user", user.as_str(),
            "--env", format!("TERM={}", env::var("TERM").unwrap_or("xterm".into())).as_str(),
            "--workdir", "/ws",
            &cli_args.name,
            default_user_shell.as_str(), "-l",
        ])
        .status()
        .with_context(|| "Could not execute engine")?;

    Ok(())
}

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    // TODO test if the engine exists at all
    // prefer the one in argument or ENV then try to find one automatically
    let engine = {
        if let Some(chosen) = args.engine {
            chosen
        } else {
            if let Some(found) = util::find_available_engine() {
                found
            } else {
                println!("No compatible container engine found in PATH");
                // TODO temporary as im testing in a container without podman first
                "echo".to_string()
                // std::process::exit(1);
            }
        }
    };

    use cli::CliCommands;
    match args.cmd {
        CliCommands::Start(x) => start_container(&engine, &x),
        CliCommands::Shell(x) => open_shell(&engine, &x),
        // CliCommands::Exec(_) => {},
        // CliCommands::List => {},
        // CliCommands::Kill(_) => {},
        _ => Ok(()),
    }
}
