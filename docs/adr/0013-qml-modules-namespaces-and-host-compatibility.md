# ADR 0013: QML modules, namespaces, and host compatibility

- Status: Accepted
- Date: 2026-08-24

## Context

QML already has a module system through `qmldir` and import paths, but it does
not distribute third-party source. Quickshell configurations and Omarchy plugins
can also use relative imports. Two packages can accidentally expose identical
module URIs or type names, and a package may depend on host-specific APIs.

## Decision

qmlpack distributes source packages; it does not replace QML module semantics.
Packages that expose reusable QML types should include a `qmldir` and use a
namespaced module URI. The first implementation validates declared Qt,
Quickshell, and Omarchy compatibility metadata and reports it during review.
Requirements are cumulative: declaring Omarchy means the package may use
Omarchy's injected services, themes, components, paths, or lifecycle even
though its source is QML executed by Quickshell.

Compatibility is package-wide. Authors should split portable primitives from
host adapters when both are independently useful, but qmlpack does not require
one package per file and does not infer portability by scanning imports. For
example, a generic rolling-number component can declare Quickshell alone,
while a control bound to Omarchy theme tokens declares Quickshell and Omarchy.

Relative imports remain allowed for small, internally namespaced vendored
libraries such as LookElsewhere's initial extraction. qmlpack does not rewrite
QML imports or source files. Conflicting module URIs or dependency labels are
errors rather than candidates for automatic renaming.

## Consequences

QML tooling and runtime imports remain recognizable, while qmlpack focuses on
retrieval and materialization. Host API compatibility is declared and reviewed,
not inferred perfectly from source.
