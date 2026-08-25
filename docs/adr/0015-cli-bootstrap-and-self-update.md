# ADR 0015: CLI bootstrap and self-update

- Status: Accepted
- Date: 2026-08-24

## Context

The package manager is more privileged than an ordinary package because it
retrieves and writes every dependency. Installing or updating the manager
through package-provided code would make its trust boundary recursive.

## Decision

Implement qmlpack as a standalone Rust binary and distribute it separately from
packages. Do not allow qmlpack packages to install or update qmlpack. The
initial repository documents pinned release binaries; a signed Arch/AUR or
Omarchy-supported system package may follow when release demand exists.

The CLI does not silently self-update. Its own releases follow a conventional
repository release process outside the source package protocol.

## Consequences

Bootstrap remains inspectable and does not require a package registry or
language runtime. Native packaging work is postponed until there is a stable
CLI contract worth distributing.
