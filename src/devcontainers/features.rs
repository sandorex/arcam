//! Everything that has to do with devcontainer features

use base64::Engine;
use tempfile::TempDir;

use super::structure::FeatureManifest;
use crate::{prelude::*, util};
use std::path::{Path, PathBuf};

const FEATURE_MANIFEST_FILENAME: &str = "devcontainer-feature.json";

/// Path to a feature, local, remote, OCI or not
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeaturePath {
    /// Feature contained on local filesystem
    Local(String),

    Git {
        url: String,
        tag: Option<String>,
    },

    OCI {
        repository: String,
        namespace: String,
        tag: Option<String>,
    },
}

impl FeaturePath {
    pub fn parse(input: &str) -> Result<Self> {
        if input.starts_with("./") || input.starts_with("/") {
            Ok(Self::Local(input.to_string()))
        } else if let Some(input) = input.strip_prefix("git://") {
            // split at `#` to allow specifying the branch/tag
            let (url, tag) = input
                .split_once("#")
                .map_or_else(|| (input.to_string(), None),
                             |(x, y)| (x.to_string(), Some(y.to_string())));

            Ok(Self::Git { url, tag })
        } else if input.starts_with("git@") {
            // split at `#` to allow specifying the branch/tag
            let (url, tag) = input
                .split_once("#")
                .map_or_else(|| (input.to_string(), None),
                             |(x, y)| (x.to_string(), Some(y.to_string())));

            Ok(Self::Git { url, tag })
        } else {
            Err(anyhow!("Invalid URI for feature"))
        }
    }

    /// Variant of the parse command compatible with clap
    pub fn parse_cli(input: &str) -> Result<Self, String> {
        // get whole context from anyhow error
        Self::parse(input).map_err(|err| format!("{err:#}"))
    }
}

/// This is a feature that was already fetched, and exists physically
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub feature_path: FeaturePath,
    pub path: PathBuf,
    pub is_oci: bool,
}

impl Feature {
    // TODO this cache needs to have a lockfile so it cannot be modified while its used!
    /// Caches the features, features that cannot be cached are kept in `temp_dir`
    pub fn cache_feature(feature_path: FeaturePath, cache_dir: &Path, temp_dir: &TempDir) -> Result<Self> {
        match feature_path {
            FeaturePath::Local(ref path) => {
                let path = PathBuf::from(path);

                if !path.exists() {
                    return Err(anyhow!("Local feature does not exist at {path:?}"));
                }

                // do not copy local features
                Ok(Self {
                    feature_path,
                    is_oci: path.join(FEATURE_MANIFEST_FILENAME).exists(),
                    path,
                })
            },
            FeaturePath::Git { ref url, ref tag } => {
                use base64::prelude::BASE64_STANDARD;

                // use url and tag encoded as base64 for cache
                let path = if let Some(ref tag) = tag {
                    cache_dir.join(format!(
                        "{}_{}",
                        BASE64_STANDARD.encode(&url),
                        BASE64_STANDARD.encode(&tag),
                    ))
                } else {
                    temp_dir.path().join(BASE64_STANDARD.encode(&url))
                };

                // TODO maybe replace with git2 crate for the progress bar etc?
                util::git_clone(&path, &url, tag.as_deref())?;

                Ok(Self {
                    feature_path,
                    is_oci: path.join(FEATURE_MANIFEST_FILENAME).exists(),
                    path,
                })
            },
            FeaturePath::OCI { .. } => todo!(),
        }
    }

    pub fn read_manifest(&self) -> Result<FeatureManifest> {
        assert!(self.is_oci);

        let file_path = self.path.join(FEATURE_MANIFEST_FILENAME);

        if !file_path.exists() {
            return Err(anyhow!("Could not find devcontainer-feature.json in feature {:?}", self.path));
        }

        let contents = std::fs::read_to_string(&file_path)
            .with_context(|| anyhow!("Could not read file {file_path:?}"))?;

        Ok(serde_json::from_str::<FeatureManifest>(&contents)
            .with_context(|| anyhow!("Could not deserialize feature manifest {file_path:?}"))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_path_parse() {
        assert_eq!(
            FeaturePath::parse_cli("/feature/local"),
            Ok(FeaturePath::Local("/feature/local".to_string()))
        );

        assert_eq!(
            FeaturePath::parse_cli("./feature/local"),
            Ok(FeaturePath::Local("./feature/local".to_string()))
        );

        assert_eq!(
            FeaturePath::parse_cli("git://github.com/sandorex/config"),
            Ok(FeaturePath::Git {
                url: "github.com/sandorex/config".to_string(),
                tag: None,
            })
        );

        assert_eq!(
            FeaturePath::parse_cli("git://github.com/sandorex/config#dev"),
            Ok(FeaturePath::Git {
                url: "github.com/sandorex/config".to_string(),
                tag: Some("dev".to_string()),
            })
        );

        assert_eq!(
            FeaturePath::parse_cli("git@github.com:sandorex/config.git"),
            Ok(FeaturePath::Git {
                url: "git@github.com:sandorex/config.git".to_string(),
                tag: None,
            })
        );

        assert_eq!(
            FeaturePath::parse_cli("git@github.com:sandorex/config.git#v1.0"),
            Ok(FeaturePath::Git {
                url: "git@github.com:sandorex/config.git".to_string(),
                tag: Some("v1.0".to_string()),
            })
        );
    }
}

