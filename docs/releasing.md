# Release handoff

qmlpack is currently validated locally. Publishing is intentionally separate
from package resolution and requires the owner's explicit credentials.

## qmlpack CLI

1. Create the `silouanwright/qmlpack` GitHub repository and add it as `origin`.
2. Run `cargo fmt --check`, `cargo test`, and
   `cargo clippy --all-targets -- -D warnings`.
3. Replace the development version in `Cargo.toml`, commit, and create a signed
   SemVer tag.
4. Build release binaries through GitHub Actions or another inspectable release
   job and attach their checksums to the GitHub release.

Do not add an automatic self-update path. Native Arch/AUR packaging can follow
after the command and lockfile contracts stabilize.

## Omatools packages

Before every independent package release, run:

```bash
qmlpack release-check packages/oma-ui
qmlpack release-check packages/bounded-read
(cd packages/oma-ui && npm pack --dry-run)
(cd packages/bounded-read && npm pack --dry-run)
```

The first npm publication still requires `npm login`, ownership of the
`@silouanwright` scope, and an explicit `npm publish --access public` from each
package directory. Never run publication from qmlpack and never add package
lifecycle scripts.

After publication, switch a consumer from GitHub to npm only through a normal
`qmlpack update` review. qmlpack does not silently substitute transports.
