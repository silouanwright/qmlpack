# ADR 0005: Standalone Rust CLI

- Status: Accepted
- Date: 2026-08-24
- Supersedes: the initial Python standard-library prototype

## Context

An initial Python slice proved the strict manifest, source-reference, and
canonical-digest contracts with little code. Qmlpack, however, is intended to
be a distributable security-sensitive developer tool rather than a plugin
runtime helper. Requiring a particular interpreter and environment is an
avoidable deployment boundary.

## Decision

Implement Qmlpack as one Rust CLI crate producing a standalone binary. Use
mature focused crates for command parsing, strict serialization, HTTPS with
rustls, SemVer, SHA-256, Unicode normalization, Base64, and secure temporary
files. Do not create a workspace or internal abstraction layers until more than
one shipped crate requires them.

Keep LookElsewhere's small `bounded-read` Python utility separate. It solves a
narrow descriptor-relative runtime problem inside the plugin and is not part of
Qmlpack's execution path.

## Consequences

Qmlpack can ship as one architecture-specific release asset and later through
AUR or an Omarchy-supported system package without a Python runtime dependency.
Compilation is heavier than the prototype, but package consumers do not compile
or run Qmlpack when using an already self-contained Omarchy plugin.
