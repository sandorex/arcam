mod util;
mod cli;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

use std::env;
use clap::Parser;
use std::process::Command;
// use std::path::Path;

// /// Check if running inside a container
// fn in_container() -> bool {
//     // in debug version allow ignoring if its a container or not
//     if cfg!(debug_assertions) {
//         if let Ok(val) = std::env::var("BOX_FORCE") {
//             return match val.to_lowercase().as_str() {
//                 "container" => true,
//                 "host" => false,
//                 _ => panic!("BOX_FORCE can only be 'container' or 'host'"),
//             };
//         }
//     }
//
//     return Path::new("/run/.containerenv").exists()
//         || Path::new("/.dockerenv").exists()
//         || std::env::var("container").is_ok()
// }

pub const INIT_SCRIPT: &'static str = include_str!("box-init.sh");

fn run_command(dry_run: bool, cmd: Command) -> Result<(), ()> {
    // basically either print it or run it, if dry_run it is always successful
    //
    Ok(())
}

fn start_container(engine: &str, dry_run: bool, args: &cli::CmdStartArgs) {
    /*
# data volume used for persistant things like neovim plugins for example
# NOTE using volume inspect as it works on both podman and docker
if ! command "$ENGINE" volume inspect box-data &>/dev/null; then
    command "$ENGINE" volume create box-data &>/dev/null
fi

# find all terminfo as debian has them scattered around
args=()
for i in /usr/share/terminfo /usr/lib/terminfo /etc/terminfo; do
    if [[ -d "$i" ]]; then
        args+=(--volume "$i:/host$i:ro")
    fi
done

# prefer argument dotfiles than env var
if [[ -n "$DOTFILES" ]]; then
    args+=(--volume "$DOTFILES:/etc/skel:ro")
elif [[ -n "$BOX_DOTFILES" ]]; then
    args+=(--volume "$BOX_DOTFILES:/etc/skel:ro")
fi

# network is on by default, so disable it if requested
if [[ "$NETWORK" -eq "0" ]]; then
    args+=(--network=none)
fi

# TODO print the name not the ID
# TODO docker does not support --userns=keep-id

# the bash -c mess is so it waits until init file is pushed to container cause
# i do not want to manually delete containers later if i remove '--rm' flag
CONTAINER_ID=$("$ENGINE" run -d --rm \
    --security-opt label=disable \
    --user root \
    --userns=keep-id \
    --label=manager=box \
    --label="box=$ENGINE" \
    --env "BOX=$ENGINE" \
    --env "BOX_VERSION=$VERSION" \
    --env "HOST_USER=$USER" \
    --env TERMINFO_DIRS=/host/usr/share/terminfo:/host/usr/lib/terminfo:/host/etc/terminfo:/usr/share/terminfo:/usr/lib/terminfo:/etc/terminfo \
    "${args[@]}" \
    --volume box-data:/data:Z \
    --volume "$PWD:/ws:Z" \
    --hostname "$(hostname)" \
    --entrypoint /bin/bash \
    "$@" \
    "$IMAGE" \
    -c \
    'while [[ ! -f /init ]]; do sleep 0.1s; done; echo done; exec /init'
)

# copy init
command "$ENGINE" cp box-init "$CONTAINER_ID:/init"
     */
}

fn main() {
    let args = cli::Cli::parse();

    // TODO test if the engine exists at all
    // prefer the one in argument or ENV then try to find one automatically
    let engine = {
        if let Some(chosen) = args.engine {
            chosen
        } else {
            if let Some(found) = util::find_available_engine() {
                found
            } else {
                println!("No compatible container engine found in PATH");
                "echo".to_string()
                // std::process::exit(1);
            }
        }
    };

    println!("got: {}", INIT_SCRIPT);

    use cli::CliCommands;
    match args.cmd {
        CliCommands::Start(x) => start_container(&engine, args.dry_run, &x),
        CliCommands::Shell(_) => {},
        CliCommands::Exec(_) => {},
        CliCommands::List => {},
        CliCommands::Kill(_) => {},
    }
}
