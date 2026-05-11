mod util;
mod nix;

use crate::cli::{CmdStartArgs, ConfigArg};
use crate::command_extensions::*;
use crate::config::{Config, Source};
use crate::prelude::*;
use crate::{APP_NAME, ENV_VAR_PREFIX, VERSION};
use std::path::PathBuf;
use util::*;

pub fn start_container(ctx: Context, cli_args: CmdStartArgs) -> Result<()> {
    // get any running containers in this directory
    let cwd_containers = ctx.get_cwd_containers()?;
    if !cwd_containers.is_empty() {
        return Err(anyhow!(
            "There are containers running in current directory: {}",
            cwd_containers.join(" ")
        ));
    }

    let mut config = match &cli_args.config {
        ConfigArg::File(file) => {
            log::debug!("Loading config file {:?}", file);
            crate::config::ConfigFile::config_from_file(&file)?
        }

        ConfigArg::Config(config_name) => {
            log::debug!("Loading config @{:?}", config_name);
            ctx.find_config(&config_name)?
        }

        ConfigArg::Image(image) => {
            Config {
                source: Source::Image(image.clone()),
                ..Default::default()
            }
        }

        ConfigArg::Nix(file, spec) => {
            Config {
                source: Source::Nix(file.clone(), spec.clone()),
                ..Default::default()
            }
        }
    };

    let config_name = config
        .name
        .clone()
        .expect("config.name is not set after loading!");

    let config_path = config.path.clone();
    let config_dir = match &config_path {
        Some(x) => Some(x.parent().unwrap()),
        None => None
    };

    // TODO probably deprecate this and add option to run a single command with same env vars
    // call the host pre init script
    if let Some(host_pre_init) = &config.host_pre_init {
        // avoid infinite loop using env var
        if std::env::var(crate::ENV_EXE_PATH).is_err() {
            use std::os::unix::process::CommandExt;

            let mut buf: String = "#!/bin/sh\n".into();
            buf += host_pre_init;

            // write to temp file
            let path = format!("/tmp/a{}", rand::random::<u64>());
            std::fs::write(&path, buf)?;

            // execute it using the shell and replace this process with it
            return Err(Command::new("/bin/sh")
                .arg(path)
                // skipping argv0 and command 'start'
                .args(std::env::args().skip(2))
                // pass the path to arcam in the env var
                .env(crate::ENV_EXE_PATH, std::env::args().next().unwrap())
                .env(crate::ENV_CFG_DIR, config_dir.expect("config.path is not set but host_pre_init is"))
                .env(crate::ENV_CFG_NAME, config_name)
                .exec()
                .into());
        }
    }

    // use generated name if it is not set
    if config.name.is_none() {
        config.name = Some(generate_name());
    }

    let container_name = config.name.as_ref().unwrap().clone();

    let context_getter = |input: &str| -> Option<String> {
        match input {
            "USER" => Some(ctx.user.clone()),
            "PWD" | "CWD" => Some(ctx.cwd.to_string_lossy().to_string()),
            "HOME" => Some(ctx.user_home.to_string_lossy().to_string()),
            "CONTAINER" | "CONTAINER_NAME" => Some(container_name.clone()),
            "RAND" | "RANDOM" => Some(rand::random::<u32>().to_string()),
            "CONFIG_DIR" => config_dir.clone().map(|x| x.to_string_lossy().to_string()),
            "CONFIG_NAME" => Some(config_name.clone()),

            // fallback to environ
            _ => {
                if let Ok(var) = std::env::var(input) {
                    Some(var)
                } else {
                    log::warn!("Could not expand {input:?} in config ({:?})", config_path);
                    None
                }
            }
        }
    };

    // NOTE: expand vars BEFORE merging!
    config.expand_vars(context_getter)?;
    config.merge_cli(&cli_args);

    log::debug!("Container name set to {container_name:?}");
    log::debug!("Using source: {}", config.source);

    // allow dry-run regardless if the container exists
    if !ctx.dry_run && ctx.engine.container_exists(&container_name)? {
        return Err(anyhow!(
            "Container with name {:?} already exists",
            container_name
        ));
    }

    // set default shell to bash if not set already
    if config.shell.is_none() {
        config.shell = Some("/bin/bash".into());
    }

    log::info!("Using {:?} as the shell", config.shell);

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

    if config.gvisor {
        if !crate::executable_in_path("runsc") {
            return Err(anyhow!("Could not find gvisor (runsc) in path"));
        }

        // TODO do i want the --rootless flag for runsc?
        cmd.args([
            // it seems it can look for it in path
            "--runtime=runsc",
            "--runtime-flag", "ignore-cgroups",
        ]);
    }

    let executable_path = ctx.get_executable_path()?;
    let projects_dir = ctx.user_home.join(crate::WS_DIR);
    let main_project_dir = projects_dir.join(ctx.cwd.file_name().unwrap());

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
            main_project_dir.to_string_lossy()
        ),
        format!(
            "--label={}={}",
            crate::CONTAINER_LABEL_USER_SHELL,
            config.shell.as_ref().unwrap()
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
            main_project_dir.to_string_lossy()
        ),
        // mount the executable inside container but do not copy it as it is slow
        format!(
            "--volume={}:{}:ro,nocopy",
            executable_path.to_string_lossy(),
            crate::ARCAM_EXE
        ),
        format!("--entrypoint={}", crate::ARCAM_EXE), // TODO this will clash with systemd probably
                                                      // and nix
        format!("--hostname={}", get_hostname()?),
    ]);

    cmd.args([
        "--userns=keep-id",
        "--ulimit=host", // the default ulimit is low
        "--tz=local", // use same timezone as host
    ]);

    // add the env vars
    for (k, v) in &config.env {
        cmd.arg(format!("--env={k}={v}"));
    }

    resolve_capabilities(&cli_args, &mut cmd);

    mount_additional_mounts(projects_dir.as_path(), &cli_args, &mut cmd)?;

    {
        // find all terminfo dirs
        let args = find_terminfo();
        cmd.args(args);
    }

    // add all volumes
    for (vol, path) in config.persist.iter().chain(config.persist_user.iter()) {
        // using mount here to prevent mounting paths from persist, either by accident or
        // intentionally
        cmd.arg(format!(
            "--mount=type=volume,source={},destination={}",
            vol, path
        ));
    }

    // set network if requested
    if !config.network {
        cmd.arg("--network=none");
    }

    mount_audio(&ctx, &cli_args, &mut cmd)?;

    mount_wayland(&ctx, &cli_args, &mut cmd)?;

    gpu_passthrough(&ctx, &cli_args, &mut cmd)?;

    mount_ssh_agent(&ctx, &cli_args, &mut cmd)?;

    mount_session_bus(&ctx, &cli_args, &mut cmd)?;

    // pass through ports
    for (container, host) in &config.ports {
        // for simplicity i am passing through both udp and tcp
        cmd.arg(format!("--publish={}:{}/tcp", host, container));
        cmd.arg(format!("--publish={}:{}/udp", host, container));
    }

    // mount skel if provided
    if let Some(skel) = &config.skel {
        cmd.arg(format!("--volume={}:/etc/skel:ro", skel));
    }

    // add the extra args verbatim
    cmd.args(config.engine_args.clone());

    match &config.source {
        // just pass the image
        Source::Image(image) => {
            cmd.arg(image);

            // pull image interactively if it does not exist
            if !ctx.dry_run && !ctx.engine.image_exists(&image)? {
                ctx.engine.image_pull(&image, true)?;
            }
        },

        // TODO pretty sure 
        Source::Nix(path, spec) => {
            cmd.args(&[
                "--volume=/nix/store:/nix/store:ro", // everything is in the nix store
                "--systemd=always",                  // systemd is a requirement
                "--rootfs",                          // argument is the rootfs not an image
            ]);

            let output_path = if let Some(spec) = spec {
                nix::evaluate_flake(&path, &spec)?
            } else {
                nix::evaluate_file(&path)?
            };

            // set the init
            cmd.arg(output_path.join("init"));

            todo!()
        }
    };

    // run init command
    cmd.arg("init");

    if ctx.dry_run {
        cmd.log();

        Ok(())
    } else {
        // do i need stdout if it fails?
        let output = cmd.log_output().expect(crate::ENGINE_ERR_MSG);

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

            // TODO this is also engine specific, abstract it
            let cmd = ctx
                .engine
                .command()
                .args(["exec", id, "test", "-f", file])
                .log_output()
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
        if let Some(on_init_pre) = &config.on_init_pre {
            if !on_init_pre.is_empty() {
                let path = PathBuf::new()
                    .join(crate::INIT_D_DIR)
                    .join("01_on_init_pre.sh");

                let buffer: String = "#!/bin/sh\nset -e\n".to_string() + on_init_pre;
                write_to_file(&ctx, id, &path, &buffer)?;
            }
        }

        if !config.persist_user.is_empty() {
            let path = PathBuf::new()
                .join(crate::INIT_D_DIR)
                .join("00_chown_persist.sh");

            // get each path
            let persist_user_paths = config.persist_user
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
        if let Some(on_init_post) = &config.on_init_post {
            if !on_init_post.is_empty() {
                let path = PathBuf::new()
                    .join(crate::INIT_D_DIR)
                    .join("99_on_init_post.sh");

                let buffer: String = "#!/bin/sh\nset -e\n".to_string() + on_init_post;
                write_to_file(&ctx, id, &path, &buffer)?;
            }
        }

        log::trace!("Waiting for container preinitalization");

        // wait until container finishes pre-initialization
        while !container_file_exists(crate::FLAG_FILE_PRE_INIT)? {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // remove pre-init flag to start initalization
        ctx.engine.exec(id, &["rm", crate::FLAG_FILE_PRE_INIT])?;

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
                    name: config.name.map(|x| x.clone()).unwrap(),
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
