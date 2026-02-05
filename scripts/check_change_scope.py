#!/usr/bin/env python3
"""Guardrail script to catch suspiciously wide file-change scope."""

from __future__ import annotations

import argparse
import os
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


def changed_files_from_refs(from_ref: str, to_ref: str) -> list[str]:
    result = run_git(["diff", "--name-only", "--diff-filter=ACMR", from_ref, to_ref])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git diff failed")
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def changed_files_staged() -> list[str]:
    result = run_git(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git diff --cached failed")
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def changed_files_worktree() -> list[str]:
    result = run_git(["status", "--porcelain"])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git status failed")

    files = set()
    for line in result.stdout.splitlines():
        if not line:
            continue
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        if path:
            files.add(path)
    return sorted(files)


def commit_messages(from_ref: str, to_ref: str) -> str:
    result = run_git(["log", "--format=%B", f"{from_ref}..{to_ref}"])
    if result.returncode != 0:
        return ""
    return result.stdout


def summarize(files: list[str]) -> tuple[int, int, dict[str, int]]:
    rust = 0
    top_dirs: dict[str, int] = {}
    for path in files:
        if path.endswith(".rs"):
            rust += 1
        head = path.split("/", 1)[0]
        top_dirs[head] = top_dirs.get(head, 0) + 1
    return len(files), rust, dict(sorted(top_dirs.items(), key=lambda kv: (-kv[1], kv[0])))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Fail when change scope exceeds configured thresholds."
    )
    parser.add_argument("--from-ref", type=str, default=None, help="Start git ref")
    parser.add_argument("--to-ref", type=str, default="HEAD", help="End git ref")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Check staged files instead of working tree or ref range",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=120,
        help="Maximum changed files allowed before failure",
    )
    parser.add_argument(
        "--max-rust-files",
        type=int,
        default=70,
        help="Maximum changed Rust files allowed before failure",
    )
    parser.add_argument(
        "--allow-env",
        type=str,
        default="EXCEL_DIFF_ALLOW_WIDE_SCOPE",
        help="Environment variable to bypass guard when set to 1/true",
    )
    parser.add_argument(
        "--allow-token",
        type=str,
        default="",
        help="Commit-message token that bypasses guard for --from-ref/--to-ref mode",
    )
    args = parser.parse_args()

    allow_env = os.environ.get(args.allow_env, "").lower() in {"1", "true", "yes"}
    if allow_env:
        print(f"Scope guard bypassed via {args.allow_env}=1")
        return 0

    try:
        if args.from_ref:
            files = changed_files_from_refs(args.from_ref, args.to_ref)
            if args.allow_token:
                messages = commit_messages(args.from_ref, args.to_ref)
                if args.allow_token in messages:
                    print(
                        f"Scope guard bypassed via commit token {args.allow_token!r} "
                        f"in range {args.from_ref}..{args.to_ref}"
                    )
                    return 0
        elif args.staged:
            files = changed_files_staged()
        else:
            files = changed_files_worktree()
    except RuntimeError as exc:
        print(f"ERROR: {exc}")
        return 2

    total, rust_total, top_dirs = summarize(files)

    print("Scope guard summary:")
    print(f"  changed files: {total}")
    print(f"  changed Rust files: {rust_total}")
    if top_dirs:
        print("  top directories:")
        for directory, count in list(top_dirs.items())[:12]:
            print(f"    - {directory}: {count}")

    failures = []
    if total > args.max_files:
        failures.append(
            f"changed files {total} exceeds limit {args.max_files}"
        )
    if rust_total > args.max_rust_files:
        failures.append(
            f"changed Rust files {rust_total} exceeds limit {args.max_rust_files}"
        )

    if failures:
        print("\nScope guard failed:")
        for failure in failures:
            print(f"  - {failure}")
        print(
            f"\nIf this blast radius is intentional, set {args.allow_env}=1 "
            "or (in ref-range mode) include the allow token in a commit message."
        )
        return 1

    print("Scope guard passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
