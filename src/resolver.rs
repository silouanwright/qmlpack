use crate::github::{GitHubClient, ResolvedPackage};
use crate::project::{LockedPackage, Lockfile};
use crate::{DEPENDENCY_DEPTH_LIMIT, PACKAGES_LIMIT, QmlpackError, Source};
use std::collections::{BTreeMap, BTreeSet};

pub struct ResolvedGraph {
    pub packages: BTreeMap<String, ResolvedPackage>,
}

impl ResolvedGraph {
    pub fn lockfile(&self) -> Lockfile {
        let packages = self
            .packages
            .iter()
            .map(|(label, package)| {
                let files = package
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), file.digest()))
                    .collect();
                (
                    label.clone(),
                    LockedPackage {
                        source: format!("github:{}", package.repository_name),
                        repository_id: package.repository_id,
                        repository_name: package.repository_name.clone(),
                        package_path: package.source.package_path.clone(),
                        requested: package.source.requested.clone(),
                        version: package.source.version().map(|version| version.to_string()),
                        tag: package.source.release_tag(),
                        commit: package.commit.clone(),
                        digest: package.digest.clone(),
                        files,
                    },
                )
            })
            .collect();
        Lockfile {
            schema_version: 1,
            packages,
        }
    }
}

pub struct Resolver<'a> {
    client: &'a mut GitHubClient,
    packages: BTreeMap<String, ResolvedPackage>,
    identities: BTreeMap<(u64, String), (String, String)>,
    active: BTreeSet<String>,
}

impl<'a> Resolver<'a> {
    pub fn new(client: &'a mut GitHubClient) -> Self {
        Self {
            client,
            packages: BTreeMap::new(),
            identities: BTreeMap::new(),
            active: BTreeSet::new(),
        }
    }

    pub fn resolve(
        mut self,
        dependencies: &BTreeMap<String, Source>,
    ) -> Result<ResolvedGraph, QmlpackError> {
        for (label, source) in dependencies {
            self.resolve_one(label, source.clone(), 0)?;
        }
        Ok(ResolvedGraph {
            packages: self.packages,
        })
    }

    fn resolve_one(
        &mut self,
        label: &str,
        source: Source,
        depth: usize,
    ) -> Result<(), QmlpackError> {
        if depth > DEPENDENCY_DEPTH_LIMIT {
            return Err(QmlpackError(format!(
                "dependency graph exceeds depth {DEPENDENCY_DEPTH_LIMIT}"
            )));
        }
        if self.packages.len() >= PACKAGES_LIMIT && !self.packages.contains_key(label) {
            return Err(QmlpackError(format!(
                "dependency graph exceeds {PACKAGES_LIMIT} packages"
            )));
        }
        let source_key = source.canonical();
        if !self.active.insert(source_key.clone()) {
            return Err(QmlpackError(format!(
                "dependency cycle through {source_key}"
            )));
        }
        let package = self.client.resolve(source)?;

        let identity = (package.repository_id, package.source.package_path.clone());
        if let Some((existing_label, existing_commit)) = self.identities.get(&identity) {
            if existing_label != label || existing_commit != &package.commit {
                return Err(QmlpackError(format!(
                    "package identity is requested inconsistently as {existing_label} and {label}"
                )));
            }
            self.active.remove(&source_key);
            return Ok(());
        }
        if let Some(existing) = self.packages.get(label) {
            return Err(QmlpackError(format!(
                "dependency label {label} refers to both {} and {}",
                existing.source.canonical(),
                package.source.canonical()
            )));
        }

        self.identities
            .insert(identity, (label.to_owned(), package.commit.clone()));
        let dependencies = package.manifest.dependencies.clone();
        self.packages.insert(label.to_owned(), package);
        let result = dependencies
            .into_iter()
            .try_for_each(|(child_label, child_source)| {
                self.resolve_one(&child_label, child_source, depth + 1)
            });
        self.active.remove(&source_key);
        result
    }
}
