# ADR 0001: Omarchy-first, portable source model

- Status: Accepted
- Date: 2026-08-24

## Context

Omarchy 4.x has a concrete plugin manifest, installation directory, shell
lifecycle, and marketplace, but no plugin dependency field. Quickshell supplies
the QML runtime while standalone shells define incompatible component, theme,
service, and directory conventions.

## Decision

Build Qmlpack as a host-aware source package manager, with Omarchy as its first
fully supported host. Its package envelope, retrieval, review, dependency, and
lock formats may carry portable QML, Quickshell-dependent QML, ordinary
utilities, or source that uses an explicit host contract.

Compatibility is declared per package, not per file:

- portable QML declares its Qt requirements;
- Quickshell code additionally declares its Quickshell requirement;
- Omarchy-aware code declares both Quickshell and Omarchy requirements;
- a complete Omarchy plugin remains an Omarchy plugin repository and may use
  Qmlpack to resolve its source dependencies.

Keep the formats source-oriented and free of LookElsewhere-specific paths. Do
not claim that Omarchy-aware code works in arbitrary Quickshell configurations.

Plugins commit vendored dependencies and remain installable through the normal
Omarchy plugin workflow without requiring Qmlpack on end-user machines.

## Why Omarchy-first does not mean Omarchy-only

Quickshell is the runtime beneath Omarchy Shell, but it does not define a
shared third-party package host. Standalone shells choose their own import
paths, theme tokens, services, configuration, IPC, lifecycle, and installation
layout. Calling every package portable Quickshell code would imply that it
integrates correctly with those incompatible hosts when only source retrieval
is genuinely shared. Qmlpack can still install genuinely portable Quickshell
modules; their manifests simply omit an Omarchy requirement.

Omarchy supplies the demonstrated product problem and enforceable contract:
plugin manifests, entry points, validation, installation, theme primitives,
injected shell services, and a marketplace. Qmlpack therefore targets Omarchy
plugin authors while keeping source identity, retrieval, digests, dependency
graphs, and locks free of LookElsewhere-specific assumptions.

This does not defer support for portable source. QML's existing `qmldir` module
system remains the library boundary, and Qmlpack supplies distribution,
resolution, review, and locking around it. What remains deferred is a promise
that Qmlpack understands every standalone shell's integration conventions. A
real second host can establish that contract and justify another host profile.

Committed vendoring is an adapter for the current host contract. If a future
Omarchy plugin manifest gains dependencies and `omarchy plugin add` installs
them, Qmlpack may emit the same resolved graph and lock data for that native
installer rather than materializing `vendor/qmlpack` in the source repository.
Package identities and manifests must therefore avoid depending on their
current vendored destination.

## Consequences

The initial product has a clear host contract and can validate the completed
plugin with `omarchy plugin validate`. A future Quickshell profile can reuse
the transport and lock model, but speculative profile interfaces are deferred.

Likewise, optional curated indexes can be layered over fully qualified sources
without changing publication or integrity authority. Curation is deferred
until package volume makes discovery or shared review materially useful.
