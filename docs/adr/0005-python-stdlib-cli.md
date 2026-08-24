# ADR 0005: Python standard-library CLI

- Status: Accepted
- Date: 2026-08-24

## Context

Omarchy includes Python. The initial tool needs bounded HTTP, strict JSON,
cryptographic hashing, descriptor-safe filesystem operations, and atomic local
writes, but does not require a daemon, native UI, or high throughput.

## Decision

Implement the first CLI with Python's standard library and no runtime package
dependencies. Distribute it independently of vendored packages; packages do
not execute or install the manager.

## Consequences

The tool remains easy to inspect and bootstrap. A compiled implementation is
appropriate only if deployment, performance, or a missing syscall boundary is
demonstrated rather than anticipated.
