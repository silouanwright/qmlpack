# Explicit Review Policy

Packages are source code that will run as part of a desktop environment. The
developer consuming a package is responsible for reviewing every installation
and every update.

Qmlpack verifies provenance, integrity, and reproducibility. It does not decide
whether code is trustworthy.

## Required interaction

`add`, `update`, and `remove` prepare a candidate and display a review summary.
They do not change the project until the developer separately runs `apply`.

The review includes:

- canonical source identity and whether the dependency is direct or transitive;
- requested GitHub reference, release tag, and resolved commit, or exact npm
  version and registry integrity;
- added, removed, and changed files;
- declared executable files and compatibility requirements;
- package and per-file SHA-256 digests;
- license and notice files.

Static indicators are review aids, not proof of safety.

## Human and AI review

Developers may inspect changes manually, ask an AI coding agent to review the
materialized candidate, or do both. Qmlpack provides deterministic Markdown,
lock data, and materialized source for that purpose.

AI review can miss malicious behavior, indirect execution, vulnerabilities, or
context-specific risk. Documentation and command output must never describe an
AI-reviewed package as certified, trusted, or safe.

## Updates

There are no automatic package updates. Checking for a newer release does not
install it. Each package and each changed transitive dependency is shown
separately. Applying one update must not implicitly authorize unrelated
dependency updates.

Existing locally modified managed files block replacement unless the developer
uses an explicit force option after seeing the affected paths.

## Language

Use:

- verified integrity;
- matches the lockfile;
- exact source and revision;
- reviewed or not yet reviewed.

Do not use:

- verified safe;
- trusted package;
- approved code;
- secure because the digest matched.
