use crate::{MANIFEST_LIMIT, QmlpackError, Source, strict_json};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawProjectManifest {
    schema_version: u64,
    profile: String,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ProjectManifest {
    pub profile: String,
    pub dependencies: BTreeMap<String, Source>,
}

impl ProjectManifest {
    pub fn parse(payload: &[u8]) -> Result<Self, QmlpackError> {
        let raw: RawProjectManifest = strict_json(payload, MANIFEST_LIMIT)?;
        if raw.schema_version != 1 {
            return Err(QmlpackError(
                "project schemaVersion must be the integer 1".into(),
            ));
        }
        if !matches!(raw.profile.as_str(), "qml" | "quickshell" | "omarchy") {
            return Err(QmlpackError(
                "profile must be qml, quickshell, or omarchy".into(),
            ));
        }
        let mut dependencies = BTreeMap::new();
        for (label, source) in raw.dependencies {
            if !valid_label(&label) {
                return Err(QmlpackError(format!("invalid dependency label: {label:?}")));
            }
            dependencies.insert(label, Source::parse(&source)?);
        }
        Ok(Self {
            profile: raw.profile,
            dependencies,
        })
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QmlpackError> {
        let dependencies = self
            .dependencies
            .iter()
            .map(|(label, source)| (label.clone(), source.canonical()))
            .collect();
        json_bytes(&SerializableProjectManifest {
            schema_version: 1,
            profile: &self.profile,
            dependencies,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableProjectManifest<'a> {
    schema_version: u64,
    profile: &'a str,
    dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lockfile {
    pub schema_version: u64,
    pub packages: BTreeMap<String, LockedPackage>,
}

impl Lockfile {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            packages: BTreeMap::new(),
        }
    }

    pub fn parse(payload: &[u8]) -> Result<Self, QmlpackError> {
        let lock: Self = strict_json(payload, 4 * MANIFEST_LIMIT)?;
        if lock.schema_version != 1 {
            return Err(QmlpackError(
                "lock schemaVersion must be the integer 1".into(),
            ));
        }
        Ok(lock)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QmlpackError> {
        json_bytes(self)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedPackage {
    pub source: String,
    pub resolution: LockedResolution,
    pub digest: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "transport", rename_all = "lowercase", deny_unknown_fields)]
pub enum LockedResolution {
    Github {
        repository_id: u64,
        repository_name: String,
        package_path: String,
        requested: String,
        version: Option<String>,
        tag: Option<String>,
        commit: String,
    },
    Npm {
        registry: String,
        name: String,
        version: String,
        integrity: String,
    },
}

pub fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, QmlpackError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| QmlpackError(format!("cannot serialize JSON: {error}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || (byte == b'-' && index > 0)
        })
        && !value.ends_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_lock_serialization_are_deterministic() {
        let project = ProjectManifest::parse(
            br#"{"schemaVersion":1,"profile":"omarchy","dependencies":{"ui":"github:o/r/p@1.0.0"}}"#,
        )
        .unwrap();
        let first = project.to_json().unwrap();
        assert_eq!(
            first,
            ProjectManifest::parse(&first).unwrap().to_json().unwrap()
        );
        let lock = Lockfile::empty();
        assert_eq!(lock, Lockfile::parse(&lock.to_json().unwrap()).unwrap());
    }
}
