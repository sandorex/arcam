use clap::{Subcommand, Args};

#[derive(Args, Debug, Clone)]
pub struct CmdImageBuildArgs {
    /// Tag to set for the image (defaults to timestamp)
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Disable caching of layers, force rebuild whole image
    #[arg(long)]
    pub no_cache: bool,

    /// Set build directory for the container (defaults to current dir)
    #[arg(short, long)]
    pub build_dir: Option<String>,

    /// Copy dotfiles inside the container as /etc/skel
    ///
    /// Note that this option just mounts the dotfiles at `/dotfiles` and the containerfile must
    /// copy them into /etc/skel
    #[arg(long)]
    pub dotfiles: Option<String>,

    /// Containerfile to use to build the image (defaults to Containerfile or Dockerfile)
    pub containerfile: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ImageCommands {
    /// Build image for box
    ///
    /// This is not strictly necessary as all images are useable with box, its more of a helper
    /// than build system
    Build(CmdImageBuildArgs),
}

