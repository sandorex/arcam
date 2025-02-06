//! Everything that has to do with devcontainer features

use crate::prelude::*;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use std::{collections::HashMap, rc::Weak};

// {
//   "schemaVersion": 2,
//   "mediaType": "application/vnd.oci.image.manifest.v1+json",
//   "config": {
//     "mediaType": "application/vnd.devcontainers",
//     "digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
//     "size": 0
//   },
//   "layers": [
//     {
//       "mediaType": "application/vnd.devcontainers.layer.v1+tar",
//       "digest": "sha256:b019d3c920cf1d98aec5aa78043f527401b74c403afb7455d3a39eb2ee05f35b",
//       "size": 14336,
//       "annotations": {
//         "org.opencontainers.image.title": "devcontainer-feature-anaconda.tgz"
//       }
//     }
//   ],
//   "annotations": {
//     "dev.containers.metadata": "{\"id\":\"anaconda\",\"version\":\"1.0.12\",\"name\":\"Anaconda\",\"documentationURL\":\"https://github.com/devcontainers/features/tree/main/src/anaconda\",\"options\":{\"version\":{\"type\":\"string\",\"proposals\":[\"latest\"],\"default\":\"latest\",\"description\":\"Select or enter an anaconda version.\"}},\"containerEnv\":{\"CONDA_DIR\":\"/usr/local/conda\",\"PATH\":\"/usr/local/conda/bin:${PATH}\"},\"installsAfter\":[\"ghcr.io/devcontainers/features/common-utils\"]}",
//     "com.github.package.type": "devcontainer_feature"
//   }
// }

#[derive(Debug, Deserialize)]
pub struct ManifestConfigV2 {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
pub struct ManifestLayerV2 {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
    annotations: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestV2 {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: ManifestConfigV2,
    layers: Vec<ManifestLayerV2>,
    annotations: HashMap<String, String>,
}

// #[derive(Debug, Default)]
// pub struct Feature {
//     pub name: String,
//     pub repository: String,
//     pub namespace: String,
//     pub dependencies: Vec<Weak<Self>>,
// }

fn get_oci_token(client: &reqwest::blocking::Client, repository: &str, namespace: &str) -> anyhow::Result<String> {
    match repository {
        "ghcr.io" => client.get("https://ghcr.io/token")
            .query(&[
                ("service", "ghcr.io"),
                ("scope", &format!("repository:{namespace}:pull")),
            ])
            .send()?
            .json::<HashMap<String, String>>()?
            .get("token")
            .cloned()
            .ok_or_else(|| anyhow!("No token returned from \"https://ghcr.io/token\"")),

        _ => return Err(anyhow::anyhow!("Unknown repository {:?} cannot get token", repository)),
    }
}

    // let response = c.get("https://ghcr.io/v2/devcontainers/features/anaconda/manifests/1.0.12")
    //     .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
    //     .bearer_auth(token)
    //     .send()?;

pub fn oci_fetch_manifest(client: &reqwest::blocking::Client, repository: &str, namespace: &str, tag: &str) -> anyhow::Result<ManifestV2> {
    let token = get_oci_token(client, repository, namespace)?;

    let resp = client.get(format!("https://{repository}/v2/{namespace}/manifests/{tag}"))
        .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
        .bearer_auth(token)
        .send()?;

    if resp.status().is_success() {
        // dbg!(resp.text());
        // Err(anyhow!("Temp error"))
        Ok(resp.json::<ManifestV2>()?)
    } else {
        Err(anyhow!("Could not get manifest for \"{}:{}\" from repository {:?}", namespace, tag, repository))
    }
}

