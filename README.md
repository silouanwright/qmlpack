# Omapack

Omapack turns ad hoc source copying into a traceable, reproducible, and
reviewable workflow for Omarchy plugin development.

Omarchy plugins are self-contained Git repositories today. That makes shared
QML components and utilities easy to copy but difficult to identify, update,
or audit later. Omapack vendors declared source files into a plugin and records
their exact origin, commit, and content digest.

Omapack verifies **what** you installed. It does not certify that third-party
code is safe. Review every package before installation and every change before
an update, manually or with an AI coding agent. AI review is useful but is not
a security guarantee.

## Status

Omapack is under active development. Its first supported profile is Omarchy
4.x. The package format is intentionally source-oriented, but standalone
Quickshell support is not yet promised.

## Design

- GitHub repositories are the initial source and distribution layer.
- Packages declare an explicit, bounded list of files.
- Releases use package-prefixed SemVer Git tags such as `oma-ui/v0.2.0`.
- Lockfiles pin the stable repository ID, tag, commit, and canonical SHA-256.
- Installed source is committed under `vendor/omapack/` with the plugin.
- Adds and updates stop for inspection before changing the working tree.
- There are no install hooks or automatic updates.

The initial ecosystem is decentralized: authors publish from their own GitHub
repositories and consumers use fully qualified sources. If adoption warrants
it, searchable or curated indexes can be layered on top without becoming the
authority for package contents.

Committed vendoring is also a compatibility strategy for Omarchy's current
plugin contract, not an assumption that must last forever. If
`omarchy plugin add` later installs declared dependencies itself, Omapack can
resolve and lock packages for the host installer instead of committing their
source into each plugin. Existing manifests and lockfiles remain useful in
either model.

The normative contracts live in:

- [Package specification](docs/package-format.md)
- [Threat model](docs/threat-model.md)
- [Review policy](docs/review-policy.md)
- [Architecture decisions](docs/adr/README.md)

## Planned first workflow

```bash
omapack init
omapack add github:silouanwright/omatools/oma-ui@v0.1.0
omapack diff oma-ui
omapack add github:silouanwright/omatools/oma-ui@v0.1.0 --apply
omapack verify
```

The exact command contract will be stabilized alongside the first end-to-end
LookElsewhere integration.

## License

MIT
