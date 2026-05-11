//! Contains nix-specific utilities

// TODO use nix crate nix-bindings-rust in the future!

use crate::command_extensions::*;
use crate::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parses nix build json output and returns out path
fn parse_out_path(input: &str) -> Result<PathBuf> {
    #[derive(serde::Deserialize)]
    struct NixBuildOutput {
        outputs: HashMap<String, String>,
    }

    let output = serde_json::from_str::<Vec<NixBuildOutput>>(input)
        .with_context(|| anyhow!("Could not parse JSON output from nix"))?;

    let out_path = output
        .first()
        .unwrap() // TODO
        .outputs
        .get("out") // i only want the output path
        .unwrap(); // TODO

    Ok(PathBuf::from(out_path))
}

pub fn evaluate_file(path: &Path) -> Result<PathBuf> {
    if !path.try_exists().unwrap_or(false) {
        return Err(anyhow!("File {path:?} does not exist"));
    }

    // turns out nix build supports non-flake evaluation
    let cmd = Command::new("nix")
        .args(&["build", "--no-link", "--file"])
        .arg(path)
        .log_output()?;

    if !cmd.status.success() {
        return Err(anyhow!("nix-build exited with code {}", cmd.get_code()))
    }

    match String::from_utf8(cmd.stdout)?.lines().last() {
        Some(x) => Ok(PathBuf::from(x)),
        None => Err(anyhow!("nix-build returned nothing")),
    }
}

pub fn evaluate_flake(path: &Path, spec: &str) -> Result<PathBuf> {
    if !path.join("flake.nix").try_exists().unwrap_or(false) {
        return Err(anyhow!("Flake not found in {path:?}"));
    }

    let cmd = Command::new("nix")
        .args(&["build", "--no-link"])
        .arg(format!("{}#{spec}", path.display()))
        .log_output()?;

    if !cmd.status.success() {
        return Err(anyhow!("nix build exited with code {}", cmd.get_code()))
    }

    let stdout = String::from_utf8(cmd.stdout)?;

    parse_out_path(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_build_parsing() {
        let raw = r#"[{"drvPath":"/nix/store/kikbarkbjmnmsm9r4mq21k8l9j8ivc8k-test-shell-script.drv","outputs":{"out":"/nix/store/92pmhqg2kgg5j91lpb0pwigkywbhdfqa-test-shell-script"}}]"#;

        assert_eq!(
            parse_out_path(raw).unwrap(),
            PathBuf::from("/nix/store/92pmhqg2kgg5j91lpb0pwigkywbhdfqa-test-shell-script")
        );
    }
}
