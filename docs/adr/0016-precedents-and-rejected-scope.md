# ADR 0016: Precedents and rejected scope

- Status: Accepted
- Date: 2026-08-24

## Context

Omapack overlaps several established systems but serves a narrower host that
currently accepts only self-contained plugin repositories.

## Decision

Adopt:

- shadcn's source ownership, explicit files, view, diff, and dry-run model;
- npm's separation of dependency intent from an exact committed lock;
- Cargo's immutable release expectation, checksums, and yanking precedent;
- Go's Git-backed module tags, subdirectory releases, vendoring consistency,
  file-envelope safety, and separation of source from optional proxies;
- qpm's insight that reusable QML source can remain in author-owned repositories.

Reject from the initial scope:

- a central publishing service or mandatory registry;
- npm-style lifecycle scripts and automatic transitive updates;
- a Cargo-style global namespace and package artifact store;
- pnpm-style symlink or hardlink installation, which conflicts with Omarchy's
  self-contained plugin validation and marketplace packaging;
- Git submodules, repository cloning, and archive extraction;
- dependency ranges, peer dependencies, and a general solver;
- discovery, popularity ranking, review badges, and curation before package
  volume makes them useful.

## Consequences

The first product solves the demonstrated Omarchy problem without pretending to
be a universal language package manager. Later services can layer discovery,
curation, mirrors, or native host installation over the same exact identities
and lock data.
