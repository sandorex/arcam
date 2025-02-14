//! Everything that has to do with devcontainer features

use super::structure::Manifest;
use crate::prelude::*;
use std::collections::HashMap;

const HEADER_ACCEPT: &str = "Accept";
const HEADER_AUTHORIZATION: &str = "Authorization";

// TODO add error context to all functions in this fiel

pub fn oci_get_token(repository: &str, namespace: &str) -> Result<String> {
    match repository {
        "ghcr.io" => ureq::get("https://ghcr.io/token")
            .query_pairs(vec![
                ("service", "ghcr.io"),
                ("scope", &format!("repository:{namespace}:pull")),
            ])
            .call()?
            .body_mut()
            .read_json::<HashMap<String, String>>()?
            .get("token")
            .cloned()
            .ok_or_else(|| anyhow!("No token returned from \"https://{repository}/token\"")),

        _ => {
            return Err(anyhow::anyhow!(
                "Unknown repository {:?} cannot get token",
                repository
            ))
        }
    }
}

pub fn oci_fetch_manifest(
    token: &str,
    repository: &str,
    namespace: &str,
    tag: &str,
) -> Result<Manifest> {
    let mut resp = ureq::get(format!(
        "https://{repository}/v2/{namespace}/manifests/{tag}"
    ))
    .header("Accept", "application/vnd.oci.image.manifest.v1+json")
    .header("Authorization", format!("Bearer {token}"))
    .call()?;

    if resp.status().is_success() {
        let text = resp.body_mut().read_to_string()?;
        Manifest::from_str(&text)
    } else {
        Err(anyhow!(
            "Could not get manifest for \"{}:{}\" from repository {:?}",
            namespace,
            tag,
            repository
        ))
    }
}

/// Pulls OCI blob and returns the response
pub fn oci_download_blob(
    token: &str,
    repository: &str,
    namespace: &str,
    digest: &str,
    media_type: &str,
    path: &str,
) -> Result<()> {
    let mut response = ureq::get(format!(
        "https://{repository}/v2/{namespace}/blobs/{digest}"
    ))
    .header("Accept", media_type)
    .header("Authorization", format!("Bearer {token}"))
    .call()?;

    // write data to file
    let mut file = std::fs::File::create(path)?;
    std::io::copy(&mut response.body_mut().as_reader(), &mut file)?;

    Ok(())
}
