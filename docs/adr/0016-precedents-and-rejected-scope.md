# ADR 0016: Precedents and rejected scope

- Status: Accepted
- Date: 2026-08-24

## Context

Qmlpack overlaps several established systems but serves a narrower host that
currently accepts only self-contained plugin repositories.

## Decision

Adopt:

- shadcn's source ownership, explicit files, view, diff, and dry-run model;
- npm's separation of dependency intent from an exact committed lock;
- Cargo's immutable release expectation, checksums, and yanking precedent;
- Go's Git-backed module tags, subdirectory releases, vendoring consistency,
  file-envelope safety, and separation of source from optional proxies;
- qpm's source vendoring, nested dependencies, immutable version-to-revision
  mapping, namespaced QML modules, and author-owned repositories;
- npm's scoped immutable releases and hosted artifact availability without
  adopting `node_modules` or lifecycle execution.

Reject from the initial scope:

- a publishing service operated by Qmlpack or a mandatory registry;
- npm-style lifecycle scripts and automatic transitive updates;
- a Qmlpack-operated global namespace and package artifact store;
- pnpm-style symlink or hardlink installation, which conflicts with Omarchy's
  self-contained plugin validation and marketplace packaging;
- Git submodules, repository cloning, and archive extraction;
- dependency ranges, peer dependencies, and a general solver;
- discovery, popularity ranking, review badges, and curation before package
  volume makes them useful.

## Consequences

The first product solves QML source materialization with Omarchy as its first
validated host. npm may provide releases and discovery, while GitHub preserves
decentralized exact sources. Later services can layer curation, mirrors, or
native host installation over the same identities and lock data.
