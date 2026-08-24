use crate::{
    FILE_LIMIT, FILES_LIMIT, MANIFEST_LIMIT, PACKAGE_LIMIT, PackageFile, PackageManifest,
    QmlpackError, Resolution, ResolvedPackage, Source, strict_json, validate_path,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use flate2::read::GzDecoder;
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::redirect::Policy;
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::time::Duration;
use tar::Archive;
use unicode_casefold::UnicodeCaseFold;

const REGISTRY: &str = "https://registry.npmjs.org";
const METADATA_LIMIT: usize = 256 * 1024;
const TARBALL_LIMIT: usize = 8 * 1024 * 1024;
const EXPANDED_LIMIT: usize = PACKAGE_LIMIT + 1024 * 1024;
const ARCHIVE_FILES_LIMIT: usize = FILES_LIMIT + 64;

pub struct NpmClient {
    client: Client,
}

impl NpmClient {
    pub fn new() -> Result<Self, QmlpackError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("qmlpack/0.1"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .build()
            .map_err(|error| QmlpackError(format!("cannot create HTTPS client: {error}")))?;
        Ok(Self { client })
    }

    pub fn resolve(&self, source: Source) -> Result<ResolvedPackage, QmlpackError> {
        let Source::Npm(npm) = &source else {
            return Err(QmlpackError("npm client received a GitHub source".into()));
        };
        let name_path = npm.name.replace('/', "%2f");
        let metadata_url = format!("{REGISTRY}/{name_path}/{}", npm.version);
        let metadata: VersionMetadata = strict_json(
            &response_bytes(self.client.get(metadata_url).send(), METADATA_LIMIT)?,
            METADATA_LIMIT,
        )?;
        if metadata.name != npm.name || metadata.version != npm.version.to_string() {
            return Err(QmlpackError(
                "npm metadata identity does not match the requested package".into(),
            ));
        }
        validate_tarball_url(&metadata.dist.tarball)?;
        let tarball = response_bytes(
            self.client.get(&metadata.dist.tarball).send(),
            TARBALL_LIMIT,
        )?;
        verify_integrity(&tarball, &metadata.dist.integrity)?;
        let (manifest, files) = extract_package(&tarball, &npm.name, &npm.version.to_string())?;
        let digest = crate::package_digest(&manifest, &files)?;
        Ok(ResolvedPackage {
            source,
            resolution: Resolution::Npm {
                registry: REGISTRY.into(),
                name: metadata.name,
                version: metadata.version,
                integrity: metadata.dist.integrity,
            },
            manifest,
            files,
            digest,
        })
    }
}

fn response_bytes(
    response: Result<Response, reqwest::Error>,
    limit: usize,
) -> Result<Vec<u8>, QmlpackError> {
    let response =
        response.map_err(|error| QmlpackError(format!("npm request failed: {error}")))?;
    if !response.status().is_success() {
        return Err(QmlpackError(format!(
            "npm request returned HTTP {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(QmlpackError(format!("npm response exceeds {limit} bytes")));
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| QmlpackError(format!("cannot read npm response: {error}")))?;
    if bytes.len() > limit {
        return Err(QmlpackError(format!("npm response exceeds {limit} bytes")));
    }
    Ok(bytes)
}

fn validate_tarball_url(value: &str) -> Result<(), QmlpackError> {
    let url = Url::parse(value).map_err(|_| QmlpackError("invalid npm tarball URL".into()))?;
    if url.scheme() != "https"
        || url.host_str() != Some("registry.npmjs.org")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(QmlpackError(
            "npm tarball URL is outside the public registry".into(),
        ));
    }
    Ok(())
}

fn verify_integrity(bytes: &[u8], integrity: &str) -> Result<(), QmlpackError> {
    let mut supported = false;
    for token in integrity.split_ascii_whitespace() {
        let Some((algorithm, expected)) = token.split_once('-') else {
            continue;
        };
        let actual = match algorithm {
            "sha512" => BASE64.encode(Sha512::digest(bytes)),
            "sha256" => BASE64.encode(Sha256::digest(bytes)),
            _ => continue,
        };
        supported = true;
        if actual == expected {
            return Ok(());
        }
    }
    if supported {
        Err(QmlpackError("npm tarball integrity mismatch".into()))
    } else {
        Err(QmlpackError(
            "npm metadata has no supported SHA-256 or SHA-512 integrity".into(),
        ))
    }
}

fn extract_package(
    tarball: &[u8],
    expected_name: &str,
    expected_version: &str,
) -> Result<(PackageManifest, Vec<PackageFile>), QmlpackError> {
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(tarball)));
    let mut expanded = 0usize;
    let mut entries_seen = 0usize;
    let mut regular_files = 0usize;
    let mut contents = BTreeMap::new();
    let mut folded = BTreeSet::new();
    for entry in archive
        .entries()
        .map_err(|error| QmlpackError(format!("invalid npm tarball: {error}")))?
    {
        let entry = entry.map_err(|error| QmlpackError(format!("invalid npm entry: {error}")))?;
        entries_seen += 1;
        if entries_seen > 1024 {
            return Err(QmlpackError("npm tarball contains too many entries".into()));
        }
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            return Err(QmlpackError(
                "npm tarball contains a link or special file".into(),
            ));
        }
        regular_files += 1;
        if regular_files > ARCHIVE_FILES_LIMIT {
            return Err(QmlpackError("npm tarball contains too many files".into()));
        }
        let archive_path = entry
            .path()
            .map_err(|_| QmlpackError("npm tarball contains an invalid path".into()))?;
        let path = archive_path
            .to_str()
            .and_then(|path| path.strip_prefix("package/"))
            .ok_or_else(|| QmlpackError("npm tarball path is outside package/".into()))?;
        let path = validate_path(path, true)?;
        let collision_key = path.case_fold().collect::<String>();
        if !folded.insert(collision_key) {
            return Err(QmlpackError(format!("duplicate npm tarball path: {path}")));
        }
        let size = usize::try_from(entry.size())
            .map_err(|_| QmlpackError("npm tarball entry is too large".into()))?;
        let file_limit = if matches!(path.as_str(), "qmlpack.json" | "package.json") {
            MANIFEST_LIMIT
        } else {
            FILE_LIMIT
        };
        if size > file_limit || expanded.saturating_add(size) > EXPANDED_LIMIT {
            return Err(QmlpackError(
                "npm tarball exceeds expanded byte limits".into(),
            ));
        }
        expanded += size;
        let mode = entry
            .header()
            .mode()
            .map_err(|_| QmlpackError("npm tarball has an invalid file mode".into()))?;
        let mut bytes = Vec::with_capacity(size);
        entry
            .take(size as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| QmlpackError(format!("cannot read npm tarball entry: {error}")))?;
        if bytes.len() != size {
            return Err(QmlpackError("npm tarball entry size mismatch".into()));
        }
        contents.insert(path, (bytes, mode & 0o111 != 0));
    }

    let (manifest_bytes, _) = contents
        .get("qmlpack.json")
        .ok_or_else(|| QmlpackError("npm package is missing qmlpack.json".into()))?;
    if manifest_bytes.len() > MANIFEST_LIMIT {
        return Err(QmlpackError("qmlpack.json exceeds its byte limit".into()));
    }
    let manifest = PackageManifest::parse(manifest_bytes)?;
    let expected_short_name = expected_name.rsplit('/').next().unwrap_or(expected_name);
    if manifest.name != expected_short_name {
        return Err(QmlpackError(
            "qmlpack.json name does not match the npm package name".into(),
        ));
    }
    let (package_json, _) = contents
        .get("package.json")
        .ok_or_else(|| QmlpackError("npm package is missing package.json".into()))?;
    let package: NpmPackageJson = strict_json(package_json, MANIFEST_LIMIT)?;
    if package.name != expected_name || package.version != expected_version {
        return Err(QmlpackError(
            "package.json identity does not match npm registry metadata".into(),
        ));
    }

    let files = manifest
        .files
        .iter()
        .map(|path| {
            let (content, executable) = contents
                .get(path)
                .ok_or_else(|| QmlpackError(format!("npm package is missing {path}")))?;
            if *executable != manifest.executables.contains(path) {
                return Err(QmlpackError(format!(
                    "npm file mode does not match qmlpack.json: {path}"
                )));
            }
            Ok(PackageFile {
                path: path.clone(),
                content: content.clone(),
                executable: *executable,
            })
        })
        .collect::<Result<Vec<_>, QmlpackError>>()?;
    Ok((manifest, files))
}

#[derive(Deserialize)]
struct VersionMetadata {
    name: String,
    version: String,
    dist: DistMetadata,
}

#[derive(Deserialize)]
struct DistMetadata {
    tarball: String,
    integrity: String,
}

#[derive(Deserialize)]
struct NpmPackageJson {
    name: String,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::{Builder, EntryType, Header};

    fn package_tarball() -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = Builder::new(encoder);
        for (path, bytes, mode) in [
            (
                "package/package.json",
                br#"{"name":"@silouanwright/oma-ui","version":"0.2.0"}"#.as_slice(),
                0o644,
            ),
            (
                "package/qmlpack.json",
                br#"{"schemaVersion":1,"name":"oma-ui","license":"MIT","files":["Ui/Button.qml"]}"#
                    .as_slice(),
                0o644,
            ),
            ("package/Ui/Button.qml", b"import QtQuick\nItem {}\n", 0o644),
        ] {
            let mut header = Header::new_gnu();
            header.set_entry_type(EntryType::Regular);
            header.set_mode(mode);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            builder.append_data(&mut header, path, bytes).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn linked_tarball() -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = Builder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Symlink);
        header.set_mode(0o777);
        header.set_size(0);
        header.set_cksum();
        builder
            .append_link(&mut header, "package/qmlpack.json", "/etc/passwd")
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn integrity_and_registry_url_are_strict() {
        let bytes = b"qmlpack";
        let integrity = format!("sha512-{}", BASE64.encode(Sha512::digest(bytes)));
        verify_integrity(bytes, &integrity).unwrap();
        assert!(verify_integrity(b"changed", &integrity).is_err());
        validate_tarball_url("https://registry.npmjs.org/a/-/a-1.0.0.tgz").unwrap();
        assert!(validate_tarball_url("https://example.com/a.tgz").is_err());
    }

    #[test]
    fn bounded_tarball_materializes_only_declared_files() {
        let (manifest, files) =
            extract_package(&package_tarball(), "@silouanwright/oma-ui", "0.2.0").unwrap();
        assert_eq!(manifest.name, "oma-ui");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "Ui/Button.qml");
        assert!(extract_package(&linked_tarball(), "x", "1.0.0").is_err());
    }
}
