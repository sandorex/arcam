//! Everything that has to do with devcontainer features

use ureq::{http::{header::{ACCEPT, AUTHORIZATION}, Response}, Body};
use super::structure::OCIManifest;
use crate::prelude::*;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feature {
    /// Feature contained on local filesystem
    Local(String),

    /// Feature that is on remote server
    Remote {
        repository: String,
        namespace: String,
        tag: Option<String>,
    },
}

impl Feature {
    pub fn parse(input: &str) -> Result<Self> {
        if input.starts_with("./") || input.starts_with("/") {
            // TODO check if valid path?
            Ok(Self::Local(input.to_string()))
        } else {
            // split at colon for the tag/version
            let (input, tag) = input
                .split_once(":")
                .map_or_else(|| (input, None), |(x, y)| (x, Some(y)));

            // uri cant parse if no scheme
            let input = format!("https://{input}");

            let Ok(uri) = ureq::http::Uri::from_str(&input) else {
                return Err(anyhow!("Invalid URI {:?}", input));
            };

            let Some(host) = uri.host() else {
                return Err(anyhow!("Could not parse host from URI {:?}", input));
            };

            Ok(Self::Remote {
                repository: host.to_string(),
                namespace: uri.path().to_string(),
                tag: tag.map(|x| x.to_string()),
            })
        }
    }

    /// Variant of the parse command compatible with clap
    pub fn parse_cli(input: &str) -> Result<Self, String> {
        // get whole context from anyhow error
        Self::parse(input).map_err(|err| format!("{err:#}"))
    }
}

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
) -> Result<OCIManifest> {
    let mut resp = ureq::get(format!(
        "https://{repository}/v2/{namespace}/manifests/{tag}"
    ))
    .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
    .header(AUTHORIZATION, format!("Bearer {token}"))
    .call()?;

    if resp.status().is_success() {
        let text = resp.body_mut().read_to_string()?;
        OCIManifest::from_str(&text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_parse() {
        assert_eq!(
            Feature::parse_cli("/feature/local"),
            Ok(Feature::Local("/feature/local".to_string()))
        );

        assert_eq!(
            Feature::parse_cli("./feature/local"),
            Ok(Feature::Local("./feature/local".to_string()))
        );

        assert_eq!(
            Feature::parse_cli("ghcr.io/devcontainers/features/anaconda:1.0.12"),
            Ok(Feature::Remote {
                repository: "ghcr.io".to_string(),
                namespace: "/devcontainers/features/anaconda".to_string(),
                tag: Some("1.0.12".to_string()),
            })
        );

        assert_eq!(
            Feature::parse_cli("ghcr.io/devcontainers/features/anaconda"),
            Ok(Feature::Remote {
                repository: "ghcr.io".to_string(),
                namespace: "/devcontainers/features/anaconda".to_string(),
                tag: None,
            })
        );
    }
}
