# ADR 0018: qmlpack name and scope

- Status: Accepted
- Date: 2026-08-24

## Context

The original Omapack name accurately described the first consumer but implied
that reusable source must depend on Omarchy. In reality, the same package
envelope can carry portable Qt/QML modules, Quickshell-aware components,
Omarchy-aware components, and non-QML utilities used by those projects.

The archived `qpm` name already identifies the historical Qt Package Manager.
`qspack` would emphasize Quickshell but understate portable QML and could imply
an official relationship with Quickshell.

## Decision

Name the project and command qmlpack. Use `qmlpack.json`, `qmlpack.lock`,
`.qmlpack/`, and `vendor/qmlpack/` consistently.

qmlpack is a review-first source dependency materializer for QML projects. It
does not replace npm, GitHub, QML modules, Quickshell, or the Omarchy plugin
manager:

- npm and GitHub publish and store source;
- QML `qmldir` files define runtime module namespaces;
- qmlpack retrieves, reviews, locks, and materializes packages;
- Quickshell supplies a runtime contract when declared;
- Omarchy supplies an optional host and complete-plugin contract.

## Consequences

Omarchy remains the first fully validated host without making Omarchy a
requirement for every package. The project must clearly state that it is an
independent community tool and not part of Qt, Quickshell, npm, or Omarchy.

No compatibility aliases are retained because Omapack has not been released.
