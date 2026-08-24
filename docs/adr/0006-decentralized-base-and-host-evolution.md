# ADR 0006: Decentralized base and host evolution

- Status: Accepted
- Date: 2026-08-24

## Context

Qmlpack needs useful package sharing before anyone operates a registry, and
Omarchy 4.x does not install dependencies declared by plugins. Both conditions
may change if the ecosystem succeeds.

## Decision

Use fully qualified GitHub sources as the publication and identity base.
Permit future curation services to index exact package identities and attach
metadata without owning package contents or releases.

Vendor resolved source into plugins for the present Omarchy contract. Keep
package manifests independent of their materialized destination so a future
native Omarchy dependency installer can consume Qmlpack resolution and lock
data without requiring committed vendor directories.

## Consequences

Qmlpack works without centralized infrastructure today. Popularity can improve
discovery and review incrementally rather than forcing a registry migration.
If Omarchy adopts package dependencies, moving installation to the host becomes
an adapter change instead of a new package ecosystem.
