# ADR 0001: Omarchy-first, portable source model

- Status: Accepted
- Date: 2026-08-24

## Context

Omarchy 4.x has a concrete plugin manifest, installation directory, shell
lifecycle, and marketplace, but no plugin dependency field. Quickshell supplies
the QML runtime while standalone shells define incompatible component, theme,
service, and directory conventions.

## Decision

Build Omapack for Omarchy plugin development first. Keep its package and lock
formats source-oriented and avoid embedding LookElsewhere-specific paths. Do
not claim general Quickshell compatibility until a real standalone consumer
demonstrates the required adapter boundary.

Plugins commit vendored dependencies and remain installable through the normal
Omarchy plugin workflow without requiring Omapack on end-user machines.

Committed vendoring is an adapter for the current host contract. If a future
Omarchy plugin manifest gains dependencies and `omarchy plugin add` installs
them, Omapack may emit the same resolved graph and lock data for that native
installer rather than materializing `vendor/omapack` in the source repository.
Package identities and manifests must therefore avoid depending on their
current vendored destination.

## Consequences

The initial product has a clear host contract and can validate the completed
plugin with `omarchy plugin validate`. A future Quickshell profile can reuse
the transport and lock model, but speculative profile interfaces are deferred.

Likewise, optional curated indexes can be layered over fully qualified sources
without changing publication or integrity authority. Curation is deferred
until package volume makes discovery or shared review materially useful.
