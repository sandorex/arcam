//! Everything that has to do with devcontainer features

use ureq::{http::{header::{ACCEPT, AUTHORIZATION}, Response}, Body};
use super::structure::Manifest;
use crate::prelude::*;
use std::collections::HashMap;

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
    .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
    .header(AUTHORIZATION, format!("Bearer {token}"))
    .call()?;

    if resp.status().is_success() {
        let text = resp.body_mut().read_to_string()?;
        Manifest::from_str(&text)
    } else {
        Err(anyhow!(
            "Could not get manifest for \"{}:{}\" from repository {:?} (status {})",
            namespace,
            tag,
            repository,
            resp.status()
        ))
    }
}

/// Pulls OCI blob and returns the response
fn oci_pull_blob(
    token: &str,
    repository: &str,
    namespace: &str,
    digest: &str,
    media_type: &str,
) -> Result<Response<Body>> {
    Ok(
        ureq::get(format!(
                "https://{repository}/v2/{namespace}/blobs/{digest}"
        ))
        .header(ACCEPT, media_type)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .call()?
    )
}

/// Downloads blob as path
pub fn oci_download_blob(
    token: &str,
    repository: &str,
    namespace: &str,
    digest: &str,
    media_type: &str,
    path: &str,
) -> Result<()> {
    let mut response = oci_pull_blob(token, repository, namespace, digest, media_type)?;

    let mut file = std::fs::File::create_new(path)
        .with_context(|| anyhow!("Creating file {:?}", path))?;

    std::io::copy(&mut response.body_mut().as_reader(), &mut file)
        .with_context(|| anyhow!("Writing blob to {:?}", path))?;

    Ok(())
}
