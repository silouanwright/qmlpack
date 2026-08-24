# ADR 0007: Explicit bounded package envelope

- Status: Accepted
- Date: 2026-08-24

## Context

An untrusted Git repository can contain very large history, packs, trees,
submodules, symbolic links, Git LFS metadata, and filesystem entries unrelated
to the package a consumer requested. Archives add traversal, special-file,
compression-ratio, entry-count, and extraction concerns.

Qmlpack packages are expected to be small collections of source files. Fetching
an entire repository or archive would create a larger security boundary than
the product requires.

## Decision

Require each package manifest to enumerate every distributed regular file.
Resolve the source to an exact commit and retrieve only the manifest and those
files. Apply fixed ceilings to response bytes, per-file bytes, aggregate bytes,
file count, dependency count, dependency depth, and total resolved packages
before retaining or parsing untrusted input.

Reject undeclared files, unsafe paths, symbolic links, submodules, devices,
duplicate normalized paths, and case-fold collisions. Do not use `git clone` or
archive extraction in schema version 1.

## Consequences

Small packages require more HTTP requests but have an inspectable resource
envelope. Archive or Git transports may be added only after measurement shows
the request overhead is material and their complete extraction boundaries are
specified and tested.
