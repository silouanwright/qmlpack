# ADR 0009: Deterministic lock and content digest

- Status: Accepted
- Date: 2026-08-24

## Context

Git commit IDs identify repository history, not the package envelope qmlpack
materializes. Tags may move, and future Git object formats may differ. A
consumer needs to detect altered files, local modifications, and noncanonical
materialization independently of Git.

## Decision

Calculate a schema-versioned SHA-256 over the exact manifest bytes and every
declared file's normalized path, installed mode, length, and exact bytes in a
defined order. Record the package digest and individual file digests in a
deterministically serialized lockfile alongside source, repository ID, tag,
version, and exact commit.

The canonical byte format is normative and receives fixed test vectors before
schema version 1 is stable. JSON key ordering and whitespace in the generated
lockfile are deterministic but are not themselves the package digest input.

## Consequences

qmlpack can prove that installed source matches the reviewed package envelope
without treating a Git hash as the only integrity primitive. Any future digest
change requires a new digest algorithm identifier or schema version rather
than silently changing existing results.
