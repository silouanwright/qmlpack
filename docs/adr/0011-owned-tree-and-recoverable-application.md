# ADR 0011: Owned tree and recoverable application

- Status: Accepted
- Date: 2026-08-24

## Context

Source vendoring can overwrite developer edits, leave stale files after package
reorganization, or produce a lockfile that describes only half of an interrupted
update. Individual file replacement makes ownership and rollback difficult.

## Decision

Qmlpack exclusively owns `vendor/qmlpack/<dependency-label>/` and records every
managed file. Consumers customize packages outside that tree or intentionally
fork them.

Before replacement, verify current files against the lock. Refuse to overwrite
modified managed files without an explicit force action. Build and validate the
entire candidate tree in a sibling temporary directory on the same filesystem,
then use directory renames plus a transaction marker and backup to make
interruption recoverable. Write the new lockfile last and remove the backup only
after the committed state is durable.

Removal deletes only the owned directory and recorded lock entry. It never
interprets package-provided uninstall instructions.

## Consequences

Updates can remove obsolete package files without guessing ownership, and a
failed operation preserves either the prior tree or enough transaction state to
recover it. Direct editing of vendored files is detectable rather than silently
supported.
