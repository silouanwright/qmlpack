# ADR 0012: Declarative capabilities and no hooks

- Status: Accepted
- Date: 2026-08-24

## Context

Package lifecycle scripts are a major supply-chain execution boundary. Qmlpack
distributes source that may itself invoke processes after a plugin loads, but
the manager does not need to execute package code to install it.

## Decision

Do not support install, build, update, activation, or removal hooks. Never
import package Python, evaluate QML or JavaScript, source shell files, or invoke
package commands during resolution or validation.

Packages explicitly enumerate executable files. Review output highlights those
files and static occurrences of process-launching interfaces as attention cues,
without claiming complete capability detection. Only declared executables are
materialized with mode `0755`; every other file uses `0644`.

## Consequences

Installation remains data movement rather than third-party execution. Packages
that require compilation or setup are outside schema version 1 and should use
the host system package manager or a separately reviewed build process.
