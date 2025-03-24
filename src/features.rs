use base64::Engine;
use tempfile::TempDir;
use crate::{prelude::*, util};
use std::{fmt::Display, path::PathBuf, process::Command, rc::Rc};

/// Path to a feature, local, remote, OCI or not
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeaturePath {
    /// Feature contained on local filesystem
    Local(String),

    Git {
        url: String,
        tag: Option<String>,
    },
}

impl FeaturePath {
    pub fn parse(input: &str) -> Result<Self> {
        if input.starts_with(".") || input.starts_with("/") {
            Ok(Self::Local(input.to_string()))
        } else if let Some(input) = input.strip_prefix("git+") {
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

impl Display for FeaturePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(x) => write!(f, "{x}"),
            Self::Git { url, tag } => write!(f, "git+{url}#{}", tag.as_deref().unwrap_or("")),
        }
    }
}

/// This is a feature that was already fetched, and exists physically
#[derive(Debug, Clone)]
pub struct Feature {
    pub feature_path: FeaturePath,
    pub path: PathBuf,

    /// Temp directory where the feature is stored, to link the lifetimes
    _temp_dir: Option<Rc<TempDir>>,
}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        // ignore the _temp_dir when comparing
        self.feature_path == other.feature_path && self.path == other.path
    }
}

impl Feature {
    /// Caches the feature
    pub fn cache_feature(feature_path: FeaturePath, temp_dir: Rc<TempDir>) -> Result<Self> {
        match feature_path {
            FeaturePath::Local(ref path) => {
                let path = PathBuf::from(path);

                if !path.exists() {
                    return Err(anyhow!("Local feature does not exist at {path:?}"));
                }

                if !path.join("install.sh").exists() {
                    return Err(anyhow!("Feature install script not found in {:?}", path));
                }

                // TODO check if install script is executable but do not modify it

                // do not copy local features
                Ok(Self {
                    feature_path,
                    path,
                    _temp_dir: None,
                })
            },
            FeaturePath::Git { ref url, ref tag } => {
                use base64::prelude::BASE64_URL_SAFE_NO_PAD as BASE64;

                // use url and tag encoded as base64 for cache
                let path = if let Some(ref tag) = tag {
                    temp_dir.path().join(format!(
                        "{}_{}",
                        BASE64.encode(&url),
                        BASE64.encode(&tag),
                    ))
                } else {
                    temp_dir.path().join(BASE64.encode(&url))
                };

                // clone only if it does not exist to stop downloading same thing twice
                if !path.exists() {
                    // TODO maybe replace with git2 crate for the progress bar etc?
                    util::git_clone(&path, &url, tag.as_deref())?;
                }

                if !path.join("install.sh").exists() {
                    return Err(anyhow!("Feature install script not found in {:?}", path));
                }

                // TODO ensure its set as executable

                Ok(Self {
                    feature_path,
                    path,
                    _temp_dir: Some(temp_dir),
                })
            },
        }
    }

    /// Create command using the install script of the feature
    pub fn command(&self) -> Command {
        Command::new(self.path.join("install.sh"))
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
            FeaturePath::parse_cli("git+https://github.com/sandorex/config"),
            Ok(FeaturePath::Git {
                url: "https://github.com/sandorex/config".to_string(),
                tag: None,
            })
        );

        assert_eq!(
            FeaturePath::parse_cli("git+https://github.com/sandorex/config#dev"),
            Ok(FeaturePath::Git {
                url: "https://github.com/sandorex/config".to_string(),
                tag: Some("dev".to_string()),
            })
        );
    }
}

