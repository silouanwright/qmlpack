# ADR 0013: QML modules, namespaces, and host compatibility

- Status: Accepted
- Date: 2026-08-24

## Context

QML already has a module system through `qmldir` and import paths, but it does
not distribute third-party source. Quickshell configurations and Omarchy plugins
can also use relative imports. Two packages can accidentally expose identical
module URIs or type names, and a package may depend on host-specific APIs.

## Decision

Omapack distributes source packages; it does not replace QML module semantics.
Packages that expose reusable QML types should include a `qmldir` and use a
namespaced module URI. The first implementation validates declared Omarchy and
Quickshell compatibility metadata and reports it during review.

Relative imports remain allowed for small, internally namespaced vendored
libraries such as LookElsewhere's initial extraction. Omapack does not rewrite
QML imports or source files. Conflicting module URIs or dependency labels are
errors rather than candidates for automatic renaming.

## Consequences

QML tooling and runtime imports remain recognizable, while Omapack focuses on
retrieval and materialization. Host API compatibility is declared and reviewed,
not inferred perfectly from source.
