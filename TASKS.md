# Project tasks

## Prepare the first CLI release artifact

- **Objective:** provide an inspectable, local release-artifact build for the
  CLI's currently validated `x86_64-unknown-linux-gnu` target without changing
  the product version or publishing anything.
- **Status:** local release-build prerequisite complete; owner release actions
  remain deferred.
- **Implementation:** `scripts/build-release-artifacts.sh` builds with Rust
  1.88.0 and `Cargo.lock`, writes a normalized archive and SHA-256 checksum to
  ignored `target/release-artifacts/`, then extracts and smoke-tests the
  packaged binary. `docs/releasing.md` records the supported target and owner
  release steps.
- **Decision:** only Linux x86_64 is claimed. The current filesystem code uses
  Unix APIs, and Omarchy on Linux x86_64 is the repository's validated host.
  Additional platforms remain deferred until tested rather than inferred from
  transitive dependency support.
- **Evidence (2026-10-23):** shell syntax, `cargo fmt --check`, stable
  `cargo test --locked`, strict clippy, and `cargo +1.88.0 test --locked` pass;
  both test runs report 11 passing tests. Two serialized runs of the artifact
  builder produced the same archive digest,
  `d47de981e1e1b6f5334c29b753991cebf4b83c5e7bada51c61123c2b20376a00`.
  Independent verification passes the generated checksum, lists only the
  binary, license, and README under one versioned directory, confirms root
  ownership and normalized timestamps in the archive, preserves binary mode
  `0755`, and runs `qmlpack 0.1.0-dev` from an extracted copy. Heavy Rust work
  used the shared two-core build limiter.
- **Remaining owner release work:** choose the release version, replace
  `0.1.0-dev`, refresh and inspect qmlpack's own `Cargo.lock` entry without
  dependency drift, run the documented gates, commit the version files, create
  a signed SemVer tag, build from a clean tagged checkout, inspect the archive
  and checksum, then explicitly create and publish the GitHub release.
- **Boundaries:** do not add self-update behavior, change the package format,
  publish crates or binaries, install globally, or create an automated release
  workflow as part of this task.
