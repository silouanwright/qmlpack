# Qmlpack

Qmlpack turns ad hoc QML source copying into a traceable, reproducible, and
reviewable dependency workflow. It supports portable QML, Quickshell code, and
Omarchy-aware packages, with Omarchy as its first fully validated host.

Omarchy plugins are self-contained Git repositories today. That makes shared
QML components and utilities easy to copy but difficult to identify, update,
or audit later. Qmlpack vendors declared source files into a plugin and records
their exact origin, commit, and content digest.

Qmlpack verifies **what** you installed. It does not certify that third-party
code is safe. Review every package before installation and every change before
an update, manually or with an AI coding agent. AI review is useful but is not
a security guarantee.

## Status

Qmlpack is under active development and is not affiliated with Qt, Quickshell,
npm, or Omarchy. Its first supported host profile is Omarchy 4.x. Portable QML
and Quickshell packages use the same source envelope without claiming support
for every standalone shell's integration conventions. Bounded GitHub and public
npm transports are implemented. Publishing the first npm packages remains an
account-level release step.

## Design

- npm is the preferred release registry in the accepted design; exact GitHub
  sources are already supported and remain first-class.
- Packages declare an explicit, bounded list of files.
- npm releases use immutable scoped name/version pairs; GitHub packages may use
  exact commits or package-prefixed SemVer tags.
- Lockfiles pin transport identity, resolved source, integrity, and canonical
  SHA-256.
- Installed source is committed under `vendor/qmlpack/` with the plugin.
- Adds and updates stop for inspection before changing the working tree.
- There are no install hooks or automatic updates.

Qmlpack does not operate a package registry. Authors may publish immutable
releases to npm or use fully qualified GitHub sources. Qmlpack talks to those
services directly without invoking `npm install` or executing package hooks.

Committed vendoring is also a compatibility strategy for Omarchy's current
plugin contract, not an assumption that must last forever. If
`omarchy plugin add` later installs declared dependencies itself, Qmlpack can
resolve and lock packages for the host installer instead of committing their
source into each plugin. Existing manifests and lockfiles remain useful in
either model.

The normative contracts live in:

- [Package specification](docs/package-format.md)
- [Threat model](docs/threat-model.md)
- [Review policy](docs/review-policy.md)
- [Architecture decisions](docs/adr/README.md)
- [Research synthesis and primary sources](docs/research.md)

## Workflow

Build and install the current checkout with the standard Rust toolchain:

```bash
cargo install --path .
```

```bash
qmlpack init
qmlpack add oma-ui github:silouanwright/omatools/packages/oma-ui@0.2.0
qmlpack diff
qmlpack apply
qmlpack verify
```

Package authors can validate an independently releasable package before tagging
or publishing it:

```bash
qmlpack release-check packages/oma-ui
```

See [Release handoff](docs/releasing.md) for the remaining GitHub and npm
publication steps. Qmlpack never performs those credentialed actions itself.

## License

MIT
