# ADR 0005: Standalone Rust CLI

- Status: Accepted
- Date: 2026-08-24
- Supersedes: the initial Python standard-library prototype

## Context

An initial Python slice proved the strict manifest, source-reference, and
canonical-digest contracts with little code. qmlpack, however, is intended to
be a distributable security-sensitive developer tool rather than a plugin
runtime helper. Requiring a particular interpreter and environment is an
avoidable deployment boundary.

## Decision

Implement qmlpack as one Rust CLI crate producing a standalone binary. Use
mature focused crates for command parsing, strict serialization, HTTPS with
rustls, SemVer, SHA-256, Unicode normalization, Base64, and secure temporary
files. Do not create a workspace or internal abstraction layers until more than
one shipped crate requires them.

Keep LookElsewhere's small `bounded-read` Python utility separate. It solves a
narrow descriptor-relative runtime problem inside the plugin and is not part of
qmlpack's execution path.

## Alternatives considered

- **Python:** produced the smallest useful prototype and remains appropriate
  for the tiny runtime helper, but would make the distributable package manager
  depend on an interpreter and Python environment. Rejected for the shipped
  CLI, not as an unsuitable language in general.
- **JavaScript or Node:** would align with npm publishing but would require a
  JavaScript runtime to install QML packages. qmlpack uses the registry HTTP API
  directly and deliberately avoids npm's installation and lifecycle execution
  model.
- **Shell:** is suitable for orchestration but not for strict duplicate-key JSON
  parsing, bounded archive handling, Unicode path validation, recoverable
  transactions, and cross-platform deterministic serialization.
- **C++ with Qt:** could reuse Qt types but would add Qt build and ABI concerns
  to a developer CLI that does not need to load or evaluate QML.
- **Go:** was a credible standalone-binary alternative and was used by the
  historical qpm client and server. Rust was selected because the implementation
  already benefits from typed source variants, explicit error propagation, and
  mature crates for the required parsing and integrity boundaries. This is a
  project choice rather than a claim that Go could not implement the design.

Rust does not make package handling secure by itself. The security properties
come from explicit byte and file-count limits, non-execution of package code,
strict parsing, path validation, digest verification, and tests at each trust
boundary.

## Consequences

qmlpack can ship as one architecture-specific release asset and later through
AUR or an Omarchy-supported system package without a Python runtime dependency.
Compilation is heavier than the prototype, but package consumers do not compile
or run qmlpack when using an already self-contained Omarchy plugin.
