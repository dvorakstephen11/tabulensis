#!/usr/bin/env python3
"""Safely run rustfmt on an explicit, narrow file set."""

from __future__ import annotations

import argparse
import shlex
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


def run_git(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def staged_rust_files() -> list[str]:
    result = run_git(["diff", "--cached", "--name-only", "--diff-filter=ACMR", "--", "*.rs"])
    if result.returncode != 0:
        return []
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def worktree_rust_files() -> list[str]:
    result = run_git(["status", "--porcelain"])
    if result.returncode != 0:
        return []

    files = set()
    for line in result.stdout.splitlines():
        if not line:
            continue
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        if path.endswith(".rs"):
            files.add(path)
    return sorted(files)


def normalize_inputs(paths: list[str]) -> list[str]:
    normalized: list[str] = []
    for path in paths:
        p = Path(path)
        if p.suffix != ".rs":
            continue
        full = (ROOT / p).resolve() if not p.is_absolute() else p.resolve()
        try:
            rel = full.relative_to(ROOT)
            normalized.append(rel.as_posix())
        except ValueError:
            # Ignore files outside repo root.
            continue
    return sorted(set(normalized))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run rustfmt only on explicit files (prevents workspace-wide churn)."
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Rust files to format. If omitted, uses staged Rust files by default.",
    )
    parser.add_argument(
        "--worktree",
        action="store_true",
        help="When no files are passed, format all modified Rust files in the working tree.",
    )
    args = parser.parse_args()

    if args.files:
        files = normalize_inputs(args.files)
    elif args.worktree:
        files = worktree_rust_files()
    else:
        files = staged_rust_files()

    if not files:
        print("No Rust files selected for formatting.")
        return 0

    cmd = ["rustfmt", *files]
    print("Running:", " ".join(shlex.quote(part) for part in cmd))
    result = subprocess.run(cmd, cwd=ROOT)
    if result.returncode != 0:
        return result.returncode

    print(f"Formatted {len(files)} Rust file(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
