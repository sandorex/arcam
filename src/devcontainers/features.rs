//! Everything that has to do with devcontainer features

use crate::prelude::*;
use reqwest::header::ACCEPT;
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, rc::Weak};

//   "annotations": {
//     "dev.containers.metadata": "{\"id\":\"anaconda\",\"version\":\"1.0.12\",\"name\":\"Anaconda\",\"documentationURL\":\"https://github.com/devcontainers/features/tree/main/src/anaconda\",\"options\":{\"version\":{\"type\":\"string\",\"proposals\":[\"latest\"],\"default\":\"latest\",\"description\":\"Select or enter an anaconda version.\"}},\"containerEnv\":{\"CONDA_DIR\":\"/usr/local/conda\",\"PATH\":\"/usr/local/conda/bin:${PATH}\"},\"installsAfter\":[\"ghcr.io/devcontainers/features/common-utils\"]}",
//     "com.github.package.type": "devcontainer_feature"
//   }

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ManifestConfig {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ManifestLayer {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
    annotations: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ManifestV2 {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: ManifestConfig,
    layers: Vec<ManifestLayer>,
    annotations: HashMap<String, String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Manifest {
    V2(ManifestV2),
}

impl Manifest {
    pub fn from_str(input: &str) -> Result<Self> {
        // parse everything as generic json
        let val = serde_json::from_str::<serde_json::Value>(input)?;

        // get the version to check schema
        let version = val.get("schemaVersion")
            .with_context(|| anyhow!("Schema version was not found"))?
            .as_u64()
            .with_context(|| anyhow!("Schema version is not an u64"))?;

        let config = match version {
            2 => Manifest::V2(serde_json::from_value::<ManifestV2>(val)?),
            _ => return Err(anyhow!("Unknown schema version {:?}", version)),
        };

        Ok(config)
    }
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

pub fn oci_fetch_manifest(client: &reqwest::blocking::Client, repository: &str, namespace: &str, tag: &str) -> anyhow::Result<Manifest> {
    // TODO this token should be cached for other actions as well
    let token = get_oci_token(client, repository, namespace)?;

    let resp = client.get(format!("https://{repository}/v2/{namespace}/manifests/{tag}"))
        .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
        .bearer_auth(token)
        .send()?;

    if resp.status().is_success() {
        let text = resp.text()?;
        Manifest::from_str(&text)
    } else {
        Err(anyhow!("Could not get manifest for \"{}:{}\" from repository {:?}", namespace, tag, repository))
    }
}

