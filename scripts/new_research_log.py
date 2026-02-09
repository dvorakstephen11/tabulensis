#!/usr/bin/env python3
"""
Create a research log from a template (file-based when present, otherwise built-in).

Creates:
  docs/meta/logs/research/YYYY-MM-DD_<slug>.md

Rules:
  - Requires an explicit <slug> argument (example: acquisition_targets).
  - Refuses to overwrite an existing file.
  - Prints the created path to stdout (repo-relative when possible).
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date
from pathlib import Path


DEFAULT_TEMPLATE = """# Research Log

## Query

## Sources

## Findings

## Actionable experiments

## Append-only log entry
"""


def _format_path(repo_root: Path, path: Path) -> str:
    try:
        rel = path.resolve().relative_to(repo_root.resolve())
        return rel.as_posix()
    except Exception:
        return str(path)


def _print_path(repo_root: Path, path: Path) -> str:
    s = _format_path(repo_root, path)
    sys.stdout.write(s + "\n")
    return s


_SLUG_RE = re.compile(r"^[a-z0-9_]+$")


def _parse_slug(value: str) -> str:
    slug = value.strip()
    if not slug:
        raise argparse.ArgumentTypeError("slug must be non-empty")
    if slug != value:
        raise argparse.ArgumentTypeError("slug must not include leading/trailing whitespace")
    if not _SLUG_RE.fullmatch(slug):
        raise argparse.ArgumentTypeError("slug must be ASCII lower_snake_case (letters/digits/underscore)")
    if slug.startswith("_") or slug.endswith("_") or "__" in slug:
        raise argparse.ArgumentTypeError("slug must be lower_snake_case (no leading/trailing/double underscores)")
    return slug


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Create a research log from template.")
    parser.add_argument("slug", type=_parse_slug, help="Identifier in lower_snake_case (e.g., acquisition_targets).")
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent
    template_path = repo_root / "docs/meta/logs/research/_TEMPLATE.md"
    template_text = DEFAULT_TEMPLATE
    if template_path.exists():
        template_text = template_path.read_text(encoding="utf-8", errors="replace")
        template_text = template_text.replace("\r\n", "\n").replace("\r", "\n")

    out_dir = repo_root / "docs/meta/logs/research"
    out_dir.mkdir(parents=True, exist_ok=True)

    yyyy_mm_dd = date.today().strftime("%Y-%m-%d")
    out_path = out_dir / f"{yyyy_mm_dd}_{args.slug}.md"

    try:
        # Use 'x' to make overwrite refusal atomic.
        with out_path.open("x", encoding="utf-8", newline="\n") as f:
            f.write(template_text)
    except FileExistsError:
        sys.stderr.write(f"Refusing to overwrite existing file: {out_path}\n")
        return 1

    _print_path(repo_root, out_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

