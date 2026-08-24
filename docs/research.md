# Research Synthesis

Research completed on 2026-08-24 before implementation.

## Ecosystem finding

No maintained Quickshell-specific or Omarchy-specific source package manager
was found. Quickshell projects generally keep reusable QML, JavaScript, assets,
and services inside complete shell repositories or copy them between configs.
Omarchy 4.x adds a real plugin lifecycle and marketplace, but its schema version
1 has no dependency field and rejects symbolic links anywhere in installed
plugins. Consumers therefore need self-contained plugin repositories today.

The nearest historical precedent is qpm, an archived Qt/QML source package
manager. qpm stored metadata in its own service while source remained in
author-owned repositories, created a project manifest, resolved nested
dependencies, and vendored source. Its service was later shut down. Qmlpack
retains the source-package insight without operating registry infrastructure
or inheriting the build-system concerns of compiled Qt applications.

## Product boundary

QML modules are the reusable source boundary. A package may be portable Qt/QML,
depend on Quickshell, or additionally depend on Omarchy APIs and design tokens.
Omarchy is the first validated host because it supplies concrete plugin, theme,
service, settings, validation, and installation contracts. Compatibility is
declared per package rather than guessed from source or annotated per file.

## Compared systems

### shadcn

Useful precedents are explicit registry files and targets, source ownership by
the consuming project, GitHub-hosted registries, and `view`, `diff`, and
`dry-run` before mutation. Its model demonstrates that source distribution does
not require a binary artifact registry.

### npm

The useful distinctions are dependency intent versus an exact committed lock,
scoped publisher namespaces, registry integrity, and immutable name/version
pairs. npm accepts packages of computer code rather than requiring JavaScript
entry points, making real QML modules suitable package contents under its
acceptable-use policy. Lifecycle scripts demonstrate a trust boundary Qmlpack
deliberately avoids: Qmlpack reads registry metadata and bounded tarballs
directly and never invokes `npm install`.

### Cargo

Useful precedents are checksums, immutable published versions, yanking without
rewriting existing locks, and release validation. A global namespace, artifact
registry, and general resolver are unnecessary for the initial problem.

### Go modules

Go most closely matches the decentralized release model: source repository and
subdirectory form module identity, package-path-prefixed Git tags independently
version modules in monorepos, exact revisions remain installable, vendoring is
checked for consistency, and package archives enforce strict path, size, and
special-file rules. Proxies and a checksum database are useful future layers,
not prerequisites for source identity.

### pnpm workspaces

Workspaces are useful for developing multiple independently released packages
in one repository. pnpm's content-addressed store and symlink/hardlink layout do
not fit committed, symlink-free Omarchy plugins. npm may publish the package
artifacts without making npm or pnpm the QML materializer.

### Swift Package Manager and modern QML modules

SwiftPM confirms that Git tags and exact revisions can coexist as dependency
requirements. Qt's QML module system supplies namespaced runtime imports and
tooling but not discovery or retrieval of author-owned source. Qmlpack should
distribute valid QML modules rather than inventing another runtime module
format.

## GitHub findings

- Stable numeric repository IDs help distinguish transfer from name reuse.
- Exact commits provide a retrieval snapshot, while an independent SHA-256
  describes the materialized package envelope.
- Immutable releases and release-asset digests can strengthen publisher
  guarantees but must not be required for decentralized packages.
- API responses may be rate-limited, fail, or report truncated trees; bounded
  direct file retrieval avoids depending on a complete untrusted repository
  tree.
- Repository topics can support future discovery but are eventually indexed,
  spoofable metadata and should not be part of installation resolution.

## Primary references

- [Omarchy source and plugin validator](https://github.com/basecamp/omarchy)
- [Quickshell](https://github.com/quickshell-mirror/quickshell)
- [Quickshell repositories on GitHub](https://github.com/topics/quickshell)
- [qpm](https://github.com/Cutehacks/qpm)
- [shadcn CLI](https://ui.shadcn.com/docs/cli)
- [shadcn registry schema](https://ui.shadcn.com/docs/registry/registry-json)
- [GitHub-backed shadcn registries](https://ui.shadcn.com/docs/registry/github)
- [npm lockfiles](https://docs.npmjs.com/files/package-lock.json/)
- [npm lifecycle scripts](https://docs.npmjs.com/cli/using-npm/scripts/)
- [npm publish](https://docs.npmjs.com/cli/commands/npm-publish/)
- [npm open-source terms](https://docs.npmjs.com/policies/open-source-terms/)
- [npm unpublish and immutability policy](https://docs.npmjs.com/policies/unpublish/)
- [Cargo registries](https://doc.rust-lang.org/cargo/reference/registries.html)
- [Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [Go module reference](https://go.dev/ref/mod)
- [pnpm workspaces](https://pnpm.io/workspaces)
- [Swift package dependencies](https://github.com/swiftlang/swift-package-manager/blob/main/Sources/PackageManagerDocs/Documentation.docc/Dependencies/AddingDependencies.md)
- [Qt QML modules](https://doc.qt.io/qt-6/qtqml-modules-topic.html)
- [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub REST rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)
- [Git trees API](https://docs.github.com/en/rest/git/trees)
