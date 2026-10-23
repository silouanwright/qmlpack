#!/usr/bin/env bash
set -euo pipefail

readonly toolchain="1.88.0"
readonly target="x86_64-unknown-linux-gnu"
readonly repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly output_dir="${QMLPACK_ARTIFACT_DIR:-${repository_root}/target/release-artifacts}"

cd "$repository_root"

host="$(rustc "+${toolchain}" -vV | sed -n 's/^host: //p')"
if [[ "$host" != "$target" ]]; then
    printf 'error: release artifacts are currently validated only on %s (host is %s)\n' \
        "$target" "$host" >&2
    exit 1
fi

cargo "+${toolchain}" build --release --locked --target "$target"

readonly built_binary="${repository_root}/target/${target}/release/qmlpack"
version_output="$($built_binary --version)"
version="${version_output#qmlpack }"
if [[ "$version" == "$version_output" || -z "$version" || "$version" == */* ]]; then
    printf 'error: unexpected version output: %s\n' "$version_output" >&2
    exit 1
fi

readonly archive_root="qmlpack-${version}-${target}"
readonly archive_name="${archive_root}.tar.gz"
readonly archive_path="${output_dir}/${archive_name}"

scratch_dir="$(mktemp -d "${TMPDIR:-/tmp}/qmlpack-release.XXXXXXXX")"
trap 'rm -rf -- "$scratch_dir"' EXIT

install -Dm755 "$built_binary" "${scratch_dir}/${archive_root}/qmlpack"
install -Dm644 LICENSE "${scratch_dir}/${archive_root}/LICENSE"
install -Dm644 README.md "${scratch_dir}/${archive_root}/README.md"

tar \
    --sort=name \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    --mtime="@${SOURCE_DATE_EPOCH:-0}" \
    -C "$scratch_dir" \
    -cf - \
    "$archive_root" \
    | gzip -n -9 > "${scratch_dir}/${archive_name}"

mkdir -p "${scratch_dir}/smoke"
tar -xzf "${scratch_dir}/${archive_name}" -C "${scratch_dir}/smoke"
smoke_version="$("${scratch_dir}/smoke/${archive_root}/qmlpack" --version)"
if [[ "$smoke_version" != "$version_output" ]]; then
    printf 'error: packaged binary reported %s, expected %s\n' \
        "$smoke_version" "$version_output" >&2
    exit 1
fi

mkdir -p "$output_dir"
install -m644 "${scratch_dir}/${archive_name}" "$archive_path"
(
    cd "$output_dir"
    sha256sum "$archive_name" > "${archive_name}.sha256"
)

printf 'Created %s\n' "$archive_path"
printf 'Created %s.sha256\n' "$archive_path"
printf 'Smoke check: %s\n' "$smoke_version"
