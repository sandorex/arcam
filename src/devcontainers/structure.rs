#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;


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

    // pub keywords: Vec<String>,
    pub entrypoint: String,

    pub id: String,

    // #[serde(rename = "installsAfter")]
    // pub installs_after: Vec<String>,

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
