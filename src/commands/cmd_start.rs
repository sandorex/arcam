use crate::{get_user, VERSION, DATA_VOLUME_NAME, template_init_script};
use crate::util;
use crate::cli;
use base64::prelude::*;

pub fn start_container(engine: &str, dry_run: bool, cli_args: &cli::CmdStartArgs) -> u8 {
    let templated_script = template_init_script(get_user().as_str());
    let cwd = std::env::current_dir().expect("Failed to get current directory");

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

    // TODO remove base64 its getting too big, just start the container then use exec to pipe the
    // file into the container echo 'blah' | podman .. exec -it 'tee /init' ?
    args.extend(vec![
        // use bash to decode the script
        "--entrypoint".into(), "/bin/bash".into(),

        // the container image
        cli_args.image.clone(),

        "-c".into(),
        // encoding the init script as base64 to pass it as the entrypoint, maybe not the most
        // elegant but certainly faster than copying whole >1mb binary for few lines of bash
        format!("printf '%s' '{}' | base64 -d > /init; exec /init", BASE64_STANDARD.encode(templated_script)),
    ]);

    let cmd = crate::engine_cmd_status(engine, dry_run, args);

    // propagate the exit code
    match cmd {
        Ok(_) => 0,
        Err(x) => x,
    }
}

