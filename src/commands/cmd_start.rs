use crate::{ExitResult, VERSION};
use crate::util::{self, Engine, EngineKind};
use crate::util::command_extensions::*;
use crate::cli;
use std::collections::HashMap;
use std::path::Path;

/// Get hostname from system using `hostname` command
#[cfg(target_os = "linux")]
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
///
/// Uses system time so its not really random cause im stingy about dependencies
fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &str = include_str!("adjectives.txt");

    // NOTE: pseudo-random without crates!
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos: usize = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        .try_into()
        .unwrap();

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives.get(nanos % adjectives.len()).unwrap();

    format!("{}-box", adjective)
}

// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
// installing is required
//
// This function is required as afaik only debian has non-standard paths for terminfo
//
fn find_terminfo(args: &mut Vec<String>) {
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
}

/// Highly inefficient expansion of env vars in string
fn expand_env(mut string: String, environ: &HashMap<String, String>) -> String {
    if string.contains("$") {
        for (k, v) in environ.iter() {
            string = string.replace(format!("${}", k).as_str(), v);
        }
    }

    string
}

pub fn start_container(engine: Engine, dry_run: bool, mut cli_args: cli::CmdStartArgs) -> ExitResult {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let executable_path = std::env::current_exe().expect("Failed to get executable path");
    let user = util::get_user();
    let ws_dir: String = {
        // NOTE /ws/ prefix is used so it does not clash with home dirs like ~/.config
        let cwd_dir_name = &cwd.file_name().unwrap().to_string_lossy();
        format!("/home/{user}/ws/{cwd_dir_name}")
    };

    // handle configs
    if cli_args.image.starts_with("@") {
        // allowed to be used in the config engine_args and dotfiles
        let expand_environ: HashMap<String, String> = HashMap::from([
            ("USER".into(), user.clone()),
            ("PWD".into(), cwd.clone().to_string_lossy().to_string()),
            ("HOME".into(), format!("/home/{}", user)),
        ]);

        // load all configs
        let configs = match util::load_configs() {
            Some(x) => x,
            None => return Err(1),
        };

        // find the config
        let config = match configs.get(&cli_args.image[1..]) {
            Some(x) => x,
            None => {
                eprintln!("Could not find config {}", cli_args.image);

                return Err(1);
            }
        };

        // take image from config
        cli_args.image = config.image.clone();

        // prefer options from cli
        cli_args.network = cli_args.network.or(Some(config.network));
        cli_args.audio = cli_args.audio.or(Some(config.audio));
        cli_args.wayland = cli_args.wayland.or(Some(config.wayland));
        cli_args.name = cli_args.name.or_else(|| config.container_name.clone());

        // prefer cli dotfiles and have env vars expanded in config
        cli_args.dotfiles = cli_args.dotfiles.or_else(|| config.dotfiles.clone().map(|x| expand_env(x, &expand_environ)));

        let engine_config = config.get_engine_config(&engine);

        cli_args.capabilities.extend(config.default.capabilities.clone());
        cli_args.capabilities.extend(engine_config.capabilities.clone());

        // at the moment only engine_args have the vars expanded
        cli_args.engine_args.extend(config.default.engine_args.iter().map(|x| expand_env(x.clone(), &expand_environ)));
        cli_args.engine_args.extend(engine_config.engine_args.iter().map(|x| expand_env(x.clone(), &expand_environ)));

        cli_args.env.extend(config.default.env.clone().iter().map(|(k, v)| format!("{k}={v}")));
        cli_args.env.extend(engine_config.env.clone().iter().map(|(k, v)| format!("{k}={v}")));
    }

    // generate a name if not provided already
    let container_name = match &cli_args.name {
        Some(x) if !x.is_empty() => x.clone(),
        _ => generate_name(),
    };

    // allow dry-run regardless if the container exists
    if !dry_run {
        // quit pre-emptively if container already exists
        if util::get_container_status(&engine, &container_name).is_some() {
            eprintln!("Container {} already exists", &container_name);
            return Err(1);
        }
    }

    let (uid, gid) = util::get_user_uid_gid();

    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(), "--rm".into(),
        "--security-opt=label=disable".into(),
        format!("--name={}", container_name),
        "--user=root".into(),
        "--label=manager=box".into(),
        "--label=box=box".into(),
        format!("--label=box_ws={}", ws_dir),
        "--env=BOX=BOX".into(),
        format!("--env=BOX_VERSION={}", VERSION),
        format!("--env=BOX_ENGINE={:?}", engine.kind),
        format!("--env=BOX_USER={}", user),
        format!("--env=BOX_USER_UID={}", uid),
        format!("--env=BOX_USER_GID={}", gid),
        format!("--env=BOX_NAME={}", container_name),
        // TODO explore all the XDG dirs and set them properly
        format!("--env=XDG_RUNTIME_DIR=/run/user/{}", uid),
        format!("--volume={}:/box:ro,nocopy", executable_path.display()),
        format!("--volume={}:{}", &cwd.to_string_lossy(), ws_dir),
        format!("--hostname={}", get_hostname()),
    ];

    match engine.kind {
        // TODO add docker equivalent
        EngineKind::Podman => {
            args.extend(vec![
                "--userns=keep-id".into(),

                // the default ulimit is low
                "--ulimit=host".into(),

                // use same timezone as host
                "--tz=local".into(),
            ]);
        },
        EngineKind::Docker => unreachable!(),
    }

    // add the env vars
    for e in &cli_args.env {
        args.push(format!("--env={}", e));
    }

    // add remove capabilities easily
    for c in &cli_args.capabilities {
        if let Some(stripped) = c.strip_prefix("!") {
            args.push(format!("--cap-drop={}", stripped));
        } else {
            args.push(format!("--cap-add={}", c));
        }
    }

    // find all terminfo dirs, they differ mostly on debian...
    find_terminfo(&mut args);

    // disable network if requested
    if ! cli_args.network.unwrap_or(true) {
        args.push("--network=none".into());
    }

    // try to pass audio
    if cli_args.audio.unwrap_or(false) {
        // TODO see if passing pipewire or alsa is possible too
        let socket_path = format!("/run/user/{}/pulse/native", uid);
        if Path::new(&socket_path).exists() {
            args.extend(vec![
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
                args.extend(vec![
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

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        args.push(format!("--volume={}:/etc/skel:ro", dotfiles));
    }

    // add the extra args verbatim
    args.extend(cli_args.engine_args.clone());

    args.extend(vec![
        "--entrypoint".into(), "/box".into(),

        // the container image
        cli_args.image.clone(),

        "init".into(),
    ]);

    let mut cmd = Command::new(&engine.path);
    cmd.args(args);

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
                .args(["sh", "-c", "test -f /initialized"])
                .output()
                .expect(crate::ENGINE_ERR_MSG);

            match cmd.to_exitcode() {
                Ok(()) => true,
                Err(1) => false,
                // this really should not happen unless something breaks
                Err(x) => panic!("Error while checking container initialization ({})", x),
            }
        };

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

