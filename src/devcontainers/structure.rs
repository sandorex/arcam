use crate::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

pub const ANNOTATION_FEATURE_METADATA: &str = "dev.containers.metadata";

//   "annotations": {
//     "dev.containers.metadata": "{\"id\":\"anaconda\",\"version\":\"1.0.12\",\"name\":\"Anaconda\",\"documentationURL\":\"https://github.com/devcontainers/features/tree/main/src/anaconda\",\"options\":{\"version\":{\"type\":\"string\",\"proposals\":[\"latest\"],\"default\":\"latest\",\"description\":\"Select or enter an anaconda version.\"}},\"containerEnv\":{\"CONDA_DIR\":\"/usr/local/conda\",\"PATH\":\"/usr/local/conda/bin:${PATH}\"},\"installsAfter\":[\"ghcr.io/devcontainers/features/common-utils\"]}",
//     "com.github.package.type": "devcontainer_feature"
//   }
// pub struct DevcontainersMetadata {
//
// }

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OCIManifestConfig {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub digest: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct OCIManifestLayer {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub digest: String,
    pub size: u64,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct OCIManifestV2 {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub config: OCIManifestConfig,
    pub layers: Vec<OCIManifestLayer>,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug)]
pub enum OCIManifest {
    V2(OCIManifestV2),
}

impl OCIManifest {
    pub fn from_str(input: &str) -> Result<Self> {
        // parse everything as generic json
        let val = serde_json::from_str::<serde_json::Value>(input)?;
        Self::from_value(val)
    }

    pub fn from_value(input: serde_json::Value) -> Result<Self> {
        // get the version to check schema
        let version = input
            .get("schemaVersion")
            .with_context(|| anyhow!("Schema version was not found"))?
            .as_u64()
            .with_context(|| anyhow!("Schema version is not an u64"))?;

        let config = match version {
            2 => OCIManifest::V2(serde_json::from_value::<OCIManifestV2>(input)?),
            _ => return Err(anyhow!("Unsupported schema version {:?}", version)),
        };

        Ok(config)
    }
}

// /// All customizations except arcam are ignored
// #[derive(Debug, Deserialize)]
// pub struct Customization {
//     // TODO allow arcam customizations in this?
//     arcam: serde_json::Value,
// }

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ArrayOrString {
    String(String),
    Array(Vec<String>),
}

/// This is type that holds a String, Array<String> or Object (which contains String or
/// Array<String>)
///
/// Array: Passed to the OS for execution without going through a shell
/// String: Goes through a shell (it needs to be parsed into command and arguments)
/// Object: All lifecycle scripts have been extended to support object types to allow for parallel execution
/// (source: https://containers.dev/implementors/json_reference/#formatting-string-vs-array-properties)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ExecCommands {
    String(String),
    Array(Vec<String>),
    Object(HashMap<String, ArrayOrString>),
}

#[derive(Debug, Deserialize)]
pub struct FeatureManifest {
    // #[serde(rename = "capAdd")]
    // pub capability_add: Vec<String>,
    //
    // #[serde(rename = "containerEnv")]
    // pub container_environ: HashMap<String, String>,

    // pub customizations: Customization,

    // pub keywords: Vec<String>,
    pub entrypoint: String,

    pub id: String,

    #[serde(rename = "installsAfter")]
    pub installs_after: Vec<String>,

    // TODO its an object
    #[serde(rename = "dependsOn")]
    pub depends_on: HashMap<String, serde_json::Value>,

    // TODO its array of objects
    pub mounts: Vec<serde_json::Value>,

    pub name: String,

    // TODO this is also a weird object
    pub options: serde_json::Value,

    // pub privileged: bool,
    pub version: String,

    // // TODO useless?
    // #[serde(rename = "legacyIds")]
    // pub legacy_ids: serde_json::Value,

    // pub deprecated: bool,

    // TODO object, string or array of strinsg
    #[serde(rename = "securityOpt")]
    pub on_create_cmd: ExecCommands,

    // TODO same as on_create_cmd
    #[serde(rename = "postCreateCommand")]
    pub post_create_cmd: ExecCommands,

    // TODO same as on_create_cmd
    #[serde(rename = "postStartCommand")]
    pub post_start_cmd: ExecCommands,
    // TODO this one is not possible atm
    // // TODO same as on_create_cmd
    // #[serde(rename = "postAttachCommand")]
    // pub post_attach_cmd: ArrayStringObject,
}
