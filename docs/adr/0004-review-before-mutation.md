# ADR 0004: Review before mutation

- Status: Accepted
- Date: 2026-08-24

## Context

Integrity does not imply safety. QML, JavaScript, scripts, and executable tools
can perform harmful actions while matching their published digest exactly.

## Decision

Package additions and updates prepare reviewable candidates and require a
separate explicit application step. qmlpack performs no automatic updates and
runs no package-provided lifecycle hooks. Human or AI-assisted review is
encouraged without being represented as certification.

## Consequences

The workflow requires deliberate developer action, which is appropriate for
source that will execute inside the desktop shell. Review output is a stable
product surface and must be available as text, Markdown, and JSON.
