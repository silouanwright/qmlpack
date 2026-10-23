# Release handoff

qmlpack is currently validated locally. Publishing is intentionally separate
from package resolution and requires the owner's explicit credentials.

## qmlpack CLI

The first supported binary target is `x86_64-unknown-linux-gnu`. This matches
the validated Omarchy host and the CLI's current Unix filesystem APIs. Do not
advertise other targets until they have an explicit compatibility pass.

1. Replace the development version in `Cargo.toml`. Run `cargo check` once to
   refresh qmlpack's own entry in `Cargo.lock`, then inspect both files and
   confirm that no dependency versions moved.
2. Run the CI checks with the lockfile enforced:

   ```bash
   cargo fmt --check
   cargo test --locked
   cargo clippy --all-targets --locked -- -D warnings
   cargo +1.88.0 test --locked
   ```

3. Commit the version files and create a signed SemVer tag.
4. From a clean checkout of that tag, build and inspect the assets:

   ```bash
   bash scripts/build-release-artifacts.sh
   cd target/release-artifacts
   version="0.1.0" # Replace with the tagged version.
   artifact="qmlpack-${version}-x86_64-unknown-linux-gnu.tar.gz"
   sha256sum --check "${artifact}.sha256"
   tar -tzf "$artifact"
   ```

The script uses Rust 1.88.0 and `Cargo.lock`, creates a normalized archive and
SHA-256 checksum under `target/release-artifacts/`, extracts the archive, and
runs the packaged binary's `--version` command. The archive contains only the
CLI binary, license, and README.

Review the checksum and archive listing before creating the GitHub release.
Attach both the `.tar.gz` and `.tar.gz.sha256` files. Publishing the release is
an explicit owner action; the script does not tag, push, upload, or publish.

Do not add an automatic self-update path. Native Arch/AUR packaging can follow
after the command and lockfile contracts stabilize.

## Omatools packages

Before every independent package release, run:

```bash
qmlpack release-check packages/oma-ui-kit
qmlpack release-check packages/bounded-read
(cd packages/oma-ui-kit && npm pack --dry-run)
(cd packages/bounded-read && npm pack --dry-run)
```

The first npm publication still requires `npm login`, ownership of the
`@silouanwright` scope, and an explicit `npm publish --access public` from each
package directory. Never run publication from qmlpack and never add package
lifecycle scripts.

After publication, switch a consumer from GitHub to npm only through a normal
`qmlpack update` review. qmlpack does not silently substitute transports.
