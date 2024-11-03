use crate::{APP_NAME, ENV_VAR_PREFIX, VERSION};
use crate::util::{self, EngineKind};
use crate::prelude::*;
use crate::util::command_extensions::*;
use crate::cli::CmdStartArgs;
use super::cmd_init::{InitArgs, INITALIZED_FLAG_FILE};
use std::collections::HashMap;
use std::path::Path;

/// Get hostname from system using `hostname` command
fn get_hostname() -> Result<String> {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        return Ok(env_hostname);
    }

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname")
        .output()
        .with_context(|| "Could not call hostname")?;

    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if !cmd.status.success() || hostname.is_empty() {
        panic!("Unable to get hostname from host");
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

    let mut existing: Vec<String> = vec![];
    for x in ["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
        if std::path::Path::new(x).exists() {
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

/// Execute each command with /bin/sh
fn execute_host_pre_init(ctx: &Context, commands: &Vec<String>) -> Result<()> {
    for command in commands {
        // execute each command using sh
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c");
        cmd.arg(&command);

        if ctx.dry_run {
            cmd.print_escaped_cmd();
        } else {
            cmd.run_interactive()?;
        }
    }


    Ok(())
}

/// Merge config if specified instead of image, returns config init commands if
/// any
fn merge_config(ctx: &Context, cli_args: &mut CmdStartArgs) -> Result<Option<Vec<String>>> {
    if cli_args.image.starts_with("@") {
        // return owned config so i could move values without cloning
        let config = match ctx.load_configs()?.remove(&cli_args.image[1..]) {
            Some(x) => x,
            None => return Err(anyhow!("Could not find config {}", cli_args.image)),
        };

        if cli_args.name.is_none() {
            cli_args.name = config.container_name.clone()
                .or_else(|| Some(generate_name()));
        }

        // expand vars
        let cwd = ctx.cwd.to_string_lossy();
        let pwd = ctx.cwd.to_string_lossy();
        let home = ctx.user_home.to_string_lossy();

        let environ: HashMap<&str, &str> = HashMap::from([
            ("USER", ctx.user.as_str()),
            ("PWD", &pwd),
            ("HOME", &home),
            ("CWD", &cwd),
            ("CONTAINER", cli_args.name.as_ref().unwrap()),
        ]);

        let context_getter = |input: &str| -> Option<String> {
            // prioritize the environ map above then get actual environ vars
            environ.get(input)
                .map(|x| x.to_string())
                .or(std::env::var(input).ok())
        };

        // expand vars in engine args and append to cli args
        for i in config.engine_args.iter().chain(config.get_engine_args(&ctx.engine).iter()) {
            let expanded = shellexpand::env_with_context_no_errors(&i, context_getter);
            cli_args.engine_args.push(expanded.to_string());
        }

        // cli skel takes priority
        if cli_args.skel.is_none() {
            if let Some(skel) = config.skel {
                let expanded = shellexpand::env_with_context_no_errors(&skel, context_getter);

                cli_args.skel = Some(expanded.to_string());
            }
        }

        // expand env as well for some fun dynamic shennanigans
        for (k, v) in &config.env {
            let mapped = format!("{k}={v}");
            let expanded = shellexpand::env_with_context_no_errors(&mapped, context_getter);
            cli_args.env.push(expanded.to_string());
        }

        // take image from config
        cli_args.image = config.image;

        // prefer options from cli
        cli_args.network = cli_args.network.or(Some(config.network));
        cli_args.audio = cli_args.audio.or(Some(config.audio));
        cli_args.wayland = cli_args.wayland.or(Some(config.wayland));
        cli_args.ssh_agent = cli_args.ssh_agent.or(Some(config.ssh_agent));
        cli_args.session_bus = cli_args.session_bus.or(Some(config.session_bus));
        cli_args.auto_shutdown = cli_args.auto_shutdown.or(Some(config.auto_shutdown));
        cli_args.ports.extend_from_slice(&config.ports);
        cli_args.on_init_pre.extend_from_slice(&config.on_init_pre);
        cli_args.on_init_post.extend_from_slice(&config.on_init_post);
        cli_args.capabilities.extend_from_slice(&config.capabilities);

        // return pre_init
        return Ok(Some(config.host_pre_init));
    } else {
        if cli_args.name.is_none() {
            cli_args.name = Some(generate_name());
        }
    }

    Ok(None)
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
                None => caps.insert(&i, true),
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
            return Err(anyhow!(r#"There are containers running in current directory: {:?}"#, x));
        }
    }

    // merge the config if possible
    let host_pre_init_commands = merge_config(&ctx, &mut cli_args)?;

    // get reference to the container_name which was set in merge_config
    let container_name = cli_args.name.as_ref().unwrap();

    // allow dry-run regardless if the container exists
    if !ctx.dry_run {
        // quit pre-emptively if container already exists
        if ctx.get_container_status(container_name).is_some() {
            return Err(anyhow!("Container {:?} already exists", container_name));
        }
    }

    // run host pre init commands if there are any
    if let Some(x) = host_pre_init_commands.as_ref() {
        execute_host_pre_init(&ctx, x)?;
    }

    let mut cmd = Command::new(&ctx.engine.path);
    cmd.args([
        "run", "-d", "--rm",
        "--security-opt=label=disable",
        "--user=root",
    ]);

    if !ctx.state_dir.exists() {
        std::fs::create_dir_all(&ctx.state_dir)?;
    }

    let socket_dir = ctx.socket_dir(&container_name);
    if !socket_dir.exists() {
        std::fs::create_dir_all(&socket_dir)?;
    }

    cmd.args([
        format!("--label=manager={}", ctx.engine),
        format!("--label={}={}", APP_NAME, VERSION),
        format!("--label={}={}", crate::CONTAINER_LABEL_HOST_DIR, ctx.cwd.to_string_lossy()),
        format!("--label={}={}", crate::CONTAINER_LABEL_CONTAINER_DIR, main_project_dir),
        format!("--env={0}={0}", APP_NAME),
        format!("--name={}", container_name),
        format!("--env={}={}", ENV_VAR_PREFIX!("VERSION"), VERSION),
        format!("--env=manager={}", ctx.engine),
        format!("--env=CONTAINER_ENGINE={}", ctx.engine),
        format!("--env=CONTAINER_NAME={}", container_name),
        format!("--env=HOST_USER={}", ctx.user),
        format!("--env=HOST_USER_UID={}", ctx.user_id),
        format!("--env=HOST_USER_GID={}", ctx.user_gid),
        // TODO explore all the xdg dirs and set them properly
        format!("--env=XDG_RUNTIME_DIR=/run/user/{}", ctx.user_id),
        format!("--volume={}:/{}:ro,nocopy", executable_path.display(), env!("CARGO_BIN_NAME")),
        format!("--volume={}:{}", ctx.cwd.to_string_lossy(), main_project_dir),
        format!("--volume={}:/run/socket", ctx.socket_dir(&container_name).to_string_lossy()),
        format!("--hostname={}", get_hostname()?)
    ]);

    // engine specific args
    match ctx.engine.kind {
        // TODO add docker equivalent
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
        EngineKind::Docker => unreachable!(),
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

    let encoded_init_args = {
        // pass all the args here
        let init_args = InitArgs {
            on_init_pre: cli_args.on_init_pre,
            on_init_post: cli_args.on_init_post,
            automatic_idle_shutdown: cli_args.auto_shutdown.unwrap_or(false),
        };

        match init_args.encode() {
            Ok(x) => x,
            Err(err) => return Err(anyhow!("Error while encoding init args into BSON: {}", err)),
        }
    };

    cmd.args([
        // detaching breaks things
        "--detach-keys=",

        concat!("--entrypoint=/", env!("CARGO_BIN_NAME")),

        // the container image
        &cli_args.image,

        "init",

        // pass the init args as encoded string
        &encoded_init_args,
    ]);

    if ctx.dry_run {
        let _ = cmd.print_escaped_cmd();

        Ok(())
    } else {
        // do i need stdout if it fails?
        let output = cmd
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if ! output.status.success() {
            return Err(anyhow!("Stderr from container init: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let id_raw = String::from_utf8_lossy(&output.stdout);
        let id = id_raw.trim();

        // as the initialization can take a second or two this prevents broken dotfiles with shell
        // command when you type quickly
        let is_initialized = || -> Result<bool> {
            let cmd = Command::new(&ctx.engine.path)
                .arg("exec")
                .arg(id)
                .args(["sh", "-c", format!("test -f {}", INITALIZED_FLAG_FILE).as_str()]) // TODO make initialized file a const
                .output()
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.get_code() {
                0 => Ok(true),
                1 => Ok(false),
                125 => return Err(anyhow!("Container has exited unexpectedly (125)")),

                // this really should not happen unless something breaks
                x => panic!("Unknown error during container initialization ({})", x),
            }
        };

        // wait until container finishes initialization
        while !is_initialized()? {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        // print the name instead of id
        let cmd = Command::new(&ctx.engine.path)
            .args(["inspect", "--format", "{{.Name}}", id])
            .status()
            .expect(crate::ENGINE_ERR_MSG);

        if !cmd.success() {
            Err(anyhow!("Error could not find container after initialization, has it quit? ({})", cmd.get_code()))
        } else {
            Ok(())
        }
    }
}
