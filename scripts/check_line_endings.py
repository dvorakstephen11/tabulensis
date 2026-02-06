#!/usr/bin/env python3
"""Guardrail to prevent CRLF / mixed line-endings churn in diffs."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
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


@dataclass(frozen=True)
class EolInfo:
    path: str
    exists: bool
    binary: bool
    crlf: int
    cr: int
    lf: int


def read_eol_info(path: Path) -> EolInfo:
    if not path.exists() or not path.is_file():
        return EolInfo(
            path=str(path),
            exists=False,
            binary=False,
            crlf=0,
            cr=0,
            lf=0,
        )

    data = path.read_bytes()
    if b"\x00" in data:
        return EolInfo(
            path=str(path),
            exists=True,
            binary=True,
            crlf=0,
            cr=0,
            lf=0,
        )

    crlf = data.count(b"\r\n")
    cr = data.count(b"\r")
    lf = data.count(b"\n")
    return EolInfo(
        path=str(path),
        exists=True,
        binary=False,
        crlf=crlf,
        cr=cr,
        lf=lf,
    )


def normalize_to_lf(path: Path) -> bool:
    """Convert CRLF / CR line endings to LF. Returns True when a write occurred."""
    if not path.exists() or not path.is_file():
        return False
    data = path.read_bytes()
    if b"\x00" in data:
        return False
    if b"\r" not in data:
        return False
    out = data.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    if out == data:
        return False
    path.write_bytes(out)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Fail when changed files contain CRLF / mixed line endings."
    )
    parser.add_argument("--from-ref", type=str, default=None, help="Start git ref")
    parser.add_argument("--to-ref", type=str, default="HEAD", help="End git ref")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Check staged files instead of working tree or ref range",
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Rewrite offending files in-place (CRLF -> LF).",
    )
    parser.add_argument(
        "--allow-env",
        type=str,
        default="EXCEL_DIFF_ALLOW_CRLF",
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
        print(f"EOL guard bypassed via {args.allow_env}=1")
        return 0

    try:
        if args.from_ref:
            files = changed_files_from_refs(args.from_ref, args.to_ref)
            if args.allow_token:
                messages = commit_messages(args.from_ref, args.to_ref)
                if args.allow_token in messages:
                    print(
                        f"EOL guard bypassed via commit token {args.allow_token!r} "
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

    if not files:
        print("EOL guard summary:")
        print("  no files in scope")
        return 0

    infos: list[EolInfo] = []
    fixed: list[str] = []
    for rel in files:
        path = ROOT / rel
        if args.fix:
            if normalize_to_lf(path):
                fixed.append(rel)
        infos.append(read_eol_info(path))

    offenders = [
        info
        for info in infos
        if info.exists and (not info.binary) and info.cr > 0
    ]
    binaries = [info for info in infos if info.exists and info.binary]
    missing = [info for info in infos if not info.exists]

    print("EOL guard summary:")
    print(f"  files in scope: {len(files)}")
    if binaries:
        print(f"  skipped binary files: {len(binaries)}")
    if missing:
        print(f"  missing paths: {len(missing)}")
    if fixed:
        print(f"  fixed files: {len(fixed)}")

    if offenders:
        print("\nCRLF / CR detected (should be LF-only):")
        for info in offenders:
            rel = info.path
            try:
                rel = str(Path(info.path).relative_to(ROOT))
            except ValueError:
                pass
            mixed = "mixed" if (info.crlf > 0 and info.lf != info.crlf) else "crlf-only"
            print(
                f"  - {rel}: crlf={info.crlf} cr={info.cr} lf={info.lf} ({mixed})"
            )

        if not args.fix:
            print(
                "\nFix options:\n"
                "  1) Re-run with --fix\n"
                "  2) Configure your editor to use LF (see .editorconfig)\n"
            )
        return 1

    print("EOL guard passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

