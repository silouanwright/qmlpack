# ADR 0007: Explicit bounded package envelope

- Status: Accepted
- Date: 2026-08-24

## Context

An untrusted Git repository can contain very large history, packs, trees,
submodules, symbolic links, Git LFS metadata, and filesystem entries unrelated
to the package a consumer requested. Archives add traversal, special-file,
compression-ratio, entry-count, and extraction concerns.

qmlpack packages are expected to be small collections of source files. Fetching
an entire repository creates a larger security boundary than the product
requires. npm, however, exposes released package contents as tar archives.

## Decision

Require each package manifest to enumerate every distributed regular file.
Resolve the source to an exact commit and retrieve only the manifest and those
files. Apply fixed ceilings to response bytes, per-file bytes, aggregate bytes,
file count, dependency count, dependency depth, and total resolved packages
before retaining or parsing untrusted input.

Reject unsafe paths, symbolic links, submodules, devices, duplicate normalized
paths, and case-fold collisions. GitHub retrieval fetches only declared blobs
and never clones or extracts the repository. npm retrieval verifies registry
integrity, bounds both compressed and expanded content, rejects special archive
entries, and retains only manifest-declared files from the archive.

## Consequences

GitHub packages require more HTTP requests but have an inspectable resource
envelope. npm packages accept the unavoidable archive boundary without treating
archive membership as authority over installed files.
