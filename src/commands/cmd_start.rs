use crate::{VERSION, get_user};
use super::cmd_init::DATA_VOLUME_NAME;
use crate::util;
use crate::cli;

pub fn start_container(engine: &str, dry_run: bool, cli_args: &cli::CmdStartArgs) -> u8 {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let executable_path = std::env::current_exe().expect("Failed to get executable path");

    // NOTE i am generating the name as its easier than reading output of the command, and this way
    // i am getting consistant nameing for all boxes :)
    let container_name = crate::generate_name();

    // TODO set XDG_ env vars just in case
    // TODO add env var with engine used (but only basename in case its a full path)
    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(), "--rm".into(),
        "--security-opt".into(), "label=disable".into(),
        "--name".into(), container_name.clone(),
        "--user".into(), "root".into(),
        "--userns=keep-id".into(), // TODO podman only
        "--label=manager=box".into(),
        "--label=box=box".into(),
        "--env".into(), "BOX=BOX".into(),
        "--env".into(), format!("BOX_VERSION={}", VERSION),
        "--env".into(), format!("BOX_USER={}", get_user()),
        "--volume".into(), format!("{}:/init:ro", executable_path.display()),
        "--volume".into(), format!("{}:/ws:Z", &cwd.to_string_lossy()),
        "--hostname".into(), util::get_hostname(),
    ];

    // add the env vars, TODO should this be checked for syntax?
    for e in &cli_args.env {
        args.extend(vec!["--env".into(), e.into()]);
    }

    // add remove capabilities easily
    for c in &cli_args.capabilities {
        if c.starts_with("!") {
            args.extend(vec!["--cap-drop".into(), c[1..].to_string()])
        } else {
            args.extend(vec!["--cap-add".into(), c.to_string()])
        }
    }

    // find all terminfo dirs, they differ mostly on debian...
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
        let inspect_cmd = crate::engine_cmd_output(engine, vec![
            "volume".into(), "inspect".into(), DATA_VOLUME_NAME.into(),
        ]);

        // if it fails then volume is missing probably
        if ! inspect_cmd.is_ok() {
            // TODO maybe i should run output() then print stdout/stderr if it fails?
            let create_vol_cmd = crate::engine_cmd_status(engine, dry_run, vec![
                "volume".into(), "create".into(), DATA_VOLUME_NAME.into(),
            ]);

            if ! create_vol_cmd.is_ok() {
                panic!("Failed to create data volume");
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
        // TODO add this as an option
        // "--env".into(), "RUST_BACKTRACE=1".into(),
        "--entrypoint".into(), "/init".into(),

        // the container image
        cli_args.image.clone(),

        "init".into(),
    ]);

    // TODO add interactive version where i can see output from the container, maybe podman logs -f
    // TODO add plain command back in, this is ugly
    let cmd = crate::engine_cmd_status(engine, dry_run, args);

    // if the command fails just return with the exit code
    match cmd {
        Err(x) => x,
        Ok(_) => 0,
    }

    // TODO print user friendly name
}

