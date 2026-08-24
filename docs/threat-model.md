# Threat Model

Omapack downloads source code that a consuming Omarchy plugin will commit and
later execute inside a long-lived desktop shell. Package input is hostile until
the developer has inspected and explicitly accepted it.

## Security claims

Omapack is intended to provide:

- bounded retrieval and parsing of package metadata and files;
- stable source identity and exact revision recording;
- deterministic integrity verification;
- path-safe, atomic installation into a tool-owned directory;
- visible review of every installation and update;
- detection of local modifications to managed files.

Omapack does not claim that package code is safe, benevolent, correct, private,
or suitable for execution.

## Adversaries and failures

The design considers:

- a malicious or compromised package repository;
- mutable tags and repository transfers or name reuse;
- oversized manifests, files, repositories, or dependency graphs;
- path traversal, Unicode/case collisions, and special filesystem entries;
- dependency cycles, conflicting identities, and dependency confusion;
- malicious QML, JavaScript, shell commands, Python, or executables;
- unexpected local changes and interrupted installations;
- deletion or privatization of an upstream repository;
- GitHub API failure, throttling, truncation, and partial responses.

It does not defend against arbitrary code already executing as the same user,
a compromised operating system, a compromised GitHub service returning a
self-consistent malicious repository, or a user explicitly forcing unsafe
replacement.

## Retrieval boundary

Schema version 1 does not clone repositories or extract archives. It resolves
one GitHub reference to an exact commit, fetches a bounded package manifest,
then fetches only explicitly declared files at that commit.

Every response has a timeout and byte ceiling. Omapack rejects redirects to
unexpected hosts, truncated API trees, undeclared files, invalid content
encodings, and responses that do not identify the requested revision.

GitHub credentials are read only through conventional environment or CLI
configuration and are never written to manifests, lockfiles, review output, or
subprocess arguments.

## Integrity and freshness

The lockfile's independent SHA-256 digest is the integrity boundary. A Git tag
is a human release name, and a Git commit identifies source history, but neither
is treated as proof that code is trustworthy.

A moved tag is detected for an existing consumer because its stored commit and
digest no longer match. A first-time consumer cannot prove that a tag was never
moved without an independent transparency service. GitHub immutable releases
and later curated indexes can strengthen that property but are not required for
the initial decentralized workflow.

## Code execution

Omapack runs no package-provided install, update, build, or removal hooks.
Executable files must be declared and are called out during review. Source is
not loaded into Quickshell during retrieval or validation.

The CLI must not import package Python modules, evaluate QML/JavaScript, source
shell files, or invoke commands declared by a package.

## Installation and recovery

All writes occur beneath a project-controlled `vendor/omapack` directory.
Paths are revalidated at the filesystem boundary. Staging uses a sibling
temporary directory on the same filesystem, and the old installation remains
recoverable until the replacement is complete. The lockfile is written only
after every package directory is in its final state.

If installation cannot finish consistently, Omapack must stop and leave either
the previous complete installation or a clearly reported recoverable staging
directory. It must never silently continue with a partially updated lockfile.

## Availability

An upstream repository may disappear. Existing plugins keep their committed
vendored files and lockfile. Fresh installations may fail until the same locked
content is restored or mirrored. A content-addressed cache is a later
availability optimization, not required for correctness.
