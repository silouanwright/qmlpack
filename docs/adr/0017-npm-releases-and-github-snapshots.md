# ADR 0017: npm releases and GitHub snapshots

- Status: Accepted
- Date: 2026-08-24

## Context

Git tags and exact commits can distribute source without an account in a
package registry, but they make Qmlpack reconstruct release identity,
immutability, discovery, and version metadata from repository conventions.
The historical Qt Package Manager operated its own metadata service while
keeping source in author-owned Git repositories. When that service shut down,
the package ecosystem lost its shared discovery and release layer.

The npm public registry accepts packages of computer code and does not require
JavaScript entry points. It supplies scoped names, immutable name/version
pairs, SemVer metadata, tarball storage, integrity values, and discovery. A QML
module remains a functional source package rather than general-purpose file
storage under npm's acceptable-use terms.

## Decision

Use npm as the preferred transport for stable public releases and retain
GitHub as a first-class transport for exact commits and Git-tagged releases.
Both transports resolve into the same bounded Qmlpack package envelope and
canonical digest.

Example sources:

```text
npm:@silouanwright/oma-ui@0.1.0
github:silouanwright/omatools/packages/oma-ui@<commit>
```

Qmlpack accesses registry metadata and tarballs directly. It does not invoke
`npm install`, create `node_modules`, or execute package lifecycle scripts.
Before parsing or retaining package content it enforces compressed-response,
expanded-byte, file-count, path, and individual-file limits. It verifies the
registry integrity value and its own canonical package digest.

The package format remains independent of either transport. npm's
`package.json` owns the published name and version; `qmlpack.json` owns the
QML/Quickshell/host compatibility declaration and explicit source envelope.
GitHub commits remain useful for development snapshots without invented
versions.

## Selection policy

Prefer npm for stable reusable package releases because it provides immutable
scoped name/version pairs, registry integrity, discovery, and artifact delivery
without GitHub API rate limits. Prefer GitHub for exact development snapshots,
unreleased or experimental packages, and authors who do not want a registry
publishing step.

Complete Omarchy plugins continue to publish as Git repositories because that
is the Omarchy installation and marketplace contract. Their Qmlpack
dependencies may independently use npm releases or GitHub snapshots.

Authors may expose the same project through both services: GitHub remains the
reviewable source history while npm carries the canonical stable artifact.
Qmlpack does not consider those locations interchangeable. It records the
requested transport in the lockfile and never silently falls back from npm to
GitHub or from GitHub to npm, even when metadata links them to the same project.
Changing transports is an explicit dependency update with a complete review.

## Consequences

Qmlpack does not operate registry infrastructure and users do not need Node or
npm to consume packages. Publishers choosing npm use its account, namespace,
and acceptable-use policies. Packages remain retrievable from GitHub when npm
is unsuitable, and lockfiles identify the transport so one cannot silently
substitute for the other.

An npm outage or policy change affects npm-backed restoration but does not
invalidate already vendored source or GitHub-backed packages. A compatible
registry may be supported later without changing QML module semantics.
