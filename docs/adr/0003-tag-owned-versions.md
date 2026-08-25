# ADR 0003: Transport-owned released versions

- Status: Accepted
- Date: 2026-08-24

## Context

A released version must have one authority. Recording it independently in
`qmlpack.json`, an npm package, and a Git tag would create conflicting sources
of truth. Ordinary development commits also should not require meaningless
version bumps.

## Decision

`qmlpack.json` does not contain a current version. For npm releases,
`package.json` and the registry's immutable name/version pair are authoritative.
For GitHub releases, independent packages use package-path-prefixed SemVer tags
such as `packages/oma-ui-kit/v0.1.0`. Exact commits may be installed as unversioned
development snapshots.

The lockfile records the requested reference, resolved tag and version when
applicable, exact commit, stable repository ID, and independent digest.

## Consequences

Release tooling validates the selected transport's metadata. npm rejects reuse
of a published name/version pair. Existing GitHub consumers detect moved tags
through their lockfile; first-time GitHub consumers need immutable releases or
a future transparency index to prove historical immutability.
