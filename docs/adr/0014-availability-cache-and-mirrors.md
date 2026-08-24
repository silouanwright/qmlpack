# ADR 0014: Availability, cache, and mirrors

- Status: Accepted
- Date: 2026-08-24

## Context

GitHub repositories may be deleted, privatized, rate-limited, or temporarily
unavailable. Integrity and availability are separate properties. Committed
vendoring protects existing plugin source but not a fresh dependency restore.

## Decision

Treat GitHub availability as fallible and use bounded timeouts, authenticated
requests when configured, rate-limit diagnostics, and conservative retry with
server-directed backoff. Never retry indefinitely.

A local content-addressed cache may later retain already verified package
envelopes keyed by canonical digest. Mirrors and curated archives are deferred;
if added, they must reproduce the same locked bytes and cannot redefine package
identity or digest.

## Consequences

Schema version 1 has no server requirement beyond GitHub, and existing committed
plugins remain usable during upstream loss. Fresh restoration is not guaranteed
until caching or mirrors are justified by adoption.
