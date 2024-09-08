use crate::config::Config;
use crate::{ExitResult, VERSION, ENV_VAR_PREFIX, BIN_NAME};
use crate::util::{self, Engine, EngineKind};
use crate::util::command_extensions::*;
use crate::cli;
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

    if ! cmd.status.success() || hostname.is_empty() {
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
        .unwrap_or_else(|_| BIN_NAME.to_string());

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

/// Highly inefficient expansion of env vars in string
fn expand_env(input: &mut String, environ: &HashMap<&str, &str>) {
    if input.contains("$") {
        let mut result = input.to_string();
        for (k, v) in environ.iter() {
            result = result.replace(format!("${}", k).as_str(), v);
        }
    }
}

fn merge_config(engine: &Engine, mut config: Config, cli_args: &mut cli::CmdStartArgs, environ: &HashMap<&str, &str>) {
    // expand config properties only
    if let Some(dotfiles) = config.dotfiles.as_mut() {
        expand_env(dotfiles, environ);
    }

    for i in config.default.engine_args.iter_mut() {
        expand_env(i, environ);
    }

    for (_, i) in config.default.env.iter_mut() {
        expand_env(i, environ);
    }

    // get the engine specific config
    let mut engine_config = config.get_engine_config(engine).clone();

    for i in engine_config.engine_args.iter_mut() {
        expand_env(i, environ);
    }

    for (_, i) in engine_config.env.iter_mut() {
        expand_env(i, environ);
    }

    // take image from config
    cli_args.image = config.image;

    // prefer options from cli
    cli_args.network = cli_args.network.or(Some(config.network));
    cli_args.audio = cli_args.audio.or(Some(config.audio));
    cli_args.wayland = cli_args.wayland.or(Some(config.wayland));
    cli_args.ssh_agent = cli_args.ssh_agent.or(Some(config.ssh_agent));
    cli_args.session_bus = cli_args.session_bus.or(Some(config.session_bus));
    cli_args.on_init.extend_from_slice(&config.on_init);
    cli_args.on_init_file.extend_from_slice(&config.on_init_file);

    // prefer cli dotfiles and have env vars expanded in config
    if cli_args.dotfiles.is_none() {
        cli_args.dotfiles = config.dotfiles;
    }

    cli_args.capabilities.extend_from_slice(&config.default.capabilities);
    cli_args.engine_args.extend(config.default.engine_args);
    cli_args.env.extend(config.default.env.clone().iter().map(|(k, v)| format!("{k}={v}")));

    cli_args.capabilities.extend_from_slice(&engine_config.capabilities);
    cli_args.engine_args.extend(engine_config.engine_args);
    cli_args.env.extend(engine_config.env.clone().iter().map(|(k, v)| format!("{k}={v}")));
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
            .clone()
            .or_else(|| config.container_name.clone())
            .unwrap_or_else(generate_name);

        // allowed to be used in the config engine_args and dotfiles
        let cwd = cwd.to_string_lossy();
        let environ: HashMap<&str, &str> = HashMap::from([
            ("USER", user.as_str()),
            ("PWD", &cwd),
            ("HOME", home_dir.as_str()),
            ("CONTAINER", container_name.as_str()),
        ]);

        merge_config(&engine, config, &mut cli_args, &environ);
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
        // TODO add display for engine so that its prints lowercase
        format!("--label=manager={:?}", engine.kind),
        format!("--label={}={}", BIN_NAME, main_project_dir),
        format!("--label=host_dir={}", cwd.to_string_lossy()),
        format!("--env={0}={0}", BIN_NAME),
        format!("--name={}", container_name),
        format!("--env={}={}", ENV_VAR_PREFIX!("VERSION"), VERSION),
        format!("--env=manager={:?}", engine.kind),
        format!("--env=CONTAINER_ENGINE={:?}", engine.kind),
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

    // add remove capabilities easily
    for c in &cli_args.capabilities {
        if let Some(stripped) = c.strip_prefix("!") {
            cmd.arg(format!("--cap-drop={}", stripped));
        } else {
            cmd.arg(format!("--cap-add={}", c));
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

    // disable network if requested
    if ! cli_args.network.unwrap_or(true) {
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

    for file_path in cli_args.on_init_file {
        let file = Path::new(&file_path);

        if !file.exists() {
            eprintln!("Could not find file {:?}", file_path);
            return Err(1);
        }

        cmd.arg(format!("--volume={}:/init.d/99_{}:copy", file.canonicalize().unwrap().to_string_lossy(), util::rand()));
    }

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        cmd.arg(format!("--volume={}:/etc/skel:ro", dotfiles));
    }

    // add the extra args verbatim
    cmd.args(cli_args.engine_args.clone());

    cmd.args([
        concat!("--entrypoint=/", env!("CARGO_BIN_NAME")),

        // the container image
        &cli_args.image,

        "init",

        // rest after this are on_init scripts
        "--",
    ]);

    // add on_init commands to init
    cmd.args(cli_args.on_init);

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
        let is_initialized = || -> bool {
            let cmd = Command::new(&engine.path)
                .arg("exec")
                .arg(id)
                .args(["sh", "-c", "test -f /initialized"]) // TODO make initialized file a const
                .output()
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.to_exitcode() {
                Ok(()) => true,
                Err(1) => false,
                // this really should not happen unless something breaks
                Err(x) => panic!("Error while checking container initialization ({})", x),
            }
        };

        // wait for container to be initialized
        let mut counter = 0;
        loop {
            if is_initialized() {
                break;
            }

            counter += 1;

            // basically try for 15 seconds
            if counter > 15 {
                eprintln!("Container initialization timeout was reached, killing the container");

                // kill the container
                return Command::new(&engine.path)
                    .args(["container", "stop", "--time", "5", id])
                    .status()
                    .expect(crate::ENGINE_ERR_MSG)
                    .to_exitcode();
            }

            // sleep for 100ms
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // print the name instead of id
        Command::new(&engine.path)
            .args(["inspect", "--format", "{{.Name}}", id])
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}

