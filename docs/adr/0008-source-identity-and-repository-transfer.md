# ADR 0008: Source identity and repository transfer

- Status: Accepted
- Date: 2026-08-24

## Context

GitHub owner and repository names can change, repositories can transfer between
accounts, and an abandoned name can later identify unrelated content. A short
package name cannot be globally unique in a decentralized ecosystem.

## Decision

Canonical identity is the source provider, GitHub's stable numeric repository
ID, and package path. Human-readable owner/repository names are retained for
display and retrieval but are not sufficient identity.

The first time a project adds a package, it records the repository ID. Later
operations reject an unexpected ID. A legitimate repository transfer requires
an explicit trust update that shows the old and new canonical names while
preserving the numeric ID. Reusing a name with a different ID is a different
package source.

Dependency references are fully qualified. Friendly aliases may be supplied
locally or by future indexes but never participate in canonical resolution.

## Consequences

Repository renames and transfers remain supportable without enabling silent
name reuse. Lockfiles are more verbose, which is appropriate for provenance
data.
