use crate::project::{Lockfile, ProjectManifest};
use crate::resolver::ResolvedGraph;
use crate::{MANIFEST_LIMIT, OmapackError, PackageFile, PackageManifest, package_digest};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::Builder;

const PROJECT_FILE: &str = "omapack.json";
const LOCK_FILE: &str = "omapack.lock";
const STATE_DIR: &str = ".omapack";
const CANDIDATE_DIR: &str = "candidate";
const VENDOR_DIR: &str = "vendor/omapack";

pub fn initialize(root: &Path) -> Result<(), OmapackError> {
    fs::create_dir_all(root)?;
    let path = root.join(PROJECT_FILE);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                OmapackError(format!("{} already exists", path.display()))
            } else {
                error.into()
            }
        })?;
    file.write_all(
        &ProjectManifest {
            dependencies: BTreeMap::new(),
        }
        .to_json()?,
    )?;
    file.sync_all()?;
    Ok(())
}

pub fn read_project(root: &Path) -> Result<ProjectManifest, OmapackError> {
    let path = root.join(PROJECT_FILE);
    let bytes = read_bounded(&path, MANIFEST_LIMIT)?;
    ProjectManifest::parse(&bytes)
}

pub fn read_lock(root: &Path) -> Result<Option<Lockfile>, OmapackError> {
    let path = root.join(LOCK_FILE);
    match fs::read(&path) {
        Ok(bytes) if bytes.len() <= 4 * MANIFEST_LIMIT => Lockfile::parse(&bytes).map(Some),
        Ok(_) => Err(OmapackError(format!("{} is too large", path.display()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn prepare(
    root: &Path,
    project: &ProjectManifest,
    graph: &ResolvedGraph,
) -> Result<String, OmapackError> {
    recover(root)?;
    let state = root.join(STATE_DIR);
    ensure_real_directory(&state)?;
    let temporary = Builder::new().prefix("candidate-").tempdir_in(&state)?;
    let temporary_root = temporary.path();
    let vendor = temporary_root.join(VENDOR_DIR);
    fs::create_dir_all(&vendor)?;
    for (label, package) in &graph.packages {
        let destination = vendor.join(label);
        fs::create_dir_all(&destination)?;
        atomic_write(
            &destination.join(".omapack-package.json"),
            &package.manifest.raw,
            0o644,
        )?;
        for file in &package.files {
            write_package_file(&destination, file)?;
        }
    }
    let lock = graph.lockfile();
    atomic_write(
        &temporary_root.join("project.json"),
        &project.to_json()?,
        0o644,
    )?;
    atomic_write(&temporary_root.join(LOCK_FILE), &lock.to_json()?, 0o644)?;
    let review = review_markdown(root, &lock, &vendor)?;
    atomic_write(&temporary_root.join("review.md"), review.as_bytes(), 0o644)?;

    let candidate = state.join(CANDIDATE_DIR);
    let previous = state.join("candidate.previous");
    remove_tree_if_present(&previous)?;
    if candidate.exists() {
        fs::rename(&candidate, &previous)?;
    }
    if let Err(error) = fs::rename(temporary.keep(), &candidate) {
        if previous.exists() {
            let _ = fs::rename(&previous, &candidate);
        }
        return Err(error.into());
    }
    remove_tree_if_present(&previous)?;
    Ok(review)
}

pub fn candidate_review(root: &Path) -> Result<String, OmapackError> {
    fs::read_to_string(root.join(STATE_DIR).join(CANDIDATE_DIR).join("review.md"))
        .map_err(|error| OmapackError(format!("no prepared candidate: {error}")))
}

pub fn apply(root: &Path, force: bool) -> Result<(), OmapackError> {
    recover(root)?;
    verify_installed(root, force)?;
    let state = root.join(STATE_DIR);
    let candidate = state.join(CANDIDATE_DIR);
    let candidate_lock = Lockfile::parse(&read_bounded(
        &candidate.join(LOCK_FILE),
        4 * MANIFEST_LIMIT,
    )?)?;
    verify_tree(&candidate.join(VENDOR_DIR), &candidate_lock)?;

    let backup = state.join("transaction-backup");
    remove_tree_if_present(&backup)?;
    fs::create_dir_all(&backup)?;
    copy_if_present(&root.join(PROJECT_FILE), &backup.join(PROJECT_FILE))?;
    copy_if_present(&root.join(LOCK_FILE), &backup.join(LOCK_FILE))?;
    let old_vendor = backup.join("vendor");
    let had_vendor = root.join(VENDOR_DIR).exists();
    if !had_vendor {
        atomic_write(&backup.join("no-vendor"), b"", 0o600)?;
    }
    let marker = state.join("transaction.json");
    atomic_write(&marker, b"{\"schemaVersion\":1}\n", 0o600)?;
    if had_vendor {
        fs::rename(root.join(VENDOR_DIR), &old_vendor)?;
    }

    let result = (|| {
        fs::create_dir_all(root.join("vendor"))?;
        fs::rename(candidate.join(VENDOR_DIR), root.join(VENDOR_DIR))?;
        atomic_write(
            &root.join(PROJECT_FILE),
            &read_bounded(&candidate.join("project.json"), MANIFEST_LIMIT)?,
            0o644,
        )?;
        atomic_write(&root.join(LOCK_FILE), &candidate_lock.to_json()?, 0o644)?;
        fs::remove_file(&marker)?;
        remove_tree_if_present(&backup)?;
        remove_tree_if_present(&candidate)?;
        Ok::<_, OmapackError>(())
    })();
    if result.is_err() {
        let _ = recover(root);
    }
    result
}

pub fn verify(root: &Path) -> Result<(), OmapackError> {
    recover(root)?;
    verify_installed(root, false)
}

fn verify_installed(root: &Path, allow_modified: bool) -> Result<(), OmapackError> {
    let Some(lock) = read_lock(root)? else {
        if root.join(VENDOR_DIR).exists() && !allow_modified {
            return Err(OmapackError(
                "vendor/omapack exists without an omapack.lock".into(),
            ));
        }
        return Ok(());
    };
    match verify_tree(&root.join(VENDOR_DIR), &lock) {
        Ok(()) => Ok(()),
        Err(_) if allow_modified => Ok(()),
        Err(error) => Err(error),
    }
}

fn verify_tree(vendor: &Path, lock: &Lockfile) -> Result<(), OmapackError> {
    let expected_labels: BTreeSet<_> = lock.packages.keys().cloned().collect();
    let actual_labels = directory_names(vendor)?;
    if actual_labels != expected_labels {
        return Err(OmapackError(
            "installed package directories do not match the lockfile".into(),
        ));
    }
    for (label, package) in &lock.packages {
        let package_root = vendor.join(label);
        let manifest_bytes =
            read_bounded(&package_root.join(".omapack-package.json"), MANIFEST_LIMIT)?;
        let manifest = PackageManifest::parse(&manifest_bytes)?;
        let actual_paths = regular_file_paths(&package_root)?;
        let mut expected_paths: BTreeSet<_> = package.files.keys().cloned().collect();
        expected_paths.insert(".omapack-package.json".into());
        if actual_paths != expected_paths {
            return Err(OmapackError(format!(
                "installed files for {label} do not match the lockfile"
            )));
        }
        let mut files = Vec::new();
        for path in &manifest.files {
            let content = fs::read(package_root.join(path))?;
            let file = PackageFile {
                path: path.clone(),
                content,
                executable: manifest.executables.contains(path),
            };
            let expected = package
                .files
                .get(path)
                .ok_or_else(|| OmapackError(format!("lock is missing {label}/{path}")))?;
            if &file.digest() != expected {
                return Err(OmapackError(format!(
                    "locally modified managed file: {label}/{path}"
                )));
            }
            files.push(file);
        }
        if package_digest(&manifest, &files)? != package.digest {
            return Err(OmapackError(format!("package digest mismatch: {label}")));
        }
    }
    Ok(())
}

pub fn recover(root: &Path) -> Result<(), OmapackError> {
    let state = root.join(STATE_DIR);
    let marker = state.join("transaction.json");
    if !marker.exists() {
        return Ok(());
    }
    let backup = state.join("transaction-backup");
    if backup.join("vendor").exists() {
        remove_tree_if_present(&root.join(VENDOR_DIR))?;
        fs::create_dir_all(root.join("vendor"))?;
        fs::rename(backup.join("vendor"), root.join(VENDOR_DIR))?;
    } else if backup.join("no-vendor").exists() {
        remove_tree_if_present(&root.join(VENDOR_DIR))?;
    }
    restore_or_remove(&backup.join(PROJECT_FILE), &root.join(PROJECT_FILE))?;
    restore_or_remove(&backup.join(LOCK_FILE), &root.join(LOCK_FILE))?;
    fs::remove_file(marker)?;
    remove_tree_if_present(&backup)?;
    Ok(())
}

fn review_markdown(
    root: &Path,
    lock: &Lockfile,
    candidate_vendor: &Path,
) -> Result<String, OmapackError> {
    let previous = read_lock(root)?.unwrap_or_else(Lockfile::empty);
    let mut output = String::from(
        "# Omapack review\n\nIntegrity is verified; package safety is not. Inspect all source before applying.\n\n",
    );
    for (label, package) in &lock.packages {
        let status = match previous.packages.get(label) {
            None => "added",
            Some(old) if old.digest == package.digest => "unchanged",
            Some(_) => "updated",
        };
        output.push_str(&format!(
            "## {label} ({status})\n\n- Source: `{}`\n- Commit: `{}`\n- Digest: `{}`\n- Files: {}\n\n",
            package.source,
            package.commit,
            package.digest,
            package.files.len()
        ));
        for path in package.files.keys() {
            output.push_str(&format!("- `{path}`\n"));
        }
        output.push('\n');
    }
    for label in previous
        .packages
        .keys()
        .filter(|label| !lock.packages.contains_key(*label))
    {
        output.push_str(&format!("## {label} (removed)\n\n"));
    }
    output.push_str(&format!(
        "Candidate source: `{}`\n",
        candidate_vendor.display()
    ));
    Ok(output)
}

fn write_package_file(root: &Path, file: &PackageFile) -> Result<(), OmapackError> {
    let destination = root.join(&file.path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(
        &destination,
        &file.content,
        if file.executable { 0o755 } else { 0o644 },
    )
}

fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), OmapackError> {
    let parent = path
        .parent()
        .ok_or_else(|| OmapackError("path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = Builder::new()
        .prefix(".omapack-write-")
        .tempfile_in(parent)?;
    temporary
        .as_file_mut()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    temporary.write_all(bytes)?;
    temporary.as_file_mut().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| OmapackError(error.error.to_string()))?;
    Ok(())
}

fn ensure_real_directory(path: &Path) -> Result<(), OmapackError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(OmapackError(format!(
            "{} must be a real directory",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(Into::into)
        }
        Err(error) => Err(error.into()),
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, OmapackError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(OmapackError(format!(
            "{} is not a bounded regular file",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    if bytes.len() > limit {
        return Err(OmapackError(format!(
            "{} exceeds {limit} bytes",
            path.display()
        )));
    }
    Ok(bytes)
}

fn directory_names(path: &Path) -> Result<BTreeSet<String>, OmapackError> {
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    fs::read_dir(path)?
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                return Err(OmapackError(format!(
                    "unexpected entry in {}",
                    path.display()
                )));
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| OmapackError("non-UTF-8 package directory".into()))
        })
        .collect()
}

fn regular_file_paths(root: &Path) -> Result<BTreeSet<String>, OmapackError> {
    fn walk(
        root: &Path,
        current: &Path,
        output: &mut BTreeSet<String>,
    ) -> Result<(), OmapackError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
                return Err(OmapackError(format!(
                    "unsafe installed entry: {}",
                    entry.path().display()
                )));
            }
            if kind.is_dir() {
                walk(root, &entry.path(), output)?;
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| OmapackError(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                output.insert(relative);
            }
        }
        Ok(())
    }
    let mut output = BTreeSet::new();
    walk(root, root, &mut output)?;
    Ok(output)
}

fn remove_tree_if_present(path: &Path) -> Result<(), OmapackError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            fs::remove_dir_all(path).map_err(Into::into)
        }
        Ok(_) => Err(OmapackError(format!(
            "refusing to remove non-directory {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn copy_if_present(source: &Path, destination: &Path) -> Result<(), OmapackError> {
    match fs::copy(source, destination) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn restore_or_remove(backup: &Path, destination: &Path) -> Result<(), OmapackError> {
    if backup.exists() {
        fs::copy(backup, destination)?;
    } else if destination.exists() {
        fs::remove_file(destination)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;
    use crate::github::ResolvedPackage;
    use crate::resolver::ResolvedGraph;

    fn graph(content: &[u8]) -> ResolvedGraph {
        let raw =
            br#"{"schemaVersion":1,"name":"oma-ui","license":"MIT","files":["Ui/Button.qml"]}"#;
        let manifest = PackageManifest::parse(raw).unwrap();
        let files = vec![PackageFile {
            path: "Ui/Button.qml".into(),
            content: content.to_vec(),
            executable: false,
        }];
        let digest = package_digest(&manifest, &files).unwrap();
        ResolvedGraph {
            packages: BTreeMap::from([(
                "oma-ui".into(),
                ResolvedPackage {
                    source: Source::parse("github:silouanwright/omatools/oma-ui@1.0.0").unwrap(),
                    repository_id: 42,
                    repository_name: "silouanwright/omatools".into(),
                    commit: "a".repeat(40),
                    manifest,
                    files,
                    digest,
                },
            )]),
        }
    }

    #[test]
    fn prepare_apply_verify_and_modified_file_guard() {
        let root = tempfile::tempdir().unwrap();
        let project = ProjectManifest {
            dependencies: BTreeMap::from([(
                "oma-ui".into(),
                Source::parse("github:silouanwright/omatools/oma-ui@1.0.0").unwrap(),
            )]),
        };
        initialize(root.path()).unwrap();
        prepare(root.path(), &project, &graph(b"first\n")).unwrap();
        assert!(!root.path().join(VENDOR_DIR).exists());
        apply(root.path(), false).unwrap();
        verify(root.path()).unwrap();

        fs::write(
            root.path().join(VENDOR_DIR).join("oma-ui/Ui/Button.qml"),
            b"local edit\n",
        )
        .unwrap();
        assert!(verify(root.path()).is_err());
        prepare(root.path(), &project, &graph(b"second\n")).unwrap();
        assert!(apply(root.path(), false).is_err());
        apply(root.path(), true).unwrap();
        verify(root.path()).unwrap();
    }
}
