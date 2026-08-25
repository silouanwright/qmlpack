# qmlpack Package Format

Status: draft for the first implementation.

## Package identity

A package is identified by its transport-qualified source, not by a local
dependency label:

```text
npm:<scoped-or-unscoped-name>@<exact-version>
github:<owner>/<repository>/<package-path>@<reference>
```

npm names are owned by the registry account or organization scope. GitHub
identity combines its stable numeric repository ID and package path so a
repository transfer is distinguishable from name reuse.

Short names are local display names. A future curated index may supply aliases
without becoming the source of package contents or release authority.

## Discovery and curation

Schema version 1 requires fully qualified package sources. npm provides
discovery for packages published there; GitHub packages remain decentralized.
Additional indexes may later attach search metadata, review status,
compatibility results, and aliases to an exact source identity. They must not
redefine a version or replace the transport identity and digest recorded by the
consumer.

## Manifest

Every package root contains `qmlpack.json`:

```json
{
  "schemaVersion": 1,
  "name": "oma-ui",
  "license": "MIT",
  "files": [
    "Ui/Button.qml",
    "Ui/qmldir",
    "LICENSE"
  ],
  "dependencies": {
    "bounded-read": "github:silouanwright/omatools/bounded-read@v0.1.0"
  },
  "compatibility": {
    "qt": ">=6.8 <7",
    "omarchy": ">=4 <5",
    "quickshell": ">=0.3 <0.4"
  },
  "executables": []
}
```

`version` is deliberately absent. npm's `package.json` owns the version of an
npm release. GitHub releases use package-prefixed SemVer tags, for example
`oma-ui/v0.2.0`. Ordinary commits are valid development snapshots and are not
assigned invented versions.

### Required fields

- `schemaVersion`: JSON integer `1`.
- `name`: a local package name matching `[a-z0-9][a-z0-9-]*`.
- `license`: one SPDX license expression for the package.
- `files`: a non-empty, explicit array of package-relative regular files.

### Optional fields

- `dependencies`: local labels mapped to fully qualified, exact sources.
- `compatibility`: declared runtime and host requirements. A package listing
  `quickshell` but not `omarchy` claims no Omarchy dependency. A package listing
  both may use Omarchy-specific APIs or design primitives. Compatibility applies
  to the whole package; it is not a per-file annotation system.
- `executables`: a subset of `files` that must be installed with mode `0755`.

Dependencies use exact releases or commits in schema version 1. Version ranges,
optional dependencies, peer dependencies, features, and install hooks are not
supported.

## Path and size rules

All paths are UTF-8 relative paths using `/`. A path must not:

- be absolute, empty, or contain an empty, `.` or `..` component;
- contain NUL, control characters, or a backslash;
- exceed 1,024 UTF-8 bytes in total or 255 UTF-8 bytes in one component;
- collide with another path after Unicode NFC normalization and case folding;
- name `.git`, `qmlpack.json`, `qmlpack.lock`, or qmlpack's staging metadata;
- resolve through a symbolic link or represent a submodule or special file.

Schema version 1 limits:

| Boundary | Limit |
|---|---:|
| Manifest | 64 KiB |
| Files per package | 256 |
| One file | 4 MiB |
| One package | 16 MiB |
| Dependencies per package | 32 |
| Resolved packages | 128 |
| Dependency depth | 16 |

Limits apply before untrusted bytes are retained or parsed. They may become
configurable in a later schema only if real packages require it.

## Releases

Each package owns an independent release stream. An npm package publishes an
immutable name/version pair. A GitHub package at `packages/oma-ui` uses tags
like:

```text
packages/oma-ui/v0.2.0
```

Git tags must never be moved or reused. qmlpack encourages GitHub immutable
releases, but consumers do not rely on tag immutability alone. npm packages
also contain `qmlpack.json`; qmlpack never executes their lifecycle scripts.

## Canonical digest

The package digest is SHA-256 over a versioned canonical byte stream. Begin
with the exact bounded `qmlpack.json` bytes, then append every declared file
sorted by its normalized UTF-8 path bytes:

```text
qmlpack-package-v1\0
<manifest-byte-length as 8-byte unsigned big endian>
<exact manifest bytes>
<path-byte-length as 8-byte unsigned big endian>
<path bytes>
<mode as ASCII: 0644 or 0755>\0
<content-byte-length as 8-byte unsigned big endian>
<exact content bytes>
```

The prefix and manifest are emitted once before the first file. Line endings
and all bytes are preserved. `qmlpack.json` must not also appear in `files`;
qmlpack stores its exact source bytes as package metadata rather than exposing
it as consumer-owned source.

Canonical digest test vectors are required before schema version 1 is declared
stable.

## Project manifest

The consuming plugin owns `qmlpack.json` at its repository root:

```json
{
  "schemaVersion": 1,
  "profile": "omarchy",
  "dependencies": {
    "oma-ui": "npm:@silouanwright/oma-ui@0.2.0",
    "experimental": "github:silouanwright/omatools/packages/oma-ui@<commit>"
  }
}
```

Dependency labels determine their directory under `vendor/qmlpack/`; they do
not replace canonical source identity.

## Lockfile

`qmlpack.lock` is deterministic JSON and must be committed:

```json
{
  "schemaVersion": 1,
  "packages": {
    "oma-ui": {
      "source": "github:silouanwright/omatools/packages/oma-ui@0.2.0",
      "resolution": {
        "transport": "github",
        "repository_id": 123456789,
        "repository_name": "silouanwright/omatools",
        "package_path": "packages/oma-ui",
        "requested": "0.2.0",
        "version": "0.2.0",
        "tag": "packages/oma-ui/v0.2.0",
        "commit": "0123456789abcdef0123456789abcdef01234567"
      },
      "digest": "sha256:...",
      "files": {
        "Ui/Button.qml": "sha256:..."
      }
    }
  }
}
```

An npm lock entry records registry URL, package name, exact version, registry
integrity, and qmlpack digest. A GitHub entry records repository
ID, package path, requested reference, resolved commit, optional tag/version,
and qmlpack digest. Development commits have version and tag set to `null`.

## Installation ownership

qmlpack owns only `vendor/qmlpack/<dependency-label>/`. Installation stages a
complete dependency tree in the consuming repository, validates it, and then
atomically replaces qmlpack-owned directories. The lockfile is written last.

An update refuses to replace a managed file whose current digest differs from
the lockfile unless the user explicitly requests a forced replacement. Removal
deletes only the recorded qmlpack-owned directory and lock entry.

## Release validation

`qmlpack release-check <package-directory>` validates the manifest, declared
regular files, byte ceilings, and executable modes before an author creates a
GitHub tag or runs `npm publish`. Publishing stays transport-owned; qmlpack does
not hold registry credentials or run release hooks.
