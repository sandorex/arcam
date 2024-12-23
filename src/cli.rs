pub mod cli_config;

use cli_config::ConfigCommands;
use clap::{Parser, Subcommand, Args};
use crate::FULL_VERSION;

const AFTER_HELP: &str = concat!(
    "For documentation for this particular version go to following url\n",
    env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("VERGEN_GIT_SHA"), "/docs"
);

/// Sandboxed development container manager, with focus on security by default
#[derive(Parser, Debug)]
#[command(name = crate::APP_NAME, author, version = FULL_VERSION, about, after_help = AFTER_HELP)]
pub struct Cli {
    // NOTE this is hidden as only podman is supported so honestly until docker
    // is supported then its bloat to help screen
    /// Explicitly set container engine to use
    #[arg(long, hide = true)]
    pub engine: Option<String>,

    /// Just print engine commands that would've been ran, do not execute
    #[arg(long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(Args, Debug, Clone, Default)]
pub struct CmdStartArgs {
    /// Name of the new container (if not set a randomly generated name will be used)
    #[arg(long, env = crate::ENV_CONTAINER)]
    pub name: Option<String>,

    /// Path to directory which will be used as /etc/skel inside the container
    ///
    /// Used for static dotfiles that can be copied verbatim
    #[arg(long)]
    pub skel: Option<String>,

    /// Set network access permission for the container
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub network: Option<bool>,

    /// Try to pass audio into the the container, security impact is unknown
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub audio: Option<bool>,

    /// Passes wayland compositor through, pokes holes in sandbox, allows r/w access to clipboard
    ///
    /// If you want to pass through a specific wayland socket use env var ARCAM_WAYLAND_DISPLAY
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub wayland: Option<bool>,

    /// Pass through ssh-agent socket
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub ssh_agent: Option<bool>,

    /// Pass through session dbus socket, allows command execution on host!
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub session_bus: Option<bool>,

    /// Run command on init, ran before all other scripts (ran using `/bin/sh`)
    #[arg(long, value_name = "SCRIPT")]
    pub on_init_pre: Vec<String>,

    /// Run command on init, ran after all other scripts (ran using `/bin/sh`)
    #[arg(long, value_name = "SCRIPT")]
    pub on_init_post: Vec<String>,

    /// Automatically shutdown the container when there are no shells or processes running in it
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub auto_shutdown: Option<bool>,

    /// Pass through container port to host (both TCP and UDP)
    ///
    /// Not all ports are allowed with rootless podman
    #[arg(short, long = "port", value_name = "PORT[:HOST_PORT]", value_parser = parse_ports)]
    pub ports: Vec<(u32, u32)>,

    /// Add capabilities, or drop them with by prefixing `!cap`
    ///
    /// For more details about capabilities read `man 7 capabilities`
    #[arg(long = "cap", value_name = "[!]CAPABILITY")]
    pub capabilities: Vec<String>,

    /// Mount additional paths inside workspace
    #[arg(short, long, value_name = "DIRECTORY")]
    pub mount: Vec<String>,

    /// Environment variables to set inside the container
    #[arg(short, long, value_name = "VAR=VALUE")]
    pub env: Vec<String>,

    /// Container image to use or @config
    #[arg(env = crate::ENV_IMAGE, value_name = "IMAGE|@CONFIG")]
    pub image: String,

    /// Pass rest of args to engine verbatim
    #[arg(last = true)]
    pub engine_args: Vec<String>,
}

fn parse_ports(input: &str) -> Result<(u32, u32), String> {
    let parse_port = |raw: &str| -> Result<u32, String> {
        raw.parse::<u32>()
            .map_err(|_| format!("Invalid port {:?}", raw))
    };

    if let Some((left_raw, right_raw)) = input.split_once(":") {
        Ok((parse_port(left_raw)?, parse_port(right_raw)?))
    } else {
        let port = parse_port(input)?;

        // map it to itself
        Ok((port, port))
    }
}

#[derive(Args, Debug, Clone)]
pub struct CmdShellArgs {
    // NOTE: this used to be a positional argument but it prevented the command from be being used
    // when the name of container was not provided
    /// Use a specific shell
    #[arg(long)]
    pub shell: Option<String>,

    /// Name or the ID of the container
    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExecArgs {
    /// Execute command using a shell, if not value is provided `/bin/sh` is used
    ///
    /// The shell will be used like `<shell> -c '<command..>'` so it must be compatible
    #[arg(long, default_missing_value = "/bin/sh", require_equals = true, num_args = 0..=1)]
    pub shell: Option<String>,

    /// Execute command using login shell
    /// All it does is adds `-l` flag to the shell cmd, so it may not work with
    /// non-posix shells
    #[arg(long, requires = "shell")]
    pub login: bool,

    /// Name or the ID of the container
    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,

    // NOTE command is required but last so that you can use name from environment
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExistsArgs {
    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdListArgs {
    /// List containers one per line with properties delimited by a tab
    ///
    /// Meant for use with scripts as its easily parseable
    #[arg(long)]
    pub raw: bool,

    /// Only show containers started in this directory
    #[arg(long)]
    pub here: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CmdLogsArgs {
    /// Follow the logs
    #[arg(short, long)]
    pub follow: bool,

    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdKillArgs {
    /// Do not ask for confirmation
    #[arg(short, long)]
    pub yes: bool,

    /// How many seconds to wait before killing the containers forcibly
    #[arg(short, long, default_value_t = 20)]
    pub timeout: u32,

    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdInitArgs {
    // NOTE DO NOT ADD ARGUMENTS HERE
    /// BASE64 encoded BSON data
    pub args: String,
}

#[derive(Subcommand, Debug)]
pub enum CliCommands {
    /// Start a container in current directory, mounting it as the rw workspace
    Start(CmdStartArgs),

    /// Enter the shell inside a running an owned container
    Shell(CmdShellArgs),

    /// Execute a command inside a running an owned container
    Exec(CmdExecArgs),

    /// Check if container exists
    ///
    /// Exit code is 0 if container exists otherwise 1
    Exists(CmdExistsArgs),

    /// Config related commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// List running owned containers
    List(CmdListArgs),

    /// Show container logs in journalctl
    Logs(CmdLogsArgs),

    /// Stop running owned container
    Kill(CmdKillArgs),

    /// Init command used to setup the container
    #[command(hide = true)]
    Init(CmdInitArgs),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
