//! Contains code for feature experimental command

use crate::cli::CmdFeatureArgs;
use crate::prelude::*;
use crate::util;
use crate::features::Feature;
use tempfile::TempDir;
use std::rc::Rc;
use crate::command_extensions::*;

pub fn cmd_feature(cli_args: CmdFeatureArgs) -> Result<()> {
    if !util::is_in_container() {
        return Err(anyhow!("Running features outside a container is dangerous, qutting.."));
    }

    let temp_dir = Rc::new(TempDir::new()?);

    log::debug!("Caching features in {:?}", temp_dir.path());

    println!("Fetching {} features", cli_args.feature.len());

    let mut features: Vec<Feature> = vec![];

    for i in cli_args.feature {
        features.push(Feature::cache_feature(i, temp_dir.clone())?);
    }

    for i in &features {
        println!("Executing feature \"{}\"", i.feature_path);

        i.command().log_status_anyhow(log::Level::Debug)?;
    }

    Ok(())
}
