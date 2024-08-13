pub mod cli_config;

use cli_config::ConfigCommands;
use clap::{Parser, Subcommand, Args};
use crate::FULL_VERSION;

/// Sandboxed pet container manager
#[derive(Parser, Debug)]
#[command(name = "box", author, version = FULL_VERSION, about)]
pub struct Cli {
    /// Explicitly set container engine to use
    #[arg(long, env = "BOX_ENGINE")]
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
    #[arg(long, env = "BOX_CONTAINER")]
    pub name: Option<String>,

    /// Path to dotfiles which will be used as /etc/skel inside the container
    #[arg(long, env = "BOX_DOTFILES")]
    pub dotfiles: Option<String>,

    // TODO network should be --network/--no-network (https://github.com/clap-rs/clap/issues/815)
    /// Set network access permission for the container
    #[arg(long, value_name = "BOOL", default_missing_value = "true", require_equals = true, num_args = 0..=1)]
    pub network: Option<bool>,

    /// Add or drop capabilities by prefixing them with '!'
    ///
    /// For more details about capabilities read `man 7 capabilities` or box wiki
    #[arg(long = "cap")]
    pub capabilities: Vec<String>,

    /// Environment variables to set inside the container
    #[arg(short, long, value_name = "VAR=VALUE")]
    pub env: Vec<String>,

    /// Container image to use or @config
    #[arg(env = "BOX_IMAGE")]
    pub image: String,

    /// Pass rest of args to engine verbatim
    #[arg(last = true)]
    pub engine_args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdShellArgs {
    /// Name or the ID of the container
    #[arg(env = "BOX_CONTAINER")]
    pub name: String,

    // i feel like `shell --shell` looks awful so i made into a position arg
    /// Use custom shell
    pub shell: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExecArgs {
    /// Execute command using bash shell (avoids bash -c '..')
    #[arg(long)]
    pub shell: bool,

    /// Name or the ID of the container
    #[arg(value_name = "CONTAINER", env = "BOX_CONTAINER")]
    pub name: String,

    // NOTE command is required but last so that you can use name from environment
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExistsArgs {
    #[arg(env = "BOX_CONTAINER")]
    pub container: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdKillArgs {
    /// Do not ask for confirmation
    #[arg(short, long)]
    pub yes: bool,

    /// How many seconds to wait before killing the containers forcibly
    #[arg(short, long, default_value_t = 20)]
    pub timeout: u32,

    #[arg(env = "BOX_CONTAINER")]
    pub container: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdLogsArgs {
    /// Follow the logs
    #[arg(short, long)]
    pub follow: bool,

    #[arg(env = "BOX_CONTAINER")]
    pub container: String,
}

#[derive(Subcommand, Debug)]
pub enum CliCommands {
    /// Start a container in current directory, mounting it as the rw workspace
    #[command(arg_required_else_help = true)]
    Start(CmdStartArgs),

    /// Enter the shell inside a running box container
    #[command(arg_required_else_help = true)]
    Shell(CmdShellArgs),

    /// Execute a command inside a running box container
    #[command(arg_required_else_help = true)]
    Exec(CmdExecArgs),

    /// Check if container exists
    ///
    /// Exit code is 0 if container exists otherwise 1
    Exists(CmdExistsArgs),

    /// Config related commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// List running containers managed by box
    List,

    /// Show container logs in journalctl
    Logs(CmdLogsArgs),

    /// Stop running containers managed by box
    #[command(arg_required_else_help = true)]
    Kill(CmdKillArgs),

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

