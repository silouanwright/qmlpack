# Omapack Package Format

Status: draft for the first implementation.

## Package identity

A package is identified by its source repository and package path, not by a
globally claimed short name:

```text
github:<owner>/<repository>/<package-path>@<reference>
```

The initial transport supports public GitHub repositories only. The resolver
records GitHub's stable numeric repository ID so repository transfer and name
reuse cannot silently change an existing identity.

Short names are local display names. A future curated index may supply aliases
without becoming the source of package contents or release authority.

## Discovery and curation

Schema version 1 requires fully qualified package sources and has no registry.
Discovery can later be provided by any number of indexes: community-maintained,
Omarchy-maintained, organization-specific, or private. An index may attach
search metadata, review status, compatibility results, and aliases to an exact
source identity. It must not redefine a package version or replace the
repository, commit, and digest recorded by the consumer.

## Manifest

Every package root contains `omapack.json`:

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
    "omarchy": ">=4 <5",
    "quickshell": ">=0.3 <0.4"
  },
  "executables": []
}
```

`version` is deliberately absent. Released versions are package-prefixed
SemVer Git tags, for example `oma-ui/v0.2.0`. Ordinary commits are valid
development snapshots and are not assigned invented versions.

### Required fields

- `schemaVersion`: JSON integer `1`.
- `name`: a local package name matching `[a-z0-9][a-z0-9-]*`.
- `license`: one SPDX license expression for the package.
- `files`: a non-empty, explicit array of package-relative regular files.

### Optional fields

- `dependencies`: local labels mapped to fully qualified, exact sources.
- `compatibility`: declared host requirements.
- `executables`: a subset of `files` that must be installed with mode `0755`.

Dependencies use exact releases or commits in schema version 1. Version ranges,
optional dependencies, peer dependencies, features, and install hooks are not
supported.

## Path and size rules

All paths are UTF-8 relative paths using `/`. A path must not:

- be absolute, empty, or contain an empty, `.` or `..` component;
- contain NUL, control characters, or a backslash;
- collide with another path after Unicode NFC normalization and case folding;
- name `.git`, `omapack.json`, `omapack.lock`, or Omapack's staging metadata;
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

Each package owns an independent release stream. A package at `packages/oma-ui`
uses tags like:

```text
packages/oma-ui/v0.2.0
```

Tags must never be moved or reused. Omapack encourages GitHub immutable
releases, but consumers do not rely on tag immutability alone.

## Canonical digest

The package digest is SHA-256 over a versioned canonical byte stream. Begin
with the exact bounded `omapack.json` bytes, then append every declared file
sorted by its normalized UTF-8 path bytes:

```text
omapack-package-v1\0
<manifest-byte-length as 8-byte unsigned big endian>
<exact manifest bytes>
<path-byte-length as 8-byte unsigned big endian>
<path bytes>
<mode as ASCII: 0644 or 0755>\0
<content-byte-length as 8-byte unsigned big endian>
<exact content bytes>
```

The prefix and manifest are emitted once before the first file. Line endings
and all bytes are preserved. `omapack.json` must not also appear in `files`;
Omapack stores its exact source bytes as package metadata rather than exposing
it as consumer-owned source.

Canonical digest test vectors are required before schema version 1 is declared
stable.

## Project manifest

The consuming plugin owns `omapack.json` at its repository root:

```json
{
  "schemaVersion": 1,
  "profile": "omarchy",
  "dependencies": {
    "oma-ui": "github:silouanwright/omatools/packages/oma-ui@v0.2.0"
  }
}
```

Dependency labels determine their directory under `vendor/omapack/`; they do
not replace canonical source identity.

## Lockfile

`omapack.lock` is deterministic JSON and must be committed:

```json
{
  "schemaVersion": 1,
  "packages": {
    "oma-ui": {
      "source": "github:silouanwright/omatools",
      "repositoryId": 123456789,
      "packagePath": "packages/oma-ui",
      "requested": "v0.2.0",
      "version": "0.2.0",
      "tag": "packages/oma-ui/v0.2.0",
      "commit": "0123456789abcdef0123456789abcdef01234567",
      "digest": "sha256:...",
      "files": {
        "Ui/Button.qml": "sha256:..."
      }
    }
  }
}
```

Development commits have `version` and `tag` set to `null`. The exact commit
and digest remain mandatory.

## Installation ownership

Omapack owns only `vendor/omapack/<dependency-label>/`. Installation stages a
complete dependency tree in the consuming repository, validates it, and then
atomically replaces Omapack-owned directories. The lockfile is written last.

An update refuses to replace a managed file whose current digest differs from
the lockfile unless the user explicitly requests a forced replacement. Removal
deletes only the recorded Omapack-owned directory and lock entry.
