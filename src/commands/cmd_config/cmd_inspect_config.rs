use std::path::Path;

use crate::config::ConfigFile;
use crate::cli::cli_config::CmdConfigInspectArgs;
use crate::prelude::*;

// TODO also try to find it as config name instead of literal path if prefixed @config
pub fn inspect_config(cli_args: CmdConfigInspectArgs) -> Result<()> {
    println!("{:#?}", ConfigFile::load_from_file(Path::new(&cli_args.path))?);

    Ok(())
}
