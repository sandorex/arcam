use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

/// Sandboxed pet container manager
#[derive(Parser, Debug)]
#[command(name = "box", author, version, about)]
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

#[derive(Args, Debug, Clone)]
pub struct CmdStartArgs {
    /// Environment variables to set inside the container
    #[arg(short, long)]
    pub env: Vec<String>,

    /// Path to dotfiles which will be used as /etc/skel inside the container
    #[arg(long, env = "BOX_DOTFILES")]
    pub dotfiles: Option<PathBuf>,

    /// Add or drop capabilities by prefixing them with '!'
    /// for more details about capabilities read `man 7 capabilities` or box wiki
    #[arg(long = "cap")]
    pub capabilities: Vec<String>,

    /// Do not mount data volume inside the container
    #[arg(long, action, env = "BOX_NO_DATA_VOLUME")]
    pub no_data_volume: bool,

    /// Disable network access for the container
    #[arg(long, action)]
    pub no_network: bool,

    // TODO make it possible to use @config to run configuration from a file
    /// Container image to use
    #[arg(env = "BOX_IMAGE")]
    pub image: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdShellArgs {
    /// Use custom shell
    #[arg(short, long)]
    pub shell: Option<String>,

    /// Name or the ID of the container
    #[arg(env = "BOX_CONTAINER")]
    pub name: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdExecArgs {
    /// Execute command using bash shell (avoids bash -c '..')
    #[arg(long)]
    pub shell: bool,

    /// Name or the ID of the container
    #[arg(env = "BOX_CONTAINER")]
    pub name: String,

    // command is required but also last so '--' can be used as name can be taken from environ
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

    /// List running containers managed by box
    // TODO see if its possible to stack the --filter podman
    List,

    /// Stop running containers managed by box
    #[command(arg_required_else_help = true)]
    Kill(CmdKillArgs),
}

