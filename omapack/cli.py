"""Command-line entry point. Commands are added one secure slice at a time."""

from __future__ import annotations

import argparse
import sys

from . import __version__
from .core import OmapackError, parse_package_manifest, parse_source


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="omapack",
        description="Review-first source package management for Omarchy plugins.",
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    subcommands = parser.add_subparsers(dest="command", required=True)

    inspect_source = subcommands.add_parser("inspect-source", help=argparse.SUPPRESS)
    inspect_source.add_argument("source")

    check_manifest = subcommands.add_parser("check-manifest", help="validate a package manifest")
    check_manifest.add_argument("path", nargs="?", default="omapack.json")
    return parser


def run(arguments: argparse.Namespace) -> int:
    if arguments.command == "inspect-source":
        print(parse_source(arguments.source).canonical)
        return 0
    if arguments.command == "check-manifest":
        with open(arguments.path, "rb") as handle:
            manifest = parse_package_manifest(handle.read(64 * 1024 + 1))
        print(f"{manifest.name}: {len(manifest.files)} files, {len(manifest.dependencies)} dependencies")
        return 0
    raise OmapackError(f"unsupported command: {arguments.command}")


def main(argv: list[str] | None = None) -> int:
    try:
        return run(build_parser().parse_args(argv))
    except (OmapackError, OSError) as error:
        print(f"omapack: {error}", file=sys.stderr)
        return 1
