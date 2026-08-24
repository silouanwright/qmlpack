"""Strict package primitives shared by transport, review, and installation."""

from __future__ import annotations

import hashlib
import json
import re
import struct
import unicodedata
from dataclasses import dataclass
from pathlib import PurePosixPath
from typing import Any, Iterable, Mapping

MANIFEST_LIMIT = 64 * 1024
FILE_LIMIT = 4 * 1024 * 1024
PACKAGE_LIMIT = 16 * 1024 * 1024
FILES_LIMIT = 256
DEPENDENCIES_LIMIT = 32
PACKAGES_LIMIT = 128
DEPENDENCY_DEPTH_LIMIT = 16

NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
REPO_PART_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$")
SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")
SEMVER_RE = re.compile(
    r"^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
RESERVED_COMPONENTS = {".git", ".omapack"}
RESERVED_FILES = {"omapack.json", "omapack.lock"}


class OmapackError(Exception):
    """A user-actionable package error."""


def _object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise OmapackError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_json(payload: bytes, *, limit: int = MANIFEST_LIMIT) -> Any:
    if len(payload) > limit:
        raise OmapackError(f"JSON exceeds {limit} bytes")
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise OmapackError("JSON must be UTF-8") from error
    try:
        return json.loads(
            text,
            object_pairs_hook=_object_without_duplicates,
            parse_constant=lambda value: (_ for _ in ()).throw(
                OmapackError(f"invalid JSON number: {value}")
            ),
        )
    except OmapackError:
        raise
    except json.JSONDecodeError as error:
        raise OmapackError(f"invalid JSON: {error.msg}") from error


def normalized_path(value: str) -> str:
    if not isinstance(value, str) or not value:
        raise OmapackError("file paths must be non-empty strings")
    if "\\" in value or any(ord(character) < 32 for character in value):
        raise OmapackError(f"unsafe file path: {value!r}")
    raw_parts = value.split("/")
    path = PurePosixPath(value)
    if (
        path.is_absolute()
        or len(value.encode("utf-8")) > 1024
        or any(part in ("", ".", "..") or len(part.encode("utf-8")) > 255 for part in raw_parts)
    ):
        raise OmapackError(f"unsafe file path: {value!r}")
    if any(part in RESERVED_COMPONENTS for part in path.parts):
        raise OmapackError(f"reserved file path: {value!r}")
    normalized = unicodedata.normalize("NFC", value)
    if normalized != value:
        raise OmapackError(f"file path must use Unicode NFC: {value!r}")
    if path.name in RESERVED_FILES:
        raise OmapackError(f"reserved file path: {value!r}")
    return value


def _require_object(value: Any, name: str) -> Mapping[str, Any]:
    if not isinstance(value, dict):
        raise OmapackError(f"{name} must be an object")
    return value


def _only_keys(value: Mapping[str, Any], allowed: set[str], name: str) -> None:
    unknown = sorted(set(value) - allowed)
    if unknown:
        raise OmapackError(f"unknown {name} field: {unknown[0]}")


@dataclass(frozen=True)
class Source:
    owner: str
    repository: str
    package_path: str
    requested: str

    @property
    def repository_name(self) -> str:
        return f"{self.owner}/{self.repository}"

    @property
    def canonical(self) -> str:
        path = f"/{self.package_path}" if self.package_path else ""
        return f"github:{self.repository_name}{path}@{self.requested}"

    @property
    def release_tag(self) -> str | None:
        match = SEMVER_RE.fullmatch(self.requested)
        if not match:
            return None
        version = self.requested if self.requested.startswith("v") else f"v{self.requested}"
        return f"{self.package_path}/{version}" if self.package_path else version

    @property
    def version(self) -> str | None:
        if self.release_tag is None:
            return None
        return self.requested.removeprefix("v")


def parse_source(value: str) -> Source:
    if not isinstance(value, str) or not value.startswith("github:"):
        raise OmapackError("source must start with github:")
    location, separator, requested = value.removeprefix("github:").rpartition("@")
    if not separator or not requested:
        raise OmapackError("source must end with @<version-or-commit>")
    parts = location.split("/")
    if len(parts) < 2 or not all(parts[:2]):
        raise OmapackError("GitHub source must include owner/repository")
    owner, repository = parts[:2]
    if not REPO_PART_RE.fullmatch(owner) or not REPO_PART_RE.fullmatch(repository):
        raise OmapackError("invalid GitHub owner or repository")
    package_path = "/".join(parts[2:])
    if package_path:
        # Package roots may contain omapack.json; validate components without
        # applying the distributed-file reserved-name rule.
        if "\\" in package_path or any(ord(character) < 32 for character in package_path):
            raise OmapackError("unsafe package path")
        raw_parts = package_path.split("/")
        if (
            len(package_path.encode("utf-8")) > 1024
            or any(
                part in ("", ".", "..", *RESERVED_COMPONENTS)
                or len(part.encode("utf-8")) > 255
                for part in raw_parts
            )
        ):
            raise OmapackError("unsafe package path")
        if unicodedata.normalize("NFC", package_path) != package_path:
            raise OmapackError("package path must use Unicode NFC")
    semver = SEMVER_RE.fullmatch(requested)
    if semver and semver.group(4):
        for identifier in semver.group(4).split("."):
            if identifier.isdigit() and len(identifier) > 1 and identifier.startswith("0"):
                raise OmapackError("numeric SemVer prerelease identifiers may not have leading zeroes")
    if not SHA_RE.fullmatch(requested) and not semver:
        raise OmapackError("reference must be an exact 40-character commit or SemVer")
    return Source(owner, repository, package_path, requested.lower() if SHA_RE.fullmatch(requested) else requested)


@dataclass(frozen=True)
class PackageManifest:
    raw: bytes
    name: str
    license: str
    files: tuple[str, ...]
    dependencies: Mapping[str, Source]
    compatibility: Mapping[str, str]
    executables: frozenset[str]


def parse_package_manifest(payload: bytes) -> PackageManifest:
    value = _require_object(strict_json(payload), "manifest")
    _only_keys(
        value,
        {"schemaVersion", "name", "license", "files", "dependencies", "compatibility", "executables"},
        "manifest",
    )
    if type(value.get("schemaVersion")) is not int or value["schemaVersion"] != 1:
        raise OmapackError("schemaVersion must be the integer 1")
    name = value.get("name")
    if not isinstance(name, str) or not NAME_RE.fullmatch(name):
        raise OmapackError("name must match [a-z0-9][a-z0-9-]*")
    license_name = value.get("license")
    if not isinstance(license_name, str) or not license_name.strip() or len(license_name) > 128:
        raise OmapackError("license must be a non-empty SPDX expression")

    raw_files = value.get("files")
    if not isinstance(raw_files, list) or not raw_files or len(raw_files) > FILES_LIMIT:
        raise OmapackError(f"files must contain 1 to {FILES_LIMIT} paths")
    files = tuple(normalized_path(path) for path in raw_files)
    if len(set(files)) != len(files):
        raise OmapackError("file paths must be unique")
    folded: dict[str, str] = {}
    for path in files:
        key = unicodedata.normalize("NFC", path).casefold()
        if key in folded:
            raise OmapackError(f"file paths collide: {folded[key]!r} and {path!r}")
        folded[key] = path

    raw_dependencies = value.get("dependencies", {})
    dependencies_object = _require_object(raw_dependencies, "dependencies")
    if len(dependencies_object) > DEPENDENCIES_LIMIT:
        raise OmapackError(f"packages may declare at most {DEPENDENCIES_LIMIT} dependencies")
    dependencies: dict[str, Source] = {}
    for label, source in dependencies_object.items():
        if not NAME_RE.fullmatch(label):
            raise OmapackError(f"invalid dependency label: {label!r}")
        dependencies[label] = parse_source(source)

    compatibility_object = _require_object(value.get("compatibility", {}), "compatibility")
    _only_keys(compatibility_object, {"omarchy", "quickshell"}, "compatibility")
    compatibility: dict[str, str] = {}
    for host, requirement in compatibility_object.items():
        if not isinstance(requirement, str) or not requirement.strip() or len(requirement) > 128:
            raise OmapackError(f"compatibility.{host} must be a non-empty string")
        compatibility[host] = requirement

    raw_executables = value.get("executables", [])
    if not isinstance(raw_executables, list):
        raise OmapackError("executables must be an array")
    executables = frozenset(normalized_path(path) for path in raw_executables)
    if not executables.issubset(files):
        raise OmapackError("every executable must also appear in files")

    return PackageManifest(
        raw=payload,
        name=name,
        license=license_name,
        files=files,
        dependencies=dependencies,
        compatibility=compatibility,
        executables=executables,
    )


@dataclass(frozen=True)
class PackageFile:
    path: str
    content: bytes
    executable: bool = False

    @property
    def mode(self) -> str:
        return "0755" if self.executable else "0644"

    @property
    def digest(self) -> str:
        return f"sha256:{hashlib.sha256(self.content).hexdigest()}"


def package_digest(manifest: PackageManifest, files: Iterable[PackageFile]) -> str:
    by_path = {file.path: file for file in files}
    if set(by_path) != set(manifest.files):
        missing = sorted(set(manifest.files) - set(by_path))
        extra = sorted(set(by_path) - set(manifest.files))
        raise OmapackError(f"package files do not match manifest (missing={missing}, extra={extra})")
    total = sum(len(file.content) for file in by_path.values())
    if total > PACKAGE_LIMIT:
        raise OmapackError(f"package exceeds {PACKAGE_LIMIT} bytes")

    digest = hashlib.sha256()
    digest.update(b"omapack-package-v1\0")
    digest.update(struct.pack(">Q", len(manifest.raw)))
    digest.update(manifest.raw)
    for path in sorted(manifest.files, key=lambda item: item.encode("utf-8")):
        file = by_path[path]
        if len(file.content) > FILE_LIMIT:
            raise OmapackError(f"file exceeds {FILE_LIMIT} bytes: {path}")
        expected_executable = path in manifest.executables
        if file.executable != expected_executable:
            raise OmapackError(f"file mode does not match manifest: {path}")
        encoded_path = path.encode("utf-8")
        digest.update(struct.pack(">Q", len(encoded_path)))
        digest.update(encoded_path)
        digest.update(file.mode.encode("ascii") + b"\0")
        digest.update(struct.pack(">Q", len(file.content)))
        digest.update(file.content)
    return f"sha256:{digest.hexdigest()}"
