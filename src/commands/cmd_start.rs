use crate::{ExitResult, VERSION, ENV_VAR_PREFIX, APP_NAME};
use crate::util::{self, Engine, EngineKind};
use crate::util::command_extensions::*;
use crate::cli;
use super::cmd_init::InitArgs;
use std::collections::HashMap;
use std::path::Path;

/// Get hostname from system using `hostname` command
fn get_hostname() -> String {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        return env_hostname;
    }

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname").output().expect("Could not call hostname");
    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if !cmd.status.success() || hostname.is_empty() {
        panic!("Unable to get hostname from host");
    }

    hostname.trim().into()
}

/// Generates random name using adjectives list
fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &str = include_str!("adjectives.txt");

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.get(util::rand() as usize % adjectives.len()).unwrap();

    // allow custom container suffix but default to bin name
    let suffix = std::env::var(ENV_VAR_PREFIX!("CONTAINER_SUFFIX"))
        .unwrap_or_else(|_| APP_NAME.to_string());

    format!("{}-{}", adjective, suffix)
}

// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
// installing is required
//
// This function is required as afaik only debian has non-standard paths for terminfo
//
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

pub fn start_container(engine: Engine, dry_run: bool, mut cli_args: cli::CmdStartArgs) -> ExitResult {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let user = std::env::var("USER").expect("Unable to get USER from env var");
    let executable_path = std::env::current_exe().expect("Failed to get executable path");
    let home_dir = format!("/home/{user}");

    // NOTE /ws/ prefix is used so it does not clash with home dirs like ~/.config
    //
    // this is the general workspace dir where the main project and additional mountpoints are
    // mounted to
    let ws_dir: String = format!("{home_dir}/ws");

    // this is the main project where app was started
    let main_project_dir: String = format!("{}/{}", ws_dir, &cwd.file_name().unwrap().to_string_lossy());

    let container_name: String;

    // get containers in this cwd, i do not care if it fails
    if let Some(x) = util::find_containers_by_cwd(&engine) {
        // check if any are running
        if !x.is_empty() {
            eprintln!("There are containers running in current directory:");
            for container in &x {
                eprintln!("   {container}");
            }

            return Err(1);
        }
    }

    // handle configs
    if cli_args.image.starts_with("@") {
        // return owned config so i could move values without cloning
        let config = match util::load_configs()?.remove(&cli_args.image[1..]) {
            Some(x) => x,
            None => {
                eprintln!("Could not find config {}", cli_args.image);

                return Err(1);
            }
        };

        container_name = cli_args.name
            .or_else(|| config.container_name.clone())
            .unwrap_or_else(generate_name);

        // expand vars
        let cwd = cwd.to_string_lossy();
        let environ: HashMap<&str, &str> = HashMap::from([
            ("USER", user.as_str()),
            ("PWD", &cwd),
            ("HOME", home_dir.as_str()),
            ("CONTAINER", container_name.as_str()),
        ]);

        let context_getter = |input: &str| -> Option<String> {
            // prioritize the environ map above then get actual environ vars
            environ.get(input)
                .map(|x| x.to_string())
                .or(std::env::var(input).ok())
        };

        // expand vars in engine args and append to cli args
        for i in config.engine_args.iter().chain(config.get_engine_args(&engine).iter()) {
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
        cli_args.on_init_pre.extend_from_slice(&config.on_init_pre);
        cli_args.on_init_post.extend_from_slice(&config.on_init_post);
        cli_args.capabilities.extend_from_slice(&config.capabilities);
    } else {
        container_name = cli_args.name.unwrap_or_else(generate_name);
    }

    // allow dry-run regardless if the container exists
    if !dry_run {
        // quit pre-emptively if container already exists
        if util::get_container_status(&engine, &container_name).is_some() {
            eprintln!("Container {} already exists", &container_name);
            return Err(1);
        }
    }

    let (uid, gid) = util::get_user_uid_gid();

    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "run", "-d", "--rm",
        "--security-opt=label=disable",
        "--user=root",
    ]);

    cmd.args([
        format!("--label=manager={}", engine),
        format!("--label={}={}", APP_NAME, main_project_dir),
        format!("--label=host_dir={}", cwd.to_string_lossy()),
        format!("--env={0}={0}", APP_NAME),
        format!("--name={}", container_name),
        format!("--env={}={}", ENV_VAR_PREFIX!("VERSION"), VERSION),
        format!("--env=manager={}", engine),
        format!("--env=CONTAINER_ENGINE={}", engine),
        format!("--env=CONTAINER_NAME={}", container_name),
        format!("--env=HOST_USER={}", user),
        format!("--env=HOST_USER_UID={}", uid),
        format!("--env=HOST_USER_GID={}", gid),
        // TODO explore all the xdg dirs and set them properly
        format!("--env=XDG_RUNTIME_DIR=/run/user/{}", uid),
        format!("--volume={}:/{}:ro,nocopy", executable_path.display(), env!("CARGO_BIN_NAME")),
        format!("--volume={}:{}", &cwd.to_string_lossy(), main_project_dir),
        format!("--hostname={}", get_hostname()),
    ]);

    // engine specific args
    match engine.kind {
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

    for m in &cli_args.mount {
        let mount = Path::new(m);
        if mount.exists() {
            if ! mount.is_dir() {
                eprintln!("Mountpoint {:?} is not a directory", mount);
                return Err(1);
            }

            // get the absolute path
            let mount = mount.canonicalize().unwrap();

            cmd.arg(format!("--volume={}:{}/{}", mount.to_string_lossy(), ws_dir, mount.file_name().unwrap().to_string_lossy()));
        } else {
            eprintln!("Mountpoint {:?} does not exist", mount);
            return Err(1);
        }
    }

    {
        // find all terminfo dirs, they differ mostly on debian...
        let args = find_terminfo();
        cmd.args(args);
    }

    // set network if requested
    if ! cli_args.network.unwrap_or(false) {
        cmd.arg("--network=none");
    }

    // try to pass audio
    if cli_args.audio.unwrap_or(false) {
        // TODO see if passing pipewire or alsa is possible too
        let socket_path = format!("/run/user/{}/pulse/native", uid);
        if Path::new(&socket_path).exists() {
            cmd.args([
                format!("--volume={0}:{0}", socket_path),
                format!("--env=PULSE_SERVER=unix:{}", socket_path),
            ]);
        } else {
            eprintln!("Could not find pulseaudio socket to pass to the container");
            return Err(1);
        }
    }

    // try to pass through wayland socket
    if cli_args.wayland.unwrap_or(false) {
        if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
            let socket_path = format!("/run/user/{}/{}", uid, wayland_display);
            if Path::new(&socket_path).exists() {
                // TODO pass XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
                cmd.args([
                    format!("--volume={0}:{0}", socket_path),
                    format!("--env=WAYLAND_DISPLAY={}", wayland_display),
                ]);
            } else {
                eprintln!("Could not find the wayland socket to pass to the container");
                return Err(1);
            }
        } else {
            eprintln!("Could not pass through wayland socket as WAYLAND_DISPLAY is not defined");
            return Err(1);
        }
    }

    if cli_args.ssh_agent.unwrap_or(false) {
        if let Ok(ssh_sock) = std::env::var("SSH_AUTH_SOCK") {
            if Path::new(&ssh_sock).exists() {
                cmd.args([
                    format!("--volume={}:/run/user/{}/ssh-auth", ssh_sock, uid),
                    format!("--env=SSH_AUTH_SOCK=/run/user/{}/ssh-auth", uid),
                ]);
            } else {
                eprintln!("Could not find the ssh-agent socket to pass to the container");
                return Err(1);
            }
        } else {
            println!("Could not pass through ssh-agent as SSH_AUTH_SOCK is not defined");
            return Err(1);
        }
    }

    if cli_args.session_bus.unwrap_or(false) {
        if let Ok(dbus_addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            if let Some(dbus_sock) = dbus_addr.strip_prefix("unix:path=") {
                if Path::new(&dbus_sock).exists() {
                    cmd.args([
                        format!("--volume={}:/run/user/{}/bus", dbus_sock, uid),
                        format!("--env=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", uid),
                    ]);
                } else {
                    eprintln!("Could not find the session bus socket to pass to the container");
                    return Err(1);
                }
            } else {
                eprintln!("Invalid format for DBUS_SESSION_BUS_ADDRESS={}", dbus_addr);
                return Err(1);
            }
        } else {
            println!("Could not pass through session bus as DBUS_SESSION_BUS_ADDRESS is not defined");
            return Err(1);
        }
    }

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
        };

        match init_args.encode() {
            Ok(x) => x,
            Err(err) => {
                eprintln!("Error while encoding init args: {}", err);
                return Err(1);
            },
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

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        // do i need stdout if it fails?
        let output = cmd
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if ! output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            return output.to_exitcode();
        }

        let id_raw = String::from_utf8_lossy(&output.stdout);
        let id = id_raw.trim();

        // as the initialization can take a second or two this prevents broken dotfiles with shell
        // command when you type quickly
        let is_initialized = || -> Result<bool, u8> {
            let cmd = Command::new(&engine.path)
                .arg("exec")
                .arg(id)
                .args(["sh", "-c", "test -f /initialized"]) // TODO make initialized file a const
                .output()
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.to_exitcode() {
                Ok(()) => Ok(true),
                Err(1) => Ok(false),
                Err(125) => {
                    eprintln!("Container has exited unexpectedly (125)");
                    Err(1)
                }

                // this really should not happen unless something breaks
                Err(x) => panic!("Error while checking container initialization ({})", x),
            }
        };

        // wait until container finishes initialization
        while !is_initialized()? {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        // print the name instead of id
        Command::new(&engine.path)
            .args(["inspect", "--format", "{{.Name}}", id])
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}

