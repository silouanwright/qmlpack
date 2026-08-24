use crate::github::GitHubClient;
use crate::npm::NpmClient;
use crate::project::{LockedPackage, LockedResolution, Lockfile};
use crate::{
    DEPENDENCY_DEPTH_LIMIT, PACKAGES_LIMIT, QmlpackError, Resolution, ResolvedPackage, Source,
};
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
                        source: package.source.canonical(),
                        resolution: match &package.resolution {
                            Resolution::GitHub {
                                repository_id,
                                repository_name,
                                package_path,
                                requested,
                                version,
                                tag,
                                commit,
                            } => LockedResolution::Github {
                                repository_id: *repository_id,
                                repository_name: repository_name.clone(),
                                package_path: package_path.clone(),
                                requested: requested.clone(),
                                version: version.clone(),
                                tag: tag.clone(),
                                commit: commit.clone(),
                            },
                            Resolution::Npm {
                                registry,
                                name,
                                version,
                                integrity,
                            } => LockedResolution::Npm {
                                registry: registry.clone(),
                                name: name.clone(),
                                version: version.clone(),
                                integrity: integrity.clone(),
                            },
                        },
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
    github: &'a mut GitHubClient,
    npm: &'a NpmClient,
    packages: BTreeMap<String, ResolvedPackage>,
    identities: BTreeMap<String, (String, String)>,
    active: BTreeSet<String>,
}

impl<'a> Resolver<'a> {
    pub fn new(github: &'a mut GitHubClient, npm: &'a NpmClient) -> Self {
        Self {
            github,
            npm,
            packages: BTreeMap::new(),
            identities: BTreeMap::new(),
            active: BTreeSet::new(),
        }
    }

    pub fn resolve(
        mut self,
        dependencies: &BTreeMap<String, Source>,
        profile: &str,
    ) -> Result<ResolvedGraph, QmlpackError> {
        for (label, source) in dependencies {
            self.resolve_one(label, source.clone(), profile, 0)?;
        }
        Ok(ResolvedGraph {
            packages: self.packages,
        })
    }

    fn resolve_one(
        &mut self,
        label: &str,
        source: Source,
        profile: &str,
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
        let package = match &source {
            Source::GitHub(_) => self.github.resolve(source)?,
            Source::Npm(_) => self.npm.resolve(source)?,
        };
        validate_profile(profile, &package)?;

        let identity = package.identity();
        if let Some((existing_label, existing_commit)) = self.identities.get(&identity) {
            if existing_label != label || existing_commit != &package.revision() {
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
            .insert(identity, (label.to_owned(), package.revision()));
        let dependencies = package.manifest.dependencies.clone();
        self.packages.insert(label.to_owned(), package);
        let result = dependencies
            .into_iter()
            .try_for_each(|(child_label, child_source)| {
                self.resolve_one(&child_label, child_source, profile, depth + 1)
            });
        self.active.remove(&source_key);
        result
    }
}

fn validate_profile(profile: &str, package: &ResolvedPackage) -> Result<(), QmlpackError> {
    let requires = &package.manifest.compatibility;
    let incompatible = match profile {
        "qml" => requires.contains_key("quickshell") || requires.contains_key("omarchy"),
        "quickshell" => requires.contains_key("omarchy"),
        "omarchy" => false,
        _ => {
            return Err(QmlpackError(format!(
                "unsupported project profile: {profile}"
            )));
        }
    };
    if incompatible {
        return Err(QmlpackError(format!(
            "{} requires a higher-level host than the {profile} project profile",
            package.source.canonical()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageManifest, Resolution};

    fn package(compatibility: &str) -> ResolvedPackage {
        let raw = format!(
            "{{\"schemaVersion\":1,\"name\":\"x\",\"license\":\"MIT\",\"files\":[\"x.qml\"],\"compatibility\":{compatibility}}}"
        );
        ResolvedPackage {
            source: Source::parse("github:o/r@1.0.0").unwrap(),
            resolution: Resolution::GitHub {
                repository_id: 1,
                repository_name: "o/r".into(),
                package_path: String::new(),
                requested: "1.0.0".into(),
                version: Some("1.0.0".into()),
                tag: Some("v1.0.0".into()),
                commit: "a".repeat(40),
            },
            manifest: PackageManifest::parse(raw.as_bytes()).unwrap(),
            files: vec![],
            digest: String::new(),
        }
    }

    #[test]
    fn lower_level_profiles_reject_host_specific_packages() {
        let portable = package("{}");
        let quickshell = package(r#"{"quickshell":">=0.3"}"#);
        let omarchy = package(r#"{"omarchy":">=4"}"#);
        validate_profile("qml", &portable).unwrap();
        assert!(validate_profile("qml", &quickshell).is_err());
        validate_profile("quickshell", &quickshell).unwrap();
        assert!(validate_profile("quickshell", &omarchy).is_err());
        validate_profile("omarchy", &omarchy).unwrap();
    }
}
