#!/usr/bin/env python3
"""
Verify that release tags match crate versions.

Branch 5 packaging expects releases to be tagged as `vX.Y.Z`, and for the
workspace crate versions to match `X.Y.Z` (without the leading `v`).

This script is designed to be used in GitHub Actions, but can also be run
locally:

  python scripts/verify_release_versions.py --tag v0.1.0
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path


TAG_RE = re.compile(
    r"^v(?P<version>[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)$"
)


def _read_package_version(cargo_toml_path: Path) -> str:
    if not cargo_toml_path.exists():
        raise FileNotFoundError(f"Missing {cargo_toml_path}")

    if sys.version_info >= (3, 11):
        import tomllib  # type: ignore[import-not-found]
    else:
        raise RuntimeError("Python 3.11+ is required (tomllib)")

    data = tomllib.loads(cargo_toml_path.read_text(encoding="utf-8"))
    package = data.get("package")
    if not isinstance(package, dict):
        raise ValueError(f"{cargo_toml_path} has no [package] section")

    version = package.get("version")
    if not isinstance(version, str) or not version.strip():
        raise ValueError(f"{cargo_toml_path} has no valid package.version")

    return version.strip()


def _resolve_tag(tag_arg: str | None) -> str | None:
    if tag_arg:
        return tag_arg.strip()

    ref = os.environ.get("GITHUB_REF", "").strip()
    if ref.startswith("refs/tags/"):
        return ref.removeprefix("refs/tags/")

    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify tag/version consistency.")
    parser.add_argument("--tag", help="Tag like v0.1.0 (defaults to GITHUB_REF)")
    parser.add_argument(
        "--workspace-root",
        default=str(Path(__file__).resolve().parent.parent),
        help="Repository root (default: inferred from script location)",
    )
    parser.add_argument(
        "--crates",
        nargs="*",
        default=["core", "cli", "wasm", "ui_payload", "desktop/backend", "desktop/wx"],
        help="Crate directories to check (default: core cli wasm ui_payload desktop/backend desktop/wx)",
    )
    args = parser.parse_args()

    tag = _resolve_tag(args.tag)
    if tag is None:
        print("No tag detected (not running on refs/tags/*); skipping version check.")
        return 0

    match = TAG_RE.match(tag)
    if not match:
        print(f"ERROR: tag {tag!r} does not match expected format vX.Y.Z", file=sys.stderr)
        return 2

    expected = match.group("version")
    root = Path(args.workspace_root)

    mismatches: list[tuple[str, str]] = []
    for crate_dir in args.crates:
        cargo_toml = root / crate_dir / "Cargo.toml"
        actual = _read_package_version(cargo_toml)
        if actual != expected:
            mismatches.append((crate_dir, actual))

    if mismatches:
        print(
            f"ERROR: tag {tag} expects crate version {expected}, but found mismatches:",
            file=sys.stderr,
        )
        for crate_dir, actual in mismatches:
            print(f"  - {crate_dir}/Cargo.toml: {actual}", file=sys.stderr)
        return 1

    print(f"OK: tag {tag} matches crate versions ({expected}).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

