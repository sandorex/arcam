use crate::prelude::*;

/// How old file has to be for the process to be considered dead, in seconds
const OLD_THRESHOLD: u64 = 30;

pub fn container_healthcheck() -> Result<()> {
    for f in std::fs::read_dir(crate::HEALTH_DIR)?.flatten() {
        let file_lifetime = std::time::SystemTime::now().duration_since(f.metadata()?.modified()?)?.as_secs();
        if file_lifetime > OLD_THRESHOLD {
            // found a detached process so delete and continue
            std::fs::remove_file(f.path())?;
            continue;
        } else {
            // found a running process so just quit
            println!("There are attached root processes found");
            return Ok(());
        }
    }

    Err(anyhow!("No attached root processes found"))
}
