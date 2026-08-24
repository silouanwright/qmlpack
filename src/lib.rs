use semver::Version;
use serde::Deserialize;
use serde::de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

pub mod github;
pub mod project;
pub mod resolver;
pub mod workspace;

pub const MANIFEST_LIMIT: usize = 64 * 1024;
pub const FILE_LIMIT: usize = 4 * 1024 * 1024;
pub const PACKAGE_LIMIT: usize = 16 * 1024 * 1024;
pub const FILES_LIMIT: usize = 256;
pub const DEPENDENCIES_LIMIT: usize = 32;
pub const PACKAGES_LIMIT: usize = 128;
pub const DEPENDENCY_DEPTH_LIMIT: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QmlpackError(pub String);

impl fmt::Display for QmlpackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for QmlpackError {}

impl From<std::io::Error> for QmlpackError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("valid JSON without duplicate object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValueSeed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = entries.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
            }
            values.insert(key, entries.next_value_seed(StrictValueSeed)?);
        }
        Ok(Value::Object(values))
    }
}

struct StrictValueSeed;

impl<'de> de::DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

pub fn strict_json<T: DeserializeOwned>(payload: &[u8], limit: usize) -> Result<T, QmlpackError> {
    if payload.len() > limit {
        return Err(QmlpackError(format!("JSON exceeds {limit} bytes")));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(payload);
    let value = de::DeserializeSeed::deserialize(StrictValueSeed, &mut deserializer)
        .map_err(|error| QmlpackError(format!("invalid JSON: {error}")))?;
    deserializer
        .end()
        .map_err(|error| QmlpackError(format!("invalid JSON: {error}")))?;
    serde_json::from_value(value)
        .map_err(|error| QmlpackError(format!("invalid JSON shape: {error}")))
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || (byte == b'-' && index > 0)
        })
        && !value.ends_with('-')
}

fn validate_path(value: &str, allow_manifest_name: bool) -> Result<String, QmlpackError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.bytes().any(|byte| byte < 32)
        || value.len() > 1024
        || value.nfc().collect::<String>() != value
    {
        return Err(QmlpackError(format!("unsafe file path: {value:?}")));
    }
    let components: Vec<_> = value.split('/').collect();
    if components.iter().any(|component| {
        component.is_empty()
            || *component == "."
            || *component == ".."
            || *component == ".git"
            || *component == ".qmlpack"
            || component.len() > 255
    }) {
        return Err(QmlpackError(format!("unsafe file path: {value:?}")));
    }
    let filename = components.last().expect("non-empty path");
    if !allow_manifest_name && (*filename == "qmlpack.json" || *filename == "qmlpack.lock") {
        return Err(QmlpackError(format!("reserved file path: {value:?}")));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub owner: String,
    pub repository: String,
    pub package_path: String,
    pub requested: String,
}

impl Source {
    pub fn parse(value: &str) -> Result<Self, QmlpackError> {
        let remainder = value
            .strip_prefix("github:")
            .ok_or_else(|| QmlpackError("source must start with github:".into()))?;
        let (location, requested) = remainder
            .rsplit_once('@')
            .filter(|(_, requested)| !requested.is_empty())
            .ok_or_else(|| QmlpackError("source must end with @<version-or-commit>".into()))?;
        let mut components = location.split('/');
        let owner = components.next().unwrap_or_default();
        let repository = components.next().unwrap_or_default();
        if !valid_repo_part(owner) || !valid_repo_part(repository) {
            return Err(QmlpackError("invalid GitHub owner or repository".into()));
        }
        let package_path = components.collect::<Vec<_>>().join("/");
        if !package_path.is_empty() {
            validate_path(&package_path, true)?;
        }
        let requested = if is_sha(requested) {
            requested.to_ascii_lowercase()
        } else {
            let semantic = requested.strip_prefix('v').unwrap_or(requested);
            Version::parse(semantic).map_err(|_| {
                QmlpackError("reference must be an exact 40-character commit or SemVer".into())
            })?;
            requested.to_owned()
        };
        Ok(Self {
            owner: owner.to_owned(),
            repository: repository.to_owned(),
            package_path,
            requested,
        })
    }

    pub fn canonical(&self) -> String {
        let path = if self.package_path.is_empty() {
            String::new()
        } else {
            format!("/{}", self.package_path)
        };
        format!(
            "github:{}/{}{}@{}",
            self.owner, self.repository, path, self.requested
        )
    }

    pub fn version(&self) -> Option<Version> {
        if is_sha(&self.requested) {
            None
        } else {
            Version::parse(self.requested.strip_prefix('v').unwrap_or(&self.requested)).ok()
        }
    }

    pub fn release_tag(&self) -> Option<String> {
        self.version().map(|version| {
            let prefix = if self.package_path.is_empty() {
                String::new()
            } else {
                format!("{}/", self.package_path)
            };
            format!("{prefix}v{version}")
        })
    }
}

fn valid_repo_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric()
                || ((byte == b'.' || byte == b'_' || byte == b'-') && index > 0)
        })
}

fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawManifest {
    schema_version: u64,
    name: String,
    license: String,
    files: Vec<String>,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default)]
    compatibility: Compatibility,
    #[serde(default)]
    executables: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Compatibility {
    qt: Option<String>,
    omarchy: Option<String>,
    quickshell: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub raw: Vec<u8>,
    pub name: String,
    pub license: String,
    pub files: Vec<String>,
    pub dependencies: BTreeMap<String, Source>,
    pub compatibility: BTreeMap<String, String>,
    pub executables: BTreeSet<String>,
}

impl PackageManifest {
    pub fn parse(payload: &[u8]) -> Result<Self, QmlpackError> {
        let raw: RawManifest = strict_json(payload, MANIFEST_LIMIT)?;
        if raw.schema_version != 1 {
            return Err(QmlpackError("schemaVersion must be the integer 1".into()));
        }
        if !valid_name(&raw.name) {
            return Err(QmlpackError(
                "name must use lowercase letters, digits, and internal hyphens".into(),
            ));
        }
        if raw.license.trim().is_empty() || raw.license.len() > 128 {
            return Err(QmlpackError(
                "license must be a non-empty SPDX expression".into(),
            ));
        }
        if raw.files.is_empty() || raw.files.len() > FILES_LIMIT {
            return Err(QmlpackError(format!(
                "files must contain 1 to {FILES_LIMIT} paths"
            )));
        }
        let mut files = Vec::with_capacity(raw.files.len());
        let mut folded = BTreeMap::new();
        for path in raw.files {
            let path = validate_path(&path, false)?;
            let key = path.case_fold().collect::<String>();
            if let Some(previous) = folded.insert(key, path.clone()) {
                return Err(QmlpackError(format!(
                    "file paths collide: {previous:?} and {path:?}"
                )));
            }
            files.push(path);
        }
        if raw.dependencies.len() > DEPENDENCIES_LIMIT {
            return Err(QmlpackError(format!(
                "packages may declare at most {DEPENDENCIES_LIMIT} dependencies"
            )));
        }
        let mut dependencies = BTreeMap::new();
        for (label, source) in raw.dependencies {
            if !valid_name(&label) {
                return Err(QmlpackError(format!("invalid dependency label: {label:?}")));
            }
            dependencies.insert(label, Source::parse(&source)?);
        }
        let mut compatibility = BTreeMap::new();
        for (host, requirement) in [
            ("qt", raw.compatibility.qt),
            ("omarchy", raw.compatibility.omarchy),
            ("quickshell", raw.compatibility.quickshell),
        ] {
            if let Some(requirement) = requirement {
                if requirement.trim().is_empty() || requirement.len() > 128 {
                    return Err(QmlpackError(format!(
                        "compatibility.{host} must be a non-empty string"
                    )));
                }
                compatibility.insert(host.to_owned(), requirement);
            }
        }
        let mut executables = BTreeSet::new();
        for path in raw.executables {
            let path = validate_path(&path, false)?;
            if !files.contains(&path) {
                return Err(QmlpackError(
                    "every executable must also appear in files".into(),
                ));
            }
            executables.insert(path);
        }
        Ok(Self {
            raw: payload.to_vec(),
            name: raw.name,
            license: raw.license,
            files,
            dependencies,
            compatibility,
            executables,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PackageFile {
    pub path: String,
    pub content: Vec<u8>,
    pub executable: bool,
}

impl PackageFile {
    pub fn digest(&self) -> String {
        format!("sha256:{:x}", Sha256::digest(&self.content))
    }

    fn mode(&self) -> &'static [u8] {
        if self.executable { b"0755" } else { b"0644" }
    }
}

pub fn package_digest(
    manifest: &PackageManifest,
    files: &[PackageFile],
) -> Result<String, QmlpackError> {
    let by_path: BTreeMap<_, _> = files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    if by_path.len() != files.len() || by_path.len() != manifest.files.len() {
        return Err(QmlpackError("package files do not match manifest".into()));
    }
    let total: usize = files.iter().map(|file| file.content.len()).sum();
    if total > PACKAGE_LIMIT {
        return Err(QmlpackError(format!(
            "package exceeds {PACKAGE_LIMIT} bytes"
        )));
    }
    let mut digest = Sha256::new();
    digest.update(b"qmlpack-package-v1\0");
    digest.update((manifest.raw.len() as u64).to_be_bytes());
    digest.update(&manifest.raw);
    let mut paths = manifest.files.iter().collect::<Vec<_>>();
    paths.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for path in paths {
        let file = by_path
            .get(path.as_str())
            .ok_or_else(|| QmlpackError(format!("missing package file: {path}")))?;
        if file.content.len() > FILE_LIMIT {
            return Err(QmlpackError(format!(
                "file exceeds {FILE_LIMIT} bytes: {path}"
            )));
        }
        if file.executable != manifest.executables.contains(path) {
            return Err(QmlpackError(format!(
                "file mode does not match manifest: {path}"
            )));
        }
        digest.update((path.len() as u64).to_be_bytes());
        digest.update(path.as_bytes());
        digest.update(file.mode());
        digest.update(b"\0");
        digest.update((file.content.len() as u64).to_be_bytes());
        digest.update(&file.content);
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(extra: &str) -> Vec<u8> {
        format!(
            "{{\"files\":[\"Ui/Button.qml\",\"Ui/qmldir\"],\"license\":\"MIT\",\"name\":\"oma-ui\",\"schemaVersion\":1{extra}}}\n"
        )
        .into_bytes()
    }

    #[test]
    fn source_versions_and_commits_are_unambiguous() {
        let source = Source::parse("github:silouanwright/omatools/packages/oma-ui@0.2.0").unwrap();
        assert_eq!(
            source.release_tag().as_deref(),
            Some("packages/oma-ui/v0.2.0")
        );
        assert_eq!(source.version().unwrap(), Version::new(0, 2, 0));

        let commit = Source::parse(&format!(
            "github:silouanwright/omatools/oma-ui@{}",
            "a".repeat(40)
        ))
        .unwrap();
        assert!(commit.version().is_none());
        for invalid in [
            "github:owner/repo/pkg@main",
            "github:owner/repo/../pkg@1.0.0",
            "github:owner/repo/pkg/@1.0.0",
            "github:owner/repo/pkg@1.0.0-01",
        ] {
            assert!(Source::parse(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn strict_manifest_rejects_duplicate_and_colliding_paths() {
        assert!(PackageManifest::parse(br#"{"schemaVersion":1,"name":"x","name":"y"}"#).is_err());
        let collision =
            br#"{"schemaVersion":1,"name":"x","license":"MIT","files":["Ui/A.qml","ui/a.qml"]}"#;
        assert!(PackageManifest::parse(collision).is_err());
        let unicode_collision = r#"{"schemaVersion":1,"name":"x","license":"MIT","files":["Ui/Straße.qml","Ui/STRASSE.qml"]}"#;
        assert!(PackageManifest::parse(unicode_collision.as_bytes()).is_err());
        let reserved =
            br#"{"schemaVersion":1,"name":"x","license":"MIT","files":["qmlpack.json"]}"#;
        assert!(PackageManifest::parse(reserved).is_err());
    }

    #[test]
    fn digest_has_a_fixed_vector() {
        let manifest = PackageManifest::parse(&manifest("")).unwrap();
        let files = vec![
            PackageFile {
                path: "Ui/Button.qml".into(),
                content: b"import QtQuick\n".to_vec(),
                executable: false,
            },
            PackageFile {
                path: "Ui/qmldir".into(),
                content: b"Button 1.0 Button.qml\n".to_vec(),
                executable: false,
            },
        ];
        assert_eq!(
            package_digest(&manifest, &files).unwrap(),
            "sha256:e174cb84faca8c982dc245397881c1491ed488792eacd5ebf6744d87e9fad7f3"
        );
        let mut reversed = files.clone();
        reversed.reverse();
        assert_eq!(
            package_digest(&manifest, &files),
            package_digest(&manifest, &reversed)
        );
    }
}
