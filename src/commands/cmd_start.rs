use crate::{APP_NAME, ENV_VAR_PREFIX, VERSION};
use crate::util::{self, rand, EngineKind};
use crate::prelude::*;
use crate::util::command_extensions::*;
use crate::cli::{CmdStartArgs, ConfigArg};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get hostname from system using `hostname` command
fn get_hostname() -> Result<String> {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        log::debug!("Getting hostname from environ");
        return Ok(env_hostname);
    }

    log::debug!("Getting hostname using hostname command");

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname")
        .output()
        .with_context(|| "Could not call hostname")?;

    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if !cmd.status.success() || hostname.is_empty() {
        return Err(anyhow!("Unable to get hostname from host"));
    }

    Ok(hostname.trim().into())
}

/// Generates random name using adjectives list
fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &str = include_str!("adjectives.txt");

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.get(util::rand() as usize % adjectives.len()).unwrap();

    // allow custom container suffix but default to bin name
    let suffix = std::env::var(crate::ENV_CONTAINER_SUFFIX)
        .unwrap_or_else(|_| APP_NAME.to_string());

    format!("{}-{}", adjective, suffix)
}

/// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
/// installing is required
///
/// This function is required as afaik only debian has non-standard paths for terminfo
fn find_terminfo() -> Vec<String> {
    let mut args: Vec<String> = vec![];

    log::debug!("Looking for terminfo directories on host system");

    let mut existing: Vec<String> = vec![];
    for x in ["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
        if std::path::Path::new(x).exists() {
            log::debug!("Found {x:?}");
            args.push(format!("--volume={0}:/host{0}:ro", x));
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
    args.push(format!("--env=TERMINFO_DIRS={}", terminfo_env));

    args
}

fn resolve_capabilities(cli_args: &CmdStartArgs, cmd: &mut Command) {
    // NOTE podman does not support drop and add at the same time, once dropped its dropped so i
    // want to extend that so config can overwrite it and then cli can overwrite the overwritten
    {
        // list of all capabilities mentioned, true => add, false => drop
        let mut caps = HashMap::<&str, bool>::new();

        for i in &cli_args.capabilities {
            match i.strip_prefix("!") {
                Some(x) => caps.insert(x, false),
                None => caps.insert(i, true),
            };
        }

        for (cap, val) in caps {
            if val {
                cmd.arg(format!("--cap-add={}", cap));
            } else {
                cmd.arg(format!("--cap-drop={}", cap));
            }
        }
    }
}

fn mount_wayland(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    // try to pass through wayland socket
    if cli_args.wayland.unwrap_or(false) {
        // prefer ARCAM_WAYLAND_DISPLAY
        if let Ok(wayland_display) = std::env::var(crate::ENV_WAYLAND_DISPLAY).or(std::env::var("WAYLAND_DISPLAY")) {
            let socket_path = format!("/run/user/{}/{}", ctx.user_id, wayland_display);
            if Path::new(&socket_path).exists() {
                log::debug!("Found wayland socket at {socket_path:?}");

                // TODO pass XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
                cmd.args([
                    format!("--volume={0}:{0}", socket_path),
                    format!("--env=WAYLAND_DISPLAY={}", wayland_display),
                ]);
            } else {
                return Err(anyhow!("Could not find the wayland socket {:?}", socket_path));
            }

            // add fonts just in case
            cmd.arg("--volume=/usr/share/fonts:/usr/share/fonts/host:ro");

            // legacy ~/.fonts
            let home_dot_fonts = ctx.user_home.join(".fonts");
            if home_dot_fonts.exists() {
                cmd.arg(format!("--volume={}:/usr/share/fonts/host_dot:ro", home_dot_fonts.to_string_lossy()));
            }

            // font dir ~/.local/share/fonts
            let home_dot_local_fonts = ctx.user_home.join(".local")
                .join("share")
                .join("fonts");

            if home_dot_local_fonts.exists() {
                cmd.arg(format!("--volume={}:/usr/share/fonts/host_local:ro", home_dot_local_fonts.to_string_lossy()));
            }
        } else {
            return Err(anyhow!("Could not pass through wayland socket as WAYLAND_DISPLAY is not defined"));
        }
    }

    Ok(())
}

fn mount_additional_mounts(ws_dir: &Path, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    for m in &cli_args.mount {
        let mount = Path::new(m);
        if mount.exists() {
            if ! mount.is_dir() {
                return Err(anyhow!("Mountpoint {:?} is not a directory", mount));
            }

            // get the absolute path
            let mount = mount.canonicalize().unwrap();

            log::debug!("Mounting additional mount {mount:?}");

            cmd.arg(format!("--volume={}:{}/{}", mount.to_string_lossy(), ws_dir.to_string_lossy(), mount.file_name().unwrap().to_string_lossy()));
        } else {
            return Err(anyhow!("Mountpoint {:?} does not exist", mount));
        }
    }

    Ok(())
}

fn mount_audio(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    // try to pass audio
    if cli_args.audio.unwrap_or(false) {
        // TODO see if passing pipewire or alsa is possible too
        let socket_path = format!("/run/user/{}/pulse/native", ctx.user_id);
        if Path::new(&socket_path).exists() {
            cmd.args([
                format!("--volume={0}:{0}", socket_path),
                format!("--env=PULSE_SERVER=unix:{}", socket_path),
            ]);

            log::debug!("Pulseaudio socket found at {socket_path:?}");
        } else {
            return Err(anyhow!("Could not find pulseaudio socket to pass to the container"));
        }
    }

    Ok(())
}

fn mount_ssh_agent(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    if cli_args.ssh_agent.unwrap_or(false) {
        if let Ok(ssh_sock) = std::env::var("SSH_AUTH_SOCK") {
            if Path::new(&ssh_sock).exists() {
                cmd.args([
                    format!("--volume={}:/run/user/{}/ssh-auth", ssh_sock, ctx.user_id),
                    format!("--env=SSH_AUTH_SOCK=/run/user/{}/ssh-auth", ctx.user_id),
                ]);

                log::debug!("ssh-agent socket found at {ssh_sock:?}");
            } else {
                return Err(anyhow!("Socket does not exist at {:?} (ssh-agent)", ssh_sock));
            }
        } else {
            return Err(anyhow!("Could not pass through ssh-agent as SSH_AUTH_SOCK is not defined"));
        }
    }

    Ok(())
}

fn mount_session_bus(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    if cli_args.session_bus.unwrap_or(false) {
        if let Ok(dbus_addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            if let Some(dbus_sock) = dbus_addr.strip_prefix("unix:path=") {
                if Path::new(&dbus_sock).exists() {
                    cmd.args([
                        format!("--volume={}:/run/user/{}/bus", dbus_sock, ctx.user_id),
                        format!("--env=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", ctx.user_id),
                    ]);

                    log::debug!("Session dbus socket found at {dbus_sock:?}");
                } else {
                    return Err(anyhow!("Socket does not exist at {:?} (session bus)", dbus_sock));
                }
            } else {
                return Err(anyhow!("Invalid format for DBUS_SESSION_BUS_ADDRESS={:?}", dbus_addr));
            }
        } else {
            return Err(anyhow!("Could not pass through session bus as DBUS_SESSION_BUS_ADDRESS is not defined"));
        }
    }

    Ok(())
}

/// Writes text to file inside the container
pub fn write_to_file(ctx: &Context, container: &str, file: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    log::trace!("Writing data to file {file:?}");

    // write to file using tee
    #[allow(clippy::zombie_processes)]
    let mut child = ctx.engine_command()
        .args(["exec", "-i", "--user", "root", container, "tee", &file.to_string_lossy()])
        .stdin(Stdio::piped()) // pipe into stdin but ignore stdout/stderr
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect(crate::ENGINE_ERR_MSG);

    let mut stdin = child.stdin.take()
        .with_context(|| anyhow!("Failed to open child stdin"))?;

    stdin.write_all(content.as_bytes())?;

    // NOTE drop is important here otherwise stdin wont close
    drop(stdin);

    let result = child.wait()?;

    if result.success() {
        Ok(())
    } else {
        Err(anyhow!("Error writing to file {:?} in container ({})", file, result.get_code()))
    }
}

pub fn start_container(ctx: Context, mut cli_args: CmdStartArgs) -> Result<()> {
    let executable_path = ctx.get_executable_path()?;

    // NOTE /ws/ prefix is used so it does not clash with home dirs like ~/.config
    //
    // this is the general workspace dir where the main project and additional mountpoints are
    // mounted to
    let ws_dir = ctx.user_home.join("ws");

    // this is the main project where app was started
    let main_project_dir: String = format!("{}/{}", ws_dir.to_string_lossy(), ctx.cwd.file_name().unwrap().to_string_lossy());

    // get containers in this cwd, i do not care if it fails
    if let Some(x) = ctx.get_cwd_container() {
        // check if any are running
        if !x.is_empty() {
            return Err(anyhow!(r#"There are containers running in current directory: {}"#, x.join(" ")));
        }
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
            },
            ConfigArg::Config(config_name) => {
                log::debug!("Loading config @{:?}", config_name);
                ctx.find_config(config_name)?
            },
        };

        if let Some(host_pre_init) = &config.host_pre_init {
            // avoid infinite loop using env var
            if std::env::var(crate::ENV_EXE_PATH).is_err() {
                use std::os::unix::process::CommandExt;

                let mut buf: String = "#!/bin/sh\n".into();
                buf += host_pre_init;

                // write to temp file
                let path = format!("/tmp/a{}{}", rand(), rand());
                std::fs::write(&path, buf)?;

                let argv0 = std::env::args().next().unwrap();

                // execute it using the shell and replace this process with it
                return Err(
                    Command::new("/bin/sh")
                        .arg(path)
                        // skipping argv0 and command 'start'
                        .args(std::env::args().skip(2))
                        // pass the path to arcam in the env var
                        .env(crate::ENV_EXE_PATH, argv0)
                        .exec()
                        .into()
                );
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
                "RAND" | "RANDOM" => Some(rand().to_string()),

                // fallback to environ
                _ => if let Ok(var) = std::env::var(input) {
                    Some(var)
                } else {
                    log::warn!("Could not expand {input:?} in config");
                    None
                },
            }
        };

        // expand vars in engine args and append to cli args
        for i in config.engine_args.iter().chain(config.get_engine_args(&ctx.engine).iter()) {
            cli_args.engine_args.push(
                shellexpand::env_with_context_no_errors(&i, context_getter).to_string()
            );
        }

        // cli skel takes priority
        if cli_args.skel.is_none() {
            if let Some(skel) = config.skel {
                cli_args.skel = Some(shellexpand::env_with_context_no_errors(&skel, context_getter).to_string());
            }
        }

        // expand env as well for some fun dynamic shennanigans
        for (k, v) in &config.env {
            let mapped = format!("{k}={v}");
            cli_args.env.push(shellexpand::env_with_context_no_errors(&mapped, context_getter).to_string());
        }

        // prefer options from cli
        cli_args.shell = cli_args.shell.or(config.shell);
        cli_args.network = cli_args.network.or(Some(config.network));
        cli_args.audio = cli_args.audio.or(Some(config.audio));
        cli_args.wayland = cli_args.wayland.or(Some(config.wayland));
        cli_args.ssh_agent = cli_args.ssh_agent.or(Some(config.ssh_agent));
        cli_args.session_bus = cli_args.session_bus.or(Some(config.session_bus));
        cli_args.ports.extend_from_slice(&config.ports);
        cli_args.capabilities.extend_from_slice(&config.capabilities);

        // get the persist paths
        persist = config.persist;
        persist_user = config.persist_user;

        // concatinate pre / post init
        on_init_pre = cli_args.on_init_pre.join("\n") + &config.on_init_pre.unwrap_or_default();
        on_init_post = cli_args.on_init_post.join("\n") + &config.on_init_post.unwrap_or_default();
    }

    log::debug!("Using image {container_image:?}");

    // allow dry-run regardless if the container exists
    if !ctx.dry_run {
        // quit pre-emptively if container already exists
        if ctx.get_container_status(&container_name).is_some() {
            return Err(anyhow!("Container {:?} already exists", container_name));
        }
    }

    // set default shell to bash if not set already
    if cli_args.shell.is_none() {
        cli_args.shell = Some("/bin/bash".into());
    }

    log::info!("Using {:?} as the shell", cli_args.shell);

    let mut cmd = Command::new(&ctx.engine.path);
    cmd.args([
        "run", "-d", "--rm",
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
        format!("--label={}={}", crate::CONTAINER_LABEL_HOST_DIR, ctx.cwd.to_string_lossy()),
        format!("--label={}={}", crate::CONTAINER_LABEL_CONTAINER_DIR, main_project_dir),
        format!("--label={}={}", crate::CONTAINER_LABEL_USER_SHELL, cli_args.shell.as_ref().unwrap()),
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
        format!("--volume={}:{}", ctx.cwd.to_string_lossy(), main_project_dir),
        format!("--volume={}:{}:ro,nocopy", executable_path.display(), crate::ARCAM_EXE),
        format!("--entrypoint={}", crate::ARCAM_EXE),
        format!("--hostname={}", get_hostname()?),
    ]);

    // engine specific args
    match ctx.engine.kind {
        EngineKind::Podman => {
            cmd.args([
                "--userns=keep-id",
                "--group-add=keep-groups",

                // the default ulimit is low
                "--ulimit=host",

                // use same timezone as host
                "--tz=local",
            ]);
        },
    }

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
        cmd.arg(format!("--mount=type=volume,source={},destination={}", vol, path));
    }

    // set network if requested
    if ! cli_args.network.unwrap_or(false) {
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

    // TODO log whole command if TRACE

    if ctx.dry_run {
        cmd.print_escaped_cmd();

        Ok(())
    } else {
        // do i need stdout if it fails?
        let output = cmd
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if !output.status.success() {
            return Err(anyhow!("Stderr from container init: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let id = String::from_utf8_lossy(&output.stdout);
        let id = id.trim();

        // check if file exists in the container, used for flag files
        let container_file_exists = |file: &str| -> Result<bool> {
            log::trace!("Testing for existance of {file:?}");

            let cmd = ctx.engine_command()
                .args(["exec", id, "test", "-f", file])
                .output()
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.get_code() {
                0 => Ok(true),
                1 => Ok(false),
                125 => Err(anyhow!("Container has exited unexpectedly (125)")),
                127 => panic!("Unknown command used during container initialization check"),

                // this really should not happen unless something breaks
                x => Err(anyhow!("Unknown error during container initialization ({})", x)),
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

            let buffer: String = format!(r#"#!/bin/sh
set -e

asroot chown "$USER:$USER" {0}
"#, persist_user_paths);

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
        ctx.engine_exec_root(id, vec!["rm", crate::FLAG_FILE_PRE_INIT])?;

        log::trace!("Waiting for container initialization");

        // wait until container finishes initialization
        while !container_file_exists(crate::FLAG_FILE_INIT)? {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        if cli_args.enter {
            log::debug!("Launching shell");

            // launch shell right away
            crate::commands::open_shell(ctx, crate::cli::CmdShellArgs {
                name: container_name,
                shell: None,
            })
        } else {
            // print container name
            println!("{}", container_name);

            Ok(())
        }
    }
}
