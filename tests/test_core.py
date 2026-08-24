from __future__ import annotations

import json
import unittest

from omapack.core import OmapackError, PackageFile, package_digest, parse_package_manifest, parse_source


def manifest(**changes: object) -> bytes:
    value: dict[str, object] = {
        "schemaVersion": 1,
        "name": "oma-ui",
        "license": "MIT",
        "files": ["Ui/Button.qml", "Ui/qmldir"],
    }
    value.update(changes)
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


class SourceTests(unittest.TestCase):
    def test_release_in_subdirectory_uses_prefixed_tag(self) -> None:
        source = parse_source("github:silouanwright/omatools/packages/oma-ui@0.2.0")
        self.assertEqual(source.release_tag, "packages/oma-ui/v0.2.0")
        self.assertEqual(source.version, "0.2.0")

    def test_exact_commit_has_no_version(self) -> None:
        source = parse_source("github:silouanwright/omatools/oma-ui@" + "a" * 40)
        self.assertIsNone(source.release_tag)
        self.assertIsNone(source.version)

    def test_rejects_branches_and_traversal(self) -> None:
        for value in (
            "github:owner/repo/pkg@main",
            "github:owner/repo/../pkg@1.0.0",
            "github:owner/repo/pkg/@1.0.0",
            "github:owner/repo/pkg@1.0.0-01",
            "https://github.com/owner/repo@1.0.0",
        ):
            with self.subTest(value=value), self.assertRaises(OmapackError):
                parse_source(value)


class ManifestTests(unittest.TestCase):
    def test_parses_strict_manifest(self) -> None:
        parsed = parse_package_manifest(
            manifest(
                dependencies={"reader": "github:silouanwright/omatools/reader@v0.1.0"},
                compatibility={"omarchy": ">=4 <5"},
                executables=["Ui/qmldir"],
            )
        )
        self.assertEqual(parsed.name, "oma-ui")
        self.assertEqual(parsed.dependencies["reader"].release_tag, "reader/v0.1.0")
        self.assertIn("Ui/qmldir", parsed.executables)

    def test_rejects_duplicate_json_keys(self) -> None:
        with self.assertRaisesRegex(OmapackError, "duplicate JSON key"):
            parse_package_manifest(b'{"schemaVersion":1,"name":"x","name":"y"}')

    def test_rejects_colliding_and_reserved_paths(self) -> None:
        for files in (["Ui/A.qml", "ui/a.qml"], ["omapack.json"]):
            with self.subTest(files=files), self.assertRaises(OmapackError):
                parse_package_manifest(manifest(files=files))

    def test_rejects_undeclared_executable(self) -> None:
        with self.assertRaisesRegex(OmapackError, "executable"):
            parse_package_manifest(manifest(executables=["bin/tool"]))


class DigestTests(unittest.TestCase):
    def test_digest_is_order_independent_and_content_sensitive(self) -> None:
        parsed = parse_package_manifest(manifest())
        files = [
            PackageFile("Ui/Button.qml", b"import QtQuick\n"),
            PackageFile("Ui/qmldir", b"Button 1.0 Button.qml\n"),
        ]
        first = package_digest(parsed, files)
        self.assertEqual(
            first,
            "sha256:c4cdce50e1a09b2f3406e98bc226c0f237bc6b6c5d1b1dd4d584a134d73c146f",
        )
        self.assertEqual(first, package_digest(parsed, reversed(files)))
        changed = [files[0], PackageFile("Ui/qmldir", b"Button 1.1 Button.qml\n")]
        self.assertNotEqual(first, package_digest(parsed, changed))

    def test_digest_rejects_wrong_file_set_or_mode(self) -> None:
        parsed = parse_package_manifest(manifest())
        with self.assertRaisesRegex(OmapackError, "do not match"):
            package_digest(parsed, [PackageFile("Ui/Button.qml", b"")])
        with self.assertRaisesRegex(OmapackError, "mode"):
            package_digest(
                parsed,
                [PackageFile("Ui/Button.qml", b"", True), PackageFile("Ui/qmldir", b"")],
            )


if __name__ == "__main__":
    unittest.main()
