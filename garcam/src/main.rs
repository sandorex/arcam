use garcam::{cli::{Cli, CliCommands}};
use anyhow::{anyhow, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = Cli::parse();
    dbg!(&args);

    Ok(())
}
