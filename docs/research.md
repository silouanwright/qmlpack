# Research Synthesis

Research completed on 2026-08-24 before implementation.

## Ecosystem finding

No maintained Quickshell-specific or Omarchy-specific source package manager
was found. Quickshell projects generally keep reusable QML, JavaScript, assets,
and services inside complete shell repositories or copy them between configs.
Omarchy 4.x adds a real plugin lifecycle and marketplace, but its schema version
1 has no dependency field and rejects symbolic links anywhere in installed
plugins. Consumers therefore need self-contained plugin repositories today.

The nearest historical precedent is qpm, a now-dormant Qt/QML source package
manager. qpm stored metadata centrally while source remained in author-owned
repositories, created a project manifest, resolved nested dependencies, and
vendored source. Omapack retains the source-package insight while avoiding a
required central service and the build-system concerns of compiled Qt apps.

## Product boundary

Omarchy is the first product boundary because it supplies concrete plugin,
theme, service, settings, validation, and installation contracts. Quickshell is
the runtime beneath it, but standalone Quickshell shells do not share one host
API or directory convention. The core package envelope remains portable; a
general Quickshell claim waits for a real second consumer.

## Compared systems

### shadcn

Useful precedents are explicit registry files and targets, source ownership by
the consuming project, GitHub-hosted registries, and `view`, `diff`, and
`dry-run` before mutation. Its model demonstrates that source distribution does
not require a binary artifact registry.

### npm

The useful distinction is dependency intent versus an exact committed lock.
Lifecycle scripts demonstrate a trust boundary Omapack deliberately avoids.
npm can make `package.json` versions authoritative because publication reserves
an immutable name/version pair in a central registry; a Git-only system cannot
make that same claim from a mutable manifest alone.

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
not fit committed, symlink-free Omarchy plugins. No JavaScript monorepo runner
is required for a small standard-library Python CLI and source packages.

### Swift Package Manager and modern QML modules

SwiftPM confirms that Git tags and exact revisions can coexist as dependency
requirements. Qt's QML module system supplies namespaced runtime imports and
tooling but not discovery or retrieval of author-owned source. Omapack should
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
- [Cargo registries](https://doc.rust-lang.org/cargo/reference/registries.html)
- [Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [Go module reference](https://go.dev/ref/mod)
- [pnpm workspaces](https://pnpm.io/workspaces)
- [Swift package dependencies](https://github.com/swiftlang/swift-package-manager/blob/main/Sources/PackageManagerDocs/Documentation.docc/Dependencies/AddingDependencies.md)
- [Qt QML modules](https://doc.qt.io/qt-6/qtqml-modules-topic.html)
- [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub REST rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)
- [Git trees API](https://docs.github.com/en/rest/git/trees)
