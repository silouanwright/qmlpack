# qmlpack

Review-first source packages for QML, Quickshell, and Omarchy.

QML developers already share code by copying files between projects. qmlpack
keeps that simple, self-contained model while adding versioning, provenance,
integrity, dependency resolution, and a safe update path.

Packages are committed as ordinary files under `vendor/qmlpack/`. People using
your app or plugin do not need qmlpack, Rust, npm, or network access.

## Install

With a stable [Rust toolchain](https://rustup.rs/):

```bash
cargo install --git https://github.com/silouanwright/qmlpack --locked
```

## Use

```bash
qmlpack init --profile omarchy
qmlpack add oma-ui github:silouanwright/omatools/packages/oma-ui@0.2.0
```

Nothing changes in your project yet. qmlpack resolves and materializes a
candidate for inspection:

```bash
qmlpack diff
```

Apply it only after review, then verify the installed source whenever needed:

```bash
qmlpack apply
qmlpack verify
```

Updates and removals use the same review boundary:

```bash
qmlpack update oma-ui --to 0.2.1
qmlpack remove oma-ui
```

## Sources

qmlpack supports exact npm releases and GitHub releases or commits:

```text
npm:@scope/package@1.2.3
github:owner/repository/package/path@1.2.3
github:owner/repository/package/path@<commit>
```

It never runs `npm install`, lifecycle scripts, build hooks, or
package-provided commands.

## Publish a package

A package owns a small `qmlpack.json` that names its license, compatibility,
distributed files, dependencies, and any executable files. Validate it before
creating a GitHub tag or publishing to npm:

```bash
qmlpack release-check packages/oma-ui
```

Packages release independently. qmlpack does not own registry credentials or
publish on an author's behalf.

## Review, not trust

qmlpack verifies exactly what source was retrieved and installed. It does not
certify third-party code as safe. Every addition and update stops for inspection
before changing the project.

For the detailed contracts and reasoning, see the
[package format](docs/package-format.md), [threat model](docs/threat-model.md),
[review policy](docs/review-policy.md), and [ADRs](docs/adr/README.md).

## Working proof

[LookElsewhere](https://github.com/silouanwright/lookelsewhere) uses qmlpack for
its shared keyboard-first QML controls and bounded state-file reader while
remaining a self-contained Omarchy plugin.

qmlpack is early software. Omarchy 4.x is its first fully validated host, with
portable QML and Quickshell packages supported by the same package format.

## License

[MIT](LICENSE)
