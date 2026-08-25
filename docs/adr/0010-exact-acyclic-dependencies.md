# ADR 0010: Exact acyclic dependencies

- Status: Accepted
- Date: 2026-08-24

## Context

Version ranges and multiple registries introduce a dependency solver,
dependency confusion, nondeterministic updates, and diamond conflicts before
qmlpack has demonstrated that Omarchy source packages need them.

## Decision

Schema version 1 dependencies use fully qualified sources pinned to an exact
release tag or commit. Resolve the graph depth-first with fixed depth and count
limits. Reject cycles, alias collisions, and two different resolved revisions
for the same canonical package identity.

Do not implement version ranges, automatic highest-version selection, peer
dependencies, optional dependencies, features, or a SAT solver. Updating a
direct package does not authorize changing an unrelated or transitive package;
every changed package appears separately in review output.

## Consequences

Resolution is deterministic and small enough to audit. Authors must publish a
new package release to change a dependency pin. Version ranges may be added in
a later schema if real dependency graphs demonstrate a need and their selection
policy can remain reviewable.
