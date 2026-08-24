# ADR 0002: GitHub direct transport

- Status: Accepted
- Date: 2026-08-24

## Context

Cloning an untrusted repository or extracting an archive exposes the installer
to repository, pack, archive, special-file, and decompression resource attacks.
Omapack packages are expected to be small source libraries.

## Decision

Resolve a GitHub reference to an exact commit, retrieve a byte-capped manifest,
and retrieve only its explicit byte-capped files. Reject special entries and
unsafe paths before writing. Record GitHub's stable numeric repository ID and
calculate an independent canonical SHA-256 digest.

## Consequences

The implementation performs more HTTP requests than an archive download and
must handle GitHub rate limits. This is an acceptable initial tradeoff for a
small, inspectable safety boundary. Archive and Git transports are deferred
until measurements show they are necessary.
