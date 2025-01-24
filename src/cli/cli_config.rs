use clap::{Subcommand, Args};

#[derive(Args, Debug, Clone)]
pub struct CmdConfigExtractArgs {
    /// Container image to use
    pub image: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdConfigInspectArgs {
    /// Path to file, name of image or @config to inspect
    #[clap(value_name = "FILE|IMAGE|@CONFIG")]
    pub config: String,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Extract container config from image
    Extract(CmdConfigExtractArgs),

    /// Inspect a config, can be used to check if syntax is correct
    Inspect(CmdConfigInspectArgs),

    /// Show all options useable in a config
    Options,
}

