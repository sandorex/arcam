//! Everything that has to do with devcontainer features

use crate::prelude::*;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use std::collections::HashMap;

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
pub struct ManifestLayer {
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
    config: ManifestConfig,
    layers: Vec<ManifestLayer>,
    annotations: HashMap<String, String>,
}

#[derive(Debug)]
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
            _ => return Err(anyhow!("Unsupported schema version {:?}", version)),
        };

        Ok(config)
    }
}

/// All customizations except arcam are ignored
#[derive(Debug, Deserialize)]
pub struct Customization {
    // TODO allow arcam customizations in this?
    arcam: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Feature {
    // these are ordered as in the schema file
    #[serde(rename = "capAdd")]
    capability_add: Vec<String>,

    #[serde(rename = "containerEnv")]
    container_environ: HashMap<String, String>,

    customizations: Customization,

    description: String,

    #[serde(rename = "documentationURL")]
    documentation_url: String,

    keywords: Vec<String>,

    entrypoint: String,

    id: String,

    init: bool,

    #[serde(rename = "installsAfter")]
    installs_after: Vec<String>,

    // TODO its an object
    #[serde(rename = "dependsOn")]
    depends_on: HashMap<String, serde_json::Value>,

    #[serde(rename = "licenseURL")]
    license_url: String,

    // TODO its array of objects
    mounts: Vec<serde_json::Value>,

    name: String,

    // TODO this is also a weird object
    options: serde_json::Value,

    privileged: bool,

    #[serde(rename = "securityOpt")]
    security_opt: Vec<String>,

    version: String,

    // TODO useless?
    #[serde(rename = "legacyIds")]
    legacy_ids: serde_json::Value,

    deprecated: bool,

    // TODO object, string or array of strinsg
    #[serde(rename = "securityOpt")]
    on_create_cmd: serde_json::Value,

    // TODO same as on_create_cmd
    #[serde(rename = "updateContentCommand")]
    update_content_cmd: serde_json::Value,

    // TODO same as on_create_cmd
    #[serde(rename = "postCreateCommand")]
    post_create_cmd: serde_json::Value,

    // TODO same as on_create_cmd
    #[serde(rename = "postStartCommand")]
    post_start_cmd: serde_json::Value,

    // TODO same as on_create_cmd
    #[serde(rename = "postAttachCommand")]
    post_attach_cmd: serde_json::Value,
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

// pub fn oci_pull()

