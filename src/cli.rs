use crate::{config::Config, Context, FULL_VERSION};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

const AFTER_HELP: &str = concat!(
    "For help visit the git repository\n   ",
    env!("CARGO_PKG_REPOSITORY")
);

/// Sandboxed development container manager, with focus on security by default
#[derive(Parser, Debug, Clone)]
#[command(name = crate::APP_NAME, author, version = FULL_VERSION, about, after_help = AFTER_HELP)]
pub struct Cli {
    /// Just print engine commands that would've been ran, do not execute
    #[arg(long)]
    pub dry_run: bool,

    /// Increase verbosity
    #[arg(short, long, value_name = "Error|Warn|Info|Debug|Trace", default_value_t = log::Level::Warn, env = crate::ENV_LOG_LEVEL)]
    pub log_level: log::Level,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ConfigArg {
    /// Use configuration from this file
    File(PathBuf),

    /// Use no configuration, plain container using this image
    Image(String),

    /// Use following config
    Config(String),
}

impl ConfigArg {
    /// Convert ContainerConfig into config
    pub fn into_config(self, ctx: &Context) -> anyhow::Result<Config> {
        use crate::config::ConfigFile;

        match self {
            Self::File(x) => ConfigFile::config_from_file(&x),
            // basically an empty config with only image set
            Self::Image(x) => Ok(Config {
                image: x.clone(),
                ..Default::default()
            }),
            Self::Config(x) => ctx.find_config(&x),
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        if input.starts_with("./")          // ex. ./local/file.toml
            || input.starts_with(".")       // ex. .arcam.toml
            || input.starts_with("/")       // ex. /etc/arcam/configs/something.toml
            || input.starts_with("~/")      // ex. ~/.config/arcam/configs/something.toml
            || input.ends_with(".toml")
        {
            // well no image is gonna end with .toml? right?
            // it must be a path
            Ok(Self::File(PathBuf::from(input)))
        } else if let Some(config_name) = input.strip_prefix("@") {
            // @ is prefix for a config
            Ok(Self::Config(config_name.to_string()))
        } else {
            // assume an image
            Ok(Self::Image(input.to_string()))
        }
    }
}

const START_HEADING_PERMISSIONS: &str = "Permissions";

#[allow(dead_code)]
const START_HEADING_EXPERIMENTAL: &str = "EXPERIMENTAL";

#[derive(Args, Debug, Clone)]
pub struct CmdStartArgs {
    /// Enter shell after container initialization finishes
    ///
    /// Ignored if stdout is not a terminal (ex. a pipe)
    #[arg(short = 'E', long, env = crate::ENV_ENTER_ON_START)]
    pub enter: bool,

    /// Name of the new container (if not set a randomly generated name will be used)
    #[arg(long, env = crate::ENV_CONTAINER)]
    pub name: Option<String>,

    /// Set container default shell
    #[arg(long)]
    pub shell: Option<String>,

    /// Path to directory which will be used as /etc/skel inside the container
    ///
    /// Used for static dotfiles that can be copied verbatim
    #[arg(long, value_name = "DIR")]
    pub skel: Option<String>,

    /// Run command on init, ran before all other scripts (ran using `/bin/sh`)
    #[arg(long, value_name = "COMMAND")]
    pub on_init_pre: Vec<String>,

    /// Run command on init, ran after all other scripts (ran using `/bin/sh`)
    #[arg(long, value_name = "COMMAND")]
    pub on_init_post: Vec<String>,

    /// Mount additional paths inside workspace
    #[arg(short, long, value_name = "DIRECTORY")]
    pub mount: Vec<String>,

    /// Environment variables to set inside the container
    #[arg(short, long, value_name = "VAR=VALUE")]
    pub env: Vec<String>,

    /// Set network access permission for the container
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1, help_heading = START_HEADING_PERMISSIONS)]
    pub network: Option<bool>,

    /// Try to pass audio into the the container, security impact is unknown
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1, help_heading = START_HEADING_PERMISSIONS)]
    pub audio: Option<bool>,

    /// Passes wayland compositor through, pokes holes in sandbox, allows r/w access to clipboard
    ///
    /// If you want to pass through a specific wayland socket use env var ARCAM_WAYLAND_DISPLAY
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1, help_heading = START_HEADING_PERMISSIONS)]
    pub wayland: Option<bool>,

    /// Pass through ssh-agent socket
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1, help_heading = START_HEADING_PERMISSIONS)]
    pub ssh_agent: Option<bool>,

    /// Pass through session dbus socket, allows command execution on host!
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1, help_heading = START_HEADING_PERMISSIONS)]
    pub session_bus: Option<bool>,

    /// Pass through container port to host (both TCP and UDP)
    ///
    /// Not all ports are allowed with rootless podman
    #[arg(short, long = "port", value_name = "PORT[:HOST_PORT]", value_parser = parse_ports, help_heading = START_HEADING_PERMISSIONS)]
    pub ports: Vec<(u32, u32)>,

    /// Add capabilities, or drop them with by prefixing `!cap`
    ///
    /// For more details about capabilities read `man 7 capabilities`
    #[arg(long = "cap", value_name = "[!]CAPABILITY", help_heading = START_HEADING_PERMISSIONS)]
    pub capabilities: Vec<String>,

    /// File, image or config to use to start a container
    #[arg(env = crate::ENV_IMAGE, value_parser = ConfigArg::parse, value_name = "FILE|IMAGE|@CONFIG")]
    pub config: ConfigArg,

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
    /// Use a specific shell
    #[arg(long)]
    pub shell: Option<String>,

    /// Name or the ID of the container
    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExecArgs {
    /// Execute the command in a shell, if value is not specified then defaults to /bin/sh
    #[arg(long, default_missing_value = "/bin/sh", require_equals = true, num_args = 0..=1)]
    pub shell: Option<String>,

    /// Execute command in a login shell
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
pub struct CmdConfigArgs {
    /// Show all options for a config
    #[clap(short, long, exclusive = true)]
    pub options: bool,

    /// Show example config
    #[clap(short, long, exclusive = true)]
    pub example: bool,

    /// Path to file, name of image or @config to inspect
    #[clap(value_parser = ConfigArg::parse, value_name = "FILE|IMAGE|@CONFIG", required_unless_present_any(["options", "example"]))]
    pub config: Option<ConfigArg>,
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

    /// Delay before container forcibly terminated (in seconds)
    #[arg(short, long, default_value_t = 10)]
    pub timeout: u32,

    #[arg(value_name = "CONTAINER", default_value = "", env = crate::ENV_CONTAINER)]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdCompletionArgs {
    /// Explicitly generate completions for specific shell
    #[arg(long, value_enum, exclusive = true)]
    pub shell: Option<clap_complete::Shell>,

    /// Used to provide better completion
    #[arg(hide = true)]
    pub complete: Option<crate::commands::ShellCompletionType>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// Start a container in current directory, mounting it read-write
    Start(CmdStartArgs),

    /// Enter the shell inside a running container
    #[clap(visible_alias = "enter")]
    Shell(CmdShellArgs),

    /// Execute a command inside a running container as the user
    Exec(CmdExecArgs),

    /// Check if container exists
    ///
    /// Exit code is 0 if container is running otherwise 1
    Exists(CmdExistsArgs),

    /// Inspect a config, can be used as syntax check
    Config(CmdConfigArgs),

    /// List running containers
    #[clap(visible_alias = "ls")]
    List(CmdListArgs),

    /// Show container logs in journalctl
    Logs(CmdLogsArgs),

    /// Stop running container
    #[clap(visible_alias = "stop")]
    Kill(CmdKillArgs),

    /// Shell autocompletion
    Completion(CmdCompletionArgs),

    /// Init command used to setup the container
    #[command(hide = true)]
    Init,
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
