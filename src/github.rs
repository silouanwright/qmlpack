use crate::{
    FILE_LIMIT, MANIFEST_LIMIT, PackageFile, PackageManifest, QmlpackError, Source, strict_json,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::time::Duration;

const API_JSON_LIMIT: usize = 2 * 1024 * 1024;
const TREE_ENTRIES_LIMIT: usize = 4096;

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub source: Source,
    pub repository_id: u64,
    pub repository_name: String,
    pub commit: String,
    pub manifest: PackageManifest,
    pub files: Vec<PackageFile>,
    pub digest: String,
}

pub struct GitHubClient {
    client: Client,
    trees: BTreeMap<String, Vec<TreeEntry>>,
}

impl GitHubClient {
    pub fn new(token: Option<&str>) -> Result<Self, QmlpackError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("qmlpack/0.1"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static("2022-11-28"),
        );
        if let Some(token) = token.filter(|token| !token.is_empty()) {
            let value = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| QmlpackError("GITHUB_TOKEN contains invalid header bytes".into()))?;
            headers.insert(AUTHORIZATION, value);
        }
        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .build()
            .map_err(|error| QmlpackError(format!("cannot create HTTPS client: {error}")))?;
        Ok(Self {
            client,
            trees: BTreeMap::new(),
        })
    }

    pub fn resolve(&mut self, source: Source) -> Result<ResolvedPackage, QmlpackError> {
        let repository: RepositoryResponse = self.api_json(
            &["repos", &source.owner, &source.repository],
            API_JSON_LIMIT,
        )?;
        if repository.private {
            return Err(QmlpackError(
                "private GitHub repositories are not supported in schema version 1".into(),
            ));
        }
        if repository.id == 0 || repository.full_name.split('/').count() != 2 {
            return Err(QmlpackError(
                "GitHub returned invalid repository identity".into(),
            ));
        }

        let reference = source
            .release_tag()
            .unwrap_or_else(|| source.requested.clone());
        let commit: CommitResponse = self.api_json(
            &[
                "repos",
                &source.owner,
                &source.repository,
                "commits",
                &reference,
            ],
            API_JSON_LIMIT,
        )?;
        if !is_sha(&commit.sha) || !is_sha(&commit.commit.tree.sha) {
            return Err(QmlpackError(
                "GitHub returned invalid commit metadata".into(),
            ));
        }

        let package_tree = self.descend_tree(
            &source.owner,
            &source.repository,
            &commit.commit.tree.sha,
            &source.package_path,
        )?;
        let manifest_entry = self.file_entry(
            &source.owner,
            &source.repository,
            &package_tree,
            "qmlpack.json",
        )?;
        if manifest_entry.mode != "100644" {
            return Err(QmlpackError(
                "qmlpack.json must be a regular non-executable file".into(),
            ));
        }
        let manifest_bytes = self.blob(
            &source.owner,
            &source.repository,
            &manifest_entry,
            MANIFEST_LIMIT,
        )?;
        let manifest = PackageManifest::parse(&manifest_bytes)?;

        let mut files = Vec::with_capacity(manifest.files.len());
        for path in &manifest.files {
            let entry = self.file_entry(&source.owner, &source.repository, &package_tree, path)?;
            let executable = manifest.executables.contains(path);
            let expected_mode = if executable { "100755" } else { "100644" };
            if entry.mode != expected_mode {
                return Err(QmlpackError(format!(
                    "source mode for {path} is {}, expected {expected_mode}",
                    entry.mode
                )));
            }
            files.push(PackageFile {
                path: path.clone(),
                content: self.blob(&source.owner, &source.repository, &entry, FILE_LIMIT)?,
                executable,
            });
        }
        let digest = crate::package_digest(&manifest, &files)?;
        Ok(ResolvedPackage {
            source,
            repository_id: repository.id,
            repository_name: repository.full_name,
            commit: commit.sha.to_ascii_lowercase(),
            manifest,
            files,
            digest,
        })
    }

    fn descend_tree(
        &mut self,
        owner: &str,
        repository: &str,
        root_sha: &str,
        path: &str,
    ) -> Result<String, QmlpackError> {
        let mut tree_sha = root_sha.to_owned();
        if path.is_empty() {
            return Ok(tree_sha);
        }
        for component in path.split('/') {
            let entries = self.tree(owner, repository, &tree_sha)?;
            let entry = entries
                .iter()
                .find(|entry| entry.path == component)
                .ok_or_else(|| QmlpackError(format!("package directory not found: {path}")))?;
            if entry.kind != "tree" || entry.mode != "040000" || !is_sha(&entry.sha) {
                return Err(QmlpackError(format!(
                    "package path is not a directory: {path}"
                )));
            }
            tree_sha.clone_from(&entry.sha);
        }
        Ok(tree_sha)
    }

    fn file_entry(
        &mut self,
        owner: &str,
        repository: &str,
        root_sha: &str,
        path: &str,
    ) -> Result<TreeEntry, QmlpackError> {
        let mut components = path.split('/').peekable();
        let mut tree_sha = root_sha.to_owned();
        while let Some(component) = components.next() {
            let entries = self.tree(owner, repository, &tree_sha)?;
            let entry = entries
                .iter()
                .find(|entry| entry.path == component)
                .ok_or_else(|| QmlpackError(format!("declared file not found: {path}")))?;
            if components.peek().is_some() {
                if entry.kind != "tree" || entry.mode != "040000" || !is_sha(&entry.sha) {
                    return Err(QmlpackError(format!(
                        "declared path is not a directory: {path}"
                    )));
                }
                tree_sha.clone_from(&entry.sha);
            } else {
                if entry.kind != "blob" || !matches!(entry.mode.as_str(), "100644" | "100755") {
                    return Err(QmlpackError(format!(
                        "declared path is not a regular file: {path}"
                    )));
                }
                return Ok(entry.clone());
            }
        }
        Err(QmlpackError(format!("invalid empty file path: {path}")))
    }

    fn tree(
        &mut self,
        owner: &str,
        repository: &str,
        sha: &str,
    ) -> Result<Vec<TreeEntry>, QmlpackError> {
        if let Some(entries) = self.trees.get(sha) {
            return Ok(entries.clone());
        }
        let response: TreeResponse = self.api_json(
            &["repos", owner, repository, "git", "trees", sha],
            API_JSON_LIMIT,
        )?;
        if response.truncated || response.tree.len() > TREE_ENTRIES_LIMIT {
            return Err(QmlpackError(
                "Git tree exceeds the bounded directory envelope".into(),
            ));
        }
        let mut names = BTreeSet::new();
        for entry in &response.tree {
            if entry.path.is_empty()
                || entry.path.contains('/')
                || entry.path.bytes().any(|byte| byte < 32)
                || !is_sha(&entry.sha)
                || !names.insert(entry.path.clone())
            {
                return Err(QmlpackError(
                    "Git tree contains invalid or duplicate entries".into(),
                ));
            }
        }
        self.trees.insert(sha.to_owned(), response.tree.clone());
        Ok(response.tree)
    }

    fn blob(
        &self,
        owner: &str,
        repository: &str,
        entry: &TreeEntry,
        limit: usize,
    ) -> Result<Vec<u8>, QmlpackError> {
        let size = entry
            .size
            .ok_or_else(|| QmlpackError("Git blob is missing its size".into()))?;
        if size > limit as u64 {
            return Err(QmlpackError(format!("Git blob exceeds {limit} bytes")));
        }
        let encoded_limit = limit.saturating_mul(4).div_ceil(3) + 64 * 1024;
        let response: BlobResponse = self.api_json(
            &["repos", owner, repository, "git", "blobs", &entry.sha],
            encoded_limit,
        )?;
        if response.encoding != "base64" || response.size != size {
            return Err(QmlpackError(
                "GitHub returned inconsistent blob metadata".into(),
            ));
        }
        let compact: String = response
            .content
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        let content = BASE64
            .decode(compact)
            .map_err(|_| QmlpackError("GitHub returned invalid Base64 blob content".into()))?;
        if content.len() as u64 != size || content.len() > limit {
            return Err(QmlpackError(
                "decoded Git blob size does not match metadata".into(),
            ));
        }
        Ok(content)
    }

    fn api_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &[&str],
        limit: usize,
    ) -> Result<T, QmlpackError> {
        let mut url = Url::parse("https://api.github.com/")
            .map_err(|error| QmlpackError(error.to_string()))?;
        url.path_segments_mut()
            .map_err(|_| QmlpackError("cannot construct GitHub API URL".into()))?
            .extend(path);
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| QmlpackError(format!("GitHub request failed: {error}")))?;
        let bytes = bounded_response(response, limit)?;
        strict_json(&bytes, limit)
    }
}

fn bounded_response(mut response: Response, limit: usize) -> Result<Vec<u8>, QmlpackError> {
    let status = response.status();
    if !status.is_success() {
        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .to_owned();
        let mut message = Vec::new();
        response
            .by_ref()
            .take(4097)
            .read_to_end(&mut message)
            .map_err(|error| QmlpackError(error.to_string()))?;
        let detail = String::from_utf8_lossy(&message[..message.len().min(4096)]);
        return Err(QmlpackError(format!(
            "GitHub returned {status} (rate remaining: {remaining}): {}",
            detail.trim()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(QmlpackError(format!(
            "GitHub response exceeds {limit} bytes"
        )));
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| QmlpackError(format!("cannot read GitHub response: {error}")))?;
    if bytes.len() > limit {
        return Err(QmlpackError(format!(
            "GitHub response exceeds {limit} bytes"
        )));
    }
    Ok(bytes)
}

fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Deserialize)]
struct RepositoryResponse {
    id: u64,
    full_name: String,
    private: bool,
}

#[derive(Deserialize)]
struct CommitResponse {
    sha: String,
    commit: CommitDetail,
}

#[derive(Deserialize)]
struct CommitDetail {
    tree: GitObject,
}

#[derive(Deserialize)]
struct GitObject {
    sha: String,
}

#[derive(Deserialize)]
struct TreeResponse {
    #[serde(default)]
    truncated: bool,
    tree: Vec<TreeEntry>,
}

#[derive(Clone, Deserialize)]
struct TreeEntry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
}

#[derive(Deserialize)]
struct BlobResponse {
    content: String,
    encoding: String,
    size: u64,
}
