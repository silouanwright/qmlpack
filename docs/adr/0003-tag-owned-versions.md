# ADR 0003: Git tags own released versions

- Status: Accepted
- Date: 2026-08-24

## Context

Without a publishing registry, recording a version in both `omapack.json` and a
Git tag creates two mutable sources of truth. Ordinary development commits also
should not require meaningless version bumps.

## Decision

Package manifests do not contain a current version. Independent package
releases use package-path-prefixed SemVer tags such as
`packages/oma-ui/v0.2.0`. Exact commits may be installed as unversioned
development snapshots.

The lockfile records the requested reference, resolved tag and version when
applicable, exact commit, stable repository ID, and independent digest.

## Consequences

Release tooling validates tag syntax and uniqueness. Existing consumers detect
moved tags through their lockfile; first-time consumers need GitHub immutable
releases or a future transparency index to prove historical immutability.
