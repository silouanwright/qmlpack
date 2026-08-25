use crate::project::{LockedResolution, Lockfile, ProjectManifest};
use crate::resolver::ResolvedGraph;
use crate::{MANIFEST_LIMIT, PackageFile, PackageManifest, QmlpackError, package_digest};
use similar::TextDiff;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::Builder;

const PROJECT_FILE: &str = "qmlpack.json";
const LOCK_FILE: &str = "qmlpack.lock";
const STATE_DIR: &str = ".qmlpack";
const CANDIDATE_DIR: &str = "candidate";
const VENDOR_DIR: &str = "vendor/qmlpack";

pub fn initialize(root: &Path, profile: &str) -> Result<(), QmlpackError> {
    if !matches!(profile, "qml" | "quickshell" | "omarchy") {
        return Err(QmlpackError(
            "profile must be qml, quickshell, or omarchy".into(),
        ));
    }
    fs::create_dir_all(root)?;
    let path = root.join(PROJECT_FILE);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                QmlpackError(format!("{} already exists", path.display()))
            } else {
                error.into()
            }
        })?;
    file.write_all(
        &ProjectManifest {
            profile: profile.into(),
            dependencies: BTreeMap::new(),
        }
        .to_json()?,
    )?;
    file.sync_all()?;
    Ok(())
}

pub fn read_project(root: &Path) -> Result<ProjectManifest, QmlpackError> {
    let path = root.join(PROJECT_FILE);
    let bytes = read_bounded(&path, MANIFEST_LIMIT)?;
    ProjectManifest::parse(&bytes)
}

pub fn read_lock(root: &Path) -> Result<Option<Lockfile>, QmlpackError> {
    let path = root.join(LOCK_FILE);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    Lockfile::parse(&read_bounded(&path, 4 * MANIFEST_LIMIT)?).map(Some)
}

pub fn release_check(root: &Path) -> Result<PackageManifest, QmlpackError> {
    let manifest =
        PackageManifest::parse(&read_bounded(&root.join(PROJECT_FILE), MANIFEST_LIMIT)?)?;
    for path in &manifest.files {
        let source = root.join(path);
        let metadata = fs::symlink_metadata(&source)?;
        if !metadata.file_type().is_file() {
            return Err(QmlpackError(format!(
                "declared package file is not a regular file: {path}"
            )));
        }
        let executable = metadata.permissions().mode() & 0o111 != 0;
        if executable != manifest.executables.contains(path) {
            return Err(QmlpackError(format!(
                "declared executable mode does not match the file: {path}"
            )));
        }
        read_bounded(&source, crate::FILE_LIMIT)?;
    }
    Ok(manifest)
}

pub fn prepare(
    root: &Path,
    project: &ProjectManifest,
    graph: &ResolvedGraph,
) -> Result<String, QmlpackError> {
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
            &destination.join(".qmlpack-package.json"),
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
    let review = review_markdown(
        root,
        project,
        graph,
        &lock,
        &vendor,
        &state.join(CANDIDATE_DIR).join(VENDOR_DIR),
    )?;
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

pub fn candidate_review(root: &Path) -> Result<String, QmlpackError> {
    let path = root.join(STATE_DIR).join(CANDIDATE_DIR).join("review.md");
    String::from_utf8(read_bounded(&path, 2 * 1024 * 1024)?)
        .map_err(|error| QmlpackError(format!("candidate review is not UTF-8: {error}")))
}

pub fn apply(root: &Path, force: bool) -> Result<(), QmlpackError> {
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
        Ok::<_, QmlpackError>(())
    })();
    if result.is_err() {
        let _ = recover(root);
    }
    result
}

pub fn verify(root: &Path) -> Result<(), QmlpackError> {
    recover(root)?;
    verify_installed(root, false)
}

fn verify_installed(root: &Path, allow_modified: bool) -> Result<(), QmlpackError> {
    let Some(lock) = read_lock(root)? else {
        if root.join(VENDOR_DIR).exists() && !allow_modified {
            return Err(QmlpackError(
                "vendor/qmlpack exists without an qmlpack.lock".into(),
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

fn verify_tree(vendor: &Path, lock: &Lockfile) -> Result<(), QmlpackError> {
    let expected_labels: BTreeSet<_> = lock.packages.keys().cloned().collect();
    let actual_labels = directory_names(vendor)?;
    if actual_labels != expected_labels {
        return Err(QmlpackError(
            "installed package directories do not match the lockfile".into(),
        ));
    }
    for (label, package) in &lock.packages {
        let package_root = vendor.join(label);
        let manifest_bytes =
            read_bounded(&package_root.join(".qmlpack-package.json"), MANIFEST_LIMIT)?;
        let manifest = PackageManifest::parse(&manifest_bytes)?;
        let actual_paths = regular_file_paths(&package_root)?;
        let mut expected_paths: BTreeSet<_> = package.files.keys().cloned().collect();
        expected_paths.insert(".qmlpack-package.json".into());
        if actual_paths != expected_paths {
            return Err(QmlpackError(format!(
                "installed files for {label} do not match the lockfile"
            )));
        }
        let mut files = Vec::new();
        for path in &manifest.files {
            let content = read_bounded(&package_root.join(path), crate::FILE_LIMIT)?;
            let file = PackageFile {
                path: path.clone(),
                content,
                executable: manifest.executables.contains(path),
            };
            let expected = package
                .files
                .get(path)
                .ok_or_else(|| QmlpackError(format!("lock is missing {label}/{path}")))?;
            if &file.digest() != expected {
                return Err(QmlpackError(format!(
                    "locally modified managed file: {label}/{path}"
                )));
            }
            files.push(file);
        }
        if package_digest(&manifest, &files)? != package.digest {
            return Err(QmlpackError(format!("package digest mismatch: {label}")));
        }
    }
    Ok(())
}

pub fn recover(root: &Path) -> Result<(), QmlpackError> {
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
    project: &ProjectManifest,
    graph: &ResolvedGraph,
    lock: &Lockfile,
    candidate_vendor: &Path,
    displayed_vendor: &Path,
) -> Result<String, QmlpackError> {
    let previous = read_lock(root)?.unwrap_or_else(Lockfile::empty);
    let mut output = String::from(
        "# qmlpack review\n\nIntegrity is verified; package safety is not. Inspect all source before applying.\n\n",
    );
    for (label, package) in &lock.packages {
        let status = match previous.packages.get(label) {
            None => "added",
            Some(old) if old.digest == package.digest => "unchanged",
            Some(_) => "updated",
        };
        let directness = if project.dependencies.contains_key(label) {
            "direct"
        } else {
            "transitive"
        };
        output.push_str(&format!(
            "## {label} ({status}, {directness})\n\n- Source: `{}`\n",
            package.source
        ));
        match &package.resolution {
            LockedResolution::Github {
                requested,
                tag,
                commit,
                ..
            } => output.push_str(&format!(
                "- Requested: `{requested}`\n- Tag: `{}`\n- Commit: `{commit}`\n",
                tag.as_deref().unwrap_or("none")
            )),
            LockedResolution::Npm {
                version, integrity, ..
            } => output.push_str(&format!(
                "- Version: `{version}`\n- Registry integrity: `{integrity}`\n"
            )),
        }
        output.push_str(&format!("- Digest: `{}`\n", package.digest));
        let resolved = graph
            .packages
            .get(label)
            .ok_or_else(|| QmlpackError(format!("resolved graph is missing package {label}")))?;
        output.push_str(&format!("- License: `{}`\n", resolved.manifest.license));
        if !resolved.manifest.compatibility.is_empty() {
            let compatibility = resolved
                .manifest
                .compatibility
                .iter()
                .map(|(host, requirement)| format!("{host} {requirement}"))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!("- Compatibility: `{compatibility}`\n"));
        }
        output.push_str(&format!(
            "- Dependencies: {}\n- Executables: {}\n- Files: {}\n\n",
            resolved.manifest.dependencies.len(),
            resolved.manifest.executables.len(),
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
    append_source_diffs(&mut output, &root.join(VENDOR_DIR), candidate_vendor, lock)?;
    output.push_str(&format!(
        "Candidate source: `{}`\n",
        displayed_vendor.display()
    ));
    Ok(output)
}

fn append_source_diffs(
    output: &mut String,
    installed: &Path,
    candidate: &Path,
    lock: &Lockfile,
) -> Result<(), QmlpackError> {
    const TEXT_FILE_LIMIT: usize = 256 * 1024;
    const REVIEW_LIMIT: usize = 2 * 1024 * 1024;
    output.push_str("# Source changes\n\n");
    let previous = read_lock(
        installed
            .parent()
            .and_then(Path::parent)
            .unwrap_or(installed),
    )?
    .unwrap_or_else(Lockfile::empty);
    let labels: BTreeSet<_> = previous
        .packages
        .keys()
        .chain(lock.packages.keys())
        .cloned()
        .collect();
    for label in labels {
        let old_files = previous
            .packages
            .get(&label)
            .map(|package| package.files.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let new_files = lock
            .packages
            .get(&label)
            .map(|package| package.files.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        for path in old_files.union(&new_files) {
            let old = fs::read(installed.join(&label).join(path)).unwrap_or_default();
            let new = fs::read(candidate.join(&label).join(path)).unwrap_or_default();
            if old == new {
                continue;
            }
            output.push_str(&format!("## `{label}/{path}`\n\n"));
            match (
                old.len() <= TEXT_FILE_LIMIT,
                new.len() <= TEXT_FILE_LIMIT,
                std::str::from_utf8(&old),
                std::str::from_utf8(&new),
            ) {
                (true, true, Ok(old_text), Ok(new_text)) => {
                    let diff = TextDiff::from_lines(old_text, new_text)
                        .unified_diff()
                        .context_radius(3)
                        .header(&format!("a/{label}/{path}"), &format!("b/{label}/{path}"))
                        .to_string();
                    output.push_str("```diff\n");
                    output.push_str(&diff);
                    output.push_str("```\n\n");
                }
                _ => output
                    .push_str("Binary or large file changed; inspect the candidate directly.\n\n"),
            }
            if output.len() > REVIEW_LIMIT {
                output.truncate(REVIEW_LIMIT);
                output.push_str(
                    "\n\nReview truncated at 2 MiB; inspect the candidate directory directly.\n\n",
                );
                return Ok(());
            }
        }
    }
    Ok(())
}

fn write_package_file(root: &Path, file: &PackageFile) -> Result<(), QmlpackError> {
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

fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), QmlpackError> {
    let parent = path
        .parent()
        .ok_or_else(|| QmlpackError("path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = Builder::new()
        .prefix(".qmlpack-write-")
        .tempfile_in(parent)?;
    temporary
        .as_file_mut()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    temporary.write_all(bytes)?;
    temporary.as_file_mut().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| QmlpackError(error.error.to_string()))?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn ensure_real_directory(path: &Path) -> Result<(), QmlpackError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(QmlpackError(format!(
            "{} must be a real directory",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(Into::into)
        }
        Err(error) => Err(error.into()),
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, QmlpackError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(QmlpackError(format!(
            "{} is not a bounded regular file",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(QmlpackError(format!(
            "{} exceeds {limit} bytes",
            path.display()
        )));
    }
    Ok(bytes)
}

fn directory_names(path: &Path) -> Result<BTreeSet<String>, QmlpackError> {
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    fs::read_dir(path)?
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                return Err(QmlpackError(format!(
                    "unexpected entry in {}",
                    path.display()
                )));
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| QmlpackError("non-UTF-8 package directory".into()))
        })
        .collect()
}

fn regular_file_paths(root: &Path) -> Result<BTreeSet<String>, QmlpackError> {
    fn walk(
        root: &Path,
        current: &Path,
        output: &mut BTreeSet<String>,
    ) -> Result<(), QmlpackError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
                return Err(QmlpackError(format!(
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
                    .map_err(|error| QmlpackError(error.to_string()))?
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

fn remove_tree_if_present(path: &Path) -> Result<(), QmlpackError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            fs::remove_dir_all(path).map_err(Into::into)
        }
        Ok(_) => Err(QmlpackError(format!(
            "refusing to remove non-directory {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn copy_if_present(source: &Path, destination: &Path) -> Result<(), QmlpackError> {
    match fs::copy(source, destination) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn restore_or_remove(backup: &Path, destination: &Path) -> Result<(), QmlpackError> {
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
    use crate::resolver::ResolvedGraph;
    use crate::{Resolution, ResolvedPackage, Source};
    use std::os::unix::fs::symlink;

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
                    resolution: Resolution::GitHub {
                        repository_id: 42,
                        repository_name: "silouanwright/omatools".into(),
                        package_path: "oma-ui".into(),
                        requested: "1.0.0".into(),
                        version: Some("1.0.0".into()),
                        tag: Some("oma-ui/v1.0.0".into()),
                        commit: "a".repeat(40),
                    },
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
            profile: "omarchy".into(),
            dependencies: BTreeMap::from([(
                "oma-ui".into(),
                Source::parse("github:silouanwright/omatools/oma-ui@1.0.0").unwrap(),
            )]),
        };
        initialize(root.path(), "omarchy").unwrap();
        let initial_review = prepare(root.path(), &project, &graph(b"first\n")).unwrap();
        assert!(initial_review.contains("+first"));
        assert!(initial_review.contains(".qmlpack/candidate/vendor/qmlpack"));
        assert!(!initial_review.contains("candidate-"));
        assert!(!root.path().join(VENDOR_DIR).exists());
        apply(root.path(), false).unwrap();
        verify(root.path()).unwrap();

        fs::write(
            root.path().join(VENDOR_DIR).join("oma-ui/Ui/Button.qml"),
            b"local edit\n",
        )
        .unwrap();
        assert!(verify(root.path()).is_err());
        let update_review = prepare(root.path(), &project, &graph(b"second\n")).unwrap();
        assert!(update_review.contains("-local edit"));
        assert!(update_review.contains("+second"));
        assert!(apply(root.path(), false).is_err());
        apply(root.path(), true).unwrap();
        verify(root.path()).unwrap();
    }

    #[test]
    fn release_check_rejects_undeclared_executable_mode() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(PROJECT_FILE),
            br#"{"schemaVersion":1,"name":"sample","license":"MIT","files":["tool"]}"#,
        )
        .unwrap();
        fs::write(root.path().join("tool"), b"#!/bin/sh\n").unwrap();
        fs::set_permissions(root.path().join("tool"), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(release_check(root.path()).is_err());
    }

    #[test]
    fn bounded_reads_reject_symlinks_and_recovery_restores_backup() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("real-lock"),
            Lockfile::empty().to_json().unwrap(),
        )
        .unwrap();
        symlink("real-lock", root.path().join(LOCK_FILE)).unwrap();
        assert!(read_lock(root.path()).is_err());

        fs::remove_file(root.path().join(LOCK_FILE)).unwrap();
        fs::create_dir_all(root.path().join(VENDOR_DIR)).unwrap();
        fs::write(root.path().join(VENDOR_DIR).join("partial"), b"partial").unwrap();
        let backup = root.path().join(STATE_DIR).join("transaction-backup");
        fs::create_dir_all(backup.join("vendor/package")).unwrap();
        fs::write(backup.join("vendor/package/restored"), b"restored").unwrap();
        fs::write(backup.join(PROJECT_FILE), b"project").unwrap();
        fs::write(backup.join(LOCK_FILE), b"lock").unwrap();
        fs::write(root.path().join(STATE_DIR).join("transaction.json"), b"{}").unwrap();

        recover(root.path()).unwrap();
        assert_eq!(
            fs::read(root.path().join(PROJECT_FILE)).unwrap(),
            b"project"
        );
        assert_eq!(fs::read(root.path().join(LOCK_FILE)).unwrap(), b"lock");
        assert_eq!(
            fs::read(root.path().join(VENDOR_DIR).join("package/restored")).unwrap(),
            b"restored"
        );
        assert!(
            !root
                .path()
                .join(STATE_DIR)
                .join("transaction.json")
                .exists()
        );
    }
}
