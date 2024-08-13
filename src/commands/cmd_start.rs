use crate::VERSION;
use crate::util::{self, CommandOutputExt, Engine, EngineKind};
use crate::cli;
use std::collections::HashMap;
use std::process::{Command, ExitCode};

// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
// installing is required
//
// This function is required as afaik only debian has non-standard paths for terminfo
//
fn find_terminfo(args: &mut Vec<String>) {
    let mut existing: Vec<String> = vec![];
    for x in ["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
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

/// Highly inefficient expansion of env vars in string
fn expand_env(mut string: String, environ: &HashMap<String, String>) -> String {
    if string.contains("$") {
        for (k, v) in environ.iter() {
            string = string.replace(format!("${}", k).as_str(), v);
        }
    }

    string
}

pub fn start_container(engine: Engine, dry_run: bool, mut cli_args: cli::CmdStartArgs) -> ExitCode {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let executable_path = std::env::current_exe().expect("Failed to get executable path");
    let user = util::get_user();
    let ws_dir: String = {
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
            None => return ExitCode::FAILURE,
        };

        // find the config
        let config = match configs.get(&cli_args.image[1..]) {
            Some(x) => x,
            None => {
                eprintln!("Could not find config {}", cli_args.image);

                return ExitCode::FAILURE;
            }
        };

        // take image from config
        cli_args.image = config.image.clone();

        // prefer cli network
        cli_args.network = cli_args.network.or(Some(config.network));

        // prefer cli name
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
        _ => util::generate_name(),
    };

    // allow dry-run regardless if the container exists
    if !dry_run {
        // quit pre-emptively if container already exists
        if util::get_container_status(&engine, &container_name).is_some() {
            eprintln!("Container {} already exists", &container_name);
            return ExitCode::FAILURE;
        }
    }

    let (uid, gid) = util::get_user_uid_gid();

    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(), "--rm".into(),
        "--security-opt=label=disable".into(),
        "--name".into(), container_name.clone(),
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
        "--volume".into(), format!("{}:/box:ro,nocopy", executable_path.display()),
        "--volume".into(), format!("{}:{}", &cwd.to_string_lossy(), ws_dir),
        "--hostname".into(), util::get_hostname(),
    ];

    match engine.kind {
        // TODO add docker equivalent
        EngineKind::Podman => {
            args.extend(vec![
                "--userns=keep-id".into(),

                // the default ulimit is low
                "--ulimit=host".into(),

                // TODO should i add --annotation run.oci.keep_original_groups=1
            ]);
        },
        EngineKind::Docker => unreachable!(),
    }

    // add the env vars
    for e in &cli_args.env {
        args.extend(vec!["--env".into(), e.into()]);
    }

    // add remove capabilities easily
    for c in &cli_args.capabilities {
        if let Some(stripped) = c.strip_prefix("!") {
            args.extend(vec!["--cap-drop".into(), stripped.to_string()])
        } else {
            args.extend(vec!["--cap-add".into(), c.to_string()])
        }
    }

    // find all terminfo dirs, they differ mostly on debian...
    find_terminfo(&mut args);

    // disable network if requested
    if ! cli_args.network.unwrap_or(true) {
        args.push("--network=none".into());
    }

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        args.extend(vec!["--volume".into(), format!("{}:/etc/skel:ro", dotfiles)]);
    }

    // add the extra args verbatim
    args.extend(cli_args.engine_args.clone());

    args.extend(vec![
        "--entrypoint".into(), "/box".into(),

        // the container image
        cli_args.image.clone(),

        "init".into(),
    ]);

    if dry_run {
        util::print_cmd_dry_run(&engine, args);

        ExitCode::SUCCESS
    } else {
        // do i need stdout if it fails?
        let cmd = Command::new(&engine.path)
            .args(args)
            .output()
            .expect("Could not execute engine");

        if ! cmd.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&cmd.stderr));
            return cmd.to_exitcode();
        }

        let id = String::from_utf8_lossy(&cmd.stdout);

        // print the name instead of id
        Command::new(&engine.path)
            .args(["inspect", "--format", "{{.Name}}", id.trim()])
            .status()
            .expect("Could not execute engine")
            .to_exitcode()
    }
}

