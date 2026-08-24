use crate::{MANIFEST_LIMIT, OmapackError, Source, strict_json};
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
    pub dependencies: BTreeMap<String, Source>,
}

impl ProjectManifest {
    pub fn parse(payload: &[u8]) -> Result<Self, OmapackError> {
        let raw: RawProjectManifest = strict_json(payload, MANIFEST_LIMIT)?;
        if raw.schema_version != 1 {
            return Err(OmapackError(
                "project schemaVersion must be the integer 1".into(),
            ));
        }
        if raw.profile != "omarchy" {
            return Err(OmapackError(
                "schema version 1 supports only the omarchy profile".into(),
            ));
        }
        let mut dependencies = BTreeMap::new();
        for (label, source) in raw.dependencies {
            if !valid_label(&label) {
                return Err(OmapackError(format!("invalid dependency label: {label:?}")));
            }
            dependencies.insert(label, Source::parse(&source)?);
        }
        Ok(Self { dependencies })
    }

    pub fn to_json(&self) -> Result<Vec<u8>, OmapackError> {
        let dependencies = self
            .dependencies
            .iter()
            .map(|(label, source)| (label.clone(), source.canonical()))
            .collect();
        json_bytes(&SerializableProjectManifest {
            schema_version: 1,
            profile: "omarchy",
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

    pub fn parse(payload: &[u8]) -> Result<Self, OmapackError> {
        let lock: Self = strict_json(payload, 4 * MANIFEST_LIMIT)?;
        if lock.schema_version != 1 {
            return Err(OmapackError(
                "lock schemaVersion must be the integer 1".into(),
            ));
        }
        Ok(lock)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, OmapackError> {
        json_bytes(self)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedPackage {
    pub source: String,
    pub repository_id: u64,
    pub repository_name: String,
    pub package_path: String,
    pub requested: String,
    pub version: Option<String>,
    pub tag: Option<String>,
    pub commit: String,
    pub digest: String,
    pub files: BTreeMap<String, String>,
}

pub fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, OmapackError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| OmapackError(format!("cannot serialize JSON: {error}")))?;
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
