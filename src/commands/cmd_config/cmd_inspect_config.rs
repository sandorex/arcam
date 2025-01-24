use std::path::Path;
use crate::config::ConfigFile;
use crate::cli::cli_config::CmdConfigInspectArgs;
use crate::prelude::*;

// TODO also try to find it as config name instead of literal path if prefixed @config
pub fn inspect_config(ctx: Context, cli_args: CmdConfigInspectArgs) -> Result<()> {
    // if let Some(config_name) = cli_args.config.strip_prefix('@') {
    //     if config_name.trim().is_empty() {
    //         return Err(anyhow!("Config cannot be unnamed"));
    //     }
    //
    //     let config = match ctx.load_configs()?.remove(config_name.trim()) {
    //         Some(x) => x,
    //         None => return Err(anyhow!("Could not find config {:?}", config_name)),
    //     };
    // } else if std::fs::exists(cli_args.config) {
    //     // file
    // } else {
    //     // use as image
    // }

    // println!("{:#?}", ConfigFile::load_from_file(Path::new(&cli_args.path))?);

    Ok(())
}
