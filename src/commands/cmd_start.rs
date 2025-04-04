mod util;

use crate::cli::{CmdStartArgs, ConfigArg};
use crate::command_extensions::*;
use crate::prelude::*;
use crate::{APP_NAME, ENV_VAR_PREFIX, VERSION};
use std::path::PathBuf;
use util::*;

pub fn start_container(ctx: Context, mut cli_args: CmdStartArgs) -> Result<()> {
    let executable_path = ctx.get_executable_path()?;

    // NOTE /ws/ prefix is used so it does not clash with home dirs like ~/.config
    //
    // this is the general workspace dir where the main project and additional mountpoints are
    // mounted to
    let ws_dir = ctx.user_home.join("ws");

    // this is the main project where app was started
    let main_project_dir: String = format!(
        "{}/{}",
        ws_dir.to_string_lossy(),
        ctx.cwd.file_name().unwrap().to_string_lossy()
    );

    // get containers in this cwd, i do not care if it fails
    let cwd_containers = ctx.get_cwd_containers()?;
    if !cwd_containers.is_empty() {
        return Err(anyhow!(
            "There are containers running in current directory: {:?}",
            cwd_containers.join(" ")
        ));
    }

    // prefer cli name over random one
    let container_name = cli_args.name.clone().unwrap_or_else(generate_name);
    let container_image: String;
    let on_init_pre: String;
    let on_init_post: String;
    let mut persist: Vec<(String, String)> = vec![];
    let mut persist_user: Vec<(String, String)> = vec![];

    log::debug!("Container name set to {container_name:?}");

    // TODO shellexpand env expansion should error out!
    if let ConfigArg::Image(image) = &cli_args.config {
        // no config used

        container_image = image.to_string();
        on_init_pre = "".into();
        on_init_post = "".into();
    } else {
        // get config from file or by name
        let config = match &cli_args.config {
            ConfigArg::Image(_) => unreachable!(),
            ConfigArg::File(file) => {
                log::debug!("Loading config file {:?}", file);
                crate::config::ConfigFile::config_from_file(file)?
            }
            ConfigArg::Config(config_name) => {
                log::debug!("Loading config @{:?}", config_name);
                ctx.find_config(config_name)?
            }
        };

        if let Some(host_pre_init) = &config.host_pre_init {
            // avoid infinite loop using env var
            if std::env::var(crate::ENV_EXE_PATH).is_err() {
                use std::os::unix::process::CommandExt;

                let mut buf: String = "#!/bin/sh\n".into();
                buf += host_pre_init;

                // write to temp file
                let path = format!("/tmp/a{}", rand::random::<u64>());
                std::fs::write(&path, buf)?;

                let argv0 = std::env::args().next().unwrap();

                // execute it using the shell and replace this process with it
                return Err(Command::new("/bin/sh")
                    .arg(path)
                    // skipping argv0 and command 'start'
                    .args(std::env::args().skip(2))
                    // pass the path to arcam in the env var
                    .env(crate::ENV_EXE_PATH, argv0)
                    .exec()
                    .into());
            }
        }

        // use config image
        container_image = config.image.clone();

        // expand vars
        let pwd = ctx.cwd.to_string_lossy();
        let home = ctx.user_home.to_string_lossy();

        let context_getter = |input: &str| -> Option<String> {
            match input {
                "USER" => Some(ctx.user.clone()),
                "PWD" | "CWD" => Some(pwd.to_string()),
                "HOME" => Some(home.to_string()),
                "CONTAINER" | "CONTAINER_NAME" => Some(container_name.clone()),
                "RAND" | "RANDOM" => Some(rand::random::<u32>().to_string()),

                // fallback to environ
                _ => {
                    if let Ok(var) = std::env::var(input) {
                        Some(var)
                    } else {
                        log::warn!("Could not expand {input:?} in config");
                        None
                    }
                }
            }
        };

        // expand vars in engine args and append to cli args
        for i in config.engine_args.iter() {
            cli_args
                .engine_args
                .push(shellexpand::env_with_context_no_errors(&i, context_getter).to_string());
        }

        // cli skel takes priority
        if cli_args.skel.is_none() {
            if let Some(skel) = config.skel {
                cli_args.skel = Some(
                    shellexpand::env_with_context_no_errors(&skel, context_getter).to_string(),
                );
            }
        }

        // expand env as well for some fun dynamic shennanigans
        for (k, v) in &config.env {
            let mapped = format!("{k}={v}");
            cli_args
                .env
                .push(shellexpand::env_with_context_no_errors(&mapped, context_getter).to_string());
        }

        // prefer options from cli
        cli_args.shell = cli_args.shell.or(config.shell);
        cli_args.network = cli_args.network.or(Some(config.network));
        cli_args.audio = cli_args.audio.or(Some(config.audio));
        cli_args.wayland = cli_args.wayland.or(Some(config.wayland));
        cli_args.ssh_agent = cli_args.ssh_agent.or(Some(config.ssh_agent));
        cli_args.session_bus = cli_args.session_bus.or(Some(config.session_bus));
        cli_args.ports.extend_from_slice(&config.ports);
        cli_args
            .capabilities
            .extend_from_slice(&config.capabilities);

        // get the persist paths
        persist = config.persist;
        persist_user = config.persist_user;

        // concatinate pre / post init
        on_init_pre = cli_args.on_init_pre.join("\n") + &config.on_init_pre.unwrap_or_default();
        on_init_post = cli_args.on_init_post.join("\n") + &config.on_init_post.unwrap_or_default();
    }

    log::debug!("Using image {container_image:?}");

    // allow dry-run regardless if the container exists
    if !ctx.dry_run && ctx.engine.container_exists(&container_name)? {
        return Err(anyhow!(
            "Container with name {:?} already exists",
            container_name
        ));
    }

    // set default shell to bash if not set already
    if cli_args.shell.is_none() {
        cli_args.shell = Some("/bin/bash".into());
    }

    log::info!("Using {:?} as the shell", cli_args.shell);

    let mut cmd = ctx.engine.command();
    cmd.args([
        "run",
        "-d",
        "--rm",
        "--security-opt=label=disable",
        "--user=root",
        // arcam does not act as the init system anymore
        "--init",
        // detaching breaks things
        "--detach-keys=",
    ]);

    cmd.args([
        format!("--name={}", container_name),
        format!("--label=manager={}", ctx.engine),
        format!("--label={}={}", APP_NAME, VERSION),
        format!(
            "--label={}={}",
            crate::CONTAINER_LABEL_HOST_DIR,
            ctx.cwd.to_string_lossy()
        ),
        format!(
            "--label={}={}",
            crate::CONTAINER_LABEL_CONTAINER_DIR,
            main_project_dir
        ),
        format!(
            "--label={}={}",
            crate::CONTAINER_LABEL_USER_SHELL,
            cli_args.shell.as_ref().unwrap()
        ),
        format!("--env={0}={0}", APP_NAME),
        format!("--env={}={}", ENV_VAR_PREFIX!("VERSION"), VERSION),
        format!("--env=manager={}", ctx.engine),
        format!("--env=CONTAINER_ENGINE={}", ctx.engine),
        format!("--env=CONTAINER_NAME={}", container_name),
        format!("--env=HOST_USER={}", ctx.user),
        format!("--env=HOST_USER_UID={}", ctx.user_id),
        format!("--env=HOST_USER_GID={}", ctx.user_gid),
        // TODO explore all the xdg dirs and set them properly
        format!("--env=XDG_RUNTIME_DIR=/run/user/{}", ctx.user_id),
        format!(
            "--volume={}:{}",
            ctx.cwd.to_string_lossy(),
            main_project_dir
        ),
        format!(
            "--volume={}:{}:ro,nocopy",
            executable_path.display(),
            crate::ARCAM_EXE
        ),
        format!("--entrypoint={}", crate::ARCAM_EXE),
        format!("--hostname={}", get_hostname()?),
    ]);

    cmd.args([
        "--userns=keep-id",
        "--group-add=keep-groups",
        // the default ulimit is low
        "--ulimit=host",
        // use same timezone as host
        "--tz=local",
    ]);

    // add the env vars
    for e in &cli_args.env {
        cmd.arg(format!("--env={}", e));
    }

    resolve_capabilities(&cli_args, &mut cmd);

    mount_additional_mounts(ws_dir.as_path(), &cli_args, &mut cmd)?;

    {
        // find all terminfo dirs, they differ mostly on debian...
        let args = find_terminfo();
        cmd.args(args);
    }

    // add all volumes
    for (vol, path) in persist.iter().chain(persist_user.iter()) {
        // using mount here to prevent mounting paths from persist, either by accident or
        // intentionally
        cmd.arg(format!(
            "--mount=type=volume,source={},destination={}",
            vol, path
        ));
    }

    // set network if requested
    if !cli_args.network.unwrap_or(false) {
        cmd.arg("--network=none");
    }

    mount_audio(&ctx, &cli_args, &mut cmd)?;

    mount_wayland(&ctx, &cli_args, &mut cmd)?;

    mount_ssh_agent(&ctx, &cli_args, &mut cmd)?;

    mount_session_bus(&ctx, &cli_args, &mut cmd)?;

    // pass through ports
    for (container, host) in &cli_args.ports {
        // for simplicity i am passing through both udp and tcp
        cmd.arg(format!("--publish={}:{}/tcp", host, container));
        cmd.arg(format!("--publish={}:{}/udp", host, container));
    }

    // mount skel if provided
    if let Some(skel) = &cli_args.skel {
        cmd.arg(format!("--volume={}:/etc/skel:ro", skel));
    }

    // add the extra args verbatim
    cmd.args(cli_args.engine_args.clone());

    cmd.args([
        // the container image
        &container_image,
        "init",
    ]);

    // TODO check if image exists and pull it interactively

    if ctx.dry_run {
        cmd.log(log::Level::Error);

        Ok(())
    } else {
        // do i need stdout if it fails?
        let output = cmd
            .log_output(log::Level::Debug)
            .expect(crate::ENGINE_ERR_MSG);

        if !output.status.success() {
            return Err(anyhow!(
                "Stderr from container init: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let id = String::from_utf8_lossy(&output.stdout);
        let id = id.trim();

        // check if file exists in the container, used for flag files
        let container_file_exists = |file: &str| -> Result<bool> {
            log::trace!("Testing for existance of {file:?}");

            let cmd = ctx
                .engine
                .command()
                .args(["exec", id, "test", "-f", file])
                .log_output(log::Level::Debug)
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.get_code() {
                0 => Ok(true),
                1 => Ok(false),
                125 => Err(anyhow!("Container has exited unexpectedly (125)")),
                127 => panic!("Unknown command used during container initialization check"),

                // this really should not happen unless something breaks
                x => Err(anyhow!(
                    "Unknown error during container initialization ({x})"
                )),
            }
        };

        // write pre init script into the container
        if !on_init_pre.is_empty() {
            let path = PathBuf::new()
                .join(crate::INIT_D_DIR)
                .join("01_on_init_pre.sh");

            let buffer: String = "#!/bin/sh\nset -e\n".to_string() + &on_init_pre;
            write_to_file(&ctx, id, &path, &buffer)?;
        }

        if !persist_user.is_empty() {
            let path = PathBuf::new()
                .join(crate::INIT_D_DIR)
                .join("00_chown_persist.sh");

            // get each path
            let persist_user_paths = persist_user
                .iter()
                .map(|(_, x)| x.clone())
                .collect::<Vec<_>>()
                .join(" ");

            let buffer: String = format!(
                r#"#!/bin/sh
set -e

asroot chown "$USER:$USER" {0}
"#,
                persist_user_paths
            );

            write_to_file(&ctx, id, &path, &buffer)?;
        }

        // write post init script into the container
        if !on_init_post.is_empty() {
            let path = PathBuf::new()
                .join(crate::INIT_D_DIR)
                .join("99_on_init_post.sh");

            let buffer: String = "#!/bin/sh\nset -e\n".to_string() + &on_init_post;
            write_to_file(&ctx, id, &path, &buffer)?;
        }

        log::trace!("Waiting for container preinitalization");

        // wait until container finishes pre-initialization
        while !container_file_exists(crate::FLAG_FILE_PRE_INIT)? {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // remove pre-init flag to start initalization
        ctx.engine
            .exec(id, &vec!["rm", crate::FLAG_FILE_PRE_INIT])?;

        log::trace!("Waiting for container initialization");

        // wait until container finishes initialization
        while !container_file_exists(crate::FLAG_FILE_INIT)? {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        if cli_args.enter {
            log::debug!("Launching shell");

            // launch shell right away
            crate::commands::open_shell(
                ctx,
                crate::cli::CmdShellArgs {
                    name: container_name,
                    shell: None,
                },
            )
        } else {
            // print container name
            println!("{}", container_name);

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::Podman;
    use crate::tests_prelude::*;
    use assert_cmd::Command;

    #[test]
    #[ignore]
    fn cmd_start_podman() -> Result<()> {
        let tempdir = tempfile::tempdir()?;

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

        // try to start another container in same directory
        Command::cargo_bin(env!("CARGO_BIN_NAME"))?
            .args(["start", "--name", &container, "debian:trixie"])
            .current_dir(tempdir.path())
            .assert()
            .failure()
            .stderr(format!(
                "Error: There are containers running in current directory: {:?}\n",
                container
            ));

        Ok(())
    }
}
