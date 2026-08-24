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

## Consequences

The initial product has a clear host contract and can validate the completed
plugin with `omarchy plugin validate`. A future Quickshell profile can reuse
the transport and lock model, but speculative profile interfaces are deferred.
