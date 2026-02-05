#!/usr/bin/env python3
"""Guardrail for perf-cycle artifact scope and retention quality."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
CYCLES_ROOT = ROOT / "benchmarks" / "perf_cycles"


@dataclass(frozen=True)
class CycleInfo:
    cycle_id: str
    path: Path
    tracked: bool
    complete: bool
    key: str | None
    pre_commit: str | None
    post_commit: str | None
    reason: str


def run_git(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def normalize_commit(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    trimmed = value.strip()
    if not trimmed:
        return None
    lowered = trimmed.lower()
    if lowered in {"none", "null", "unknown"}:
        return None
    return trimmed


def cycle_id_from_path(path: str) -> str | None:
    prefix = "benchmarks/perf_cycles/"
    if not path.startswith(prefix):
        return None
    rest = path[len(prefix) :]
    if not rest:
        return None
    cycle_id = rest.split("/", 1)[0].strip()
    return cycle_id or None


def changed_paths_from_refs(from_ref: str, to_ref: str) -> list[str]:
    result = run_git(["diff", "--name-only", "--diff-filter=ACMR", from_ref, to_ref])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git diff failed")
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def changed_paths_staged() -> list[str]:
    result = run_git(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git diff --cached failed")
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def changed_paths_worktree() -> list[str]:
    result = run_git(["status", "--porcelain", "--", "benchmarks/perf_cycles"])
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


def tracked_cycle_ids() -> set[str]:
    result = run_git(["ls-files", "benchmarks/perf_cycles"])
    if result.returncode != 0:
        return set()
    ids: set[str] = set()
    for line in result.stdout.splitlines():
        cid = cycle_id_from_path(line.strip())
        if cid:
            ids.add(cid)
    return ids


def cycle_ids_from_paths(paths: list[str]) -> list[str]:
    ids: set[str] = set()
    for path in paths:
        cid = cycle_id_from_path(path)
        if cid:
            ids.add(cid)
    return sorted(ids)


def cycle_ids_all() -> list[str]:
    if not CYCLES_ROOT.exists():
        return []
    return sorted(
        p.name
        for p in CYCLES_ROOT.iterdir()
        if p.is_dir()
    )


def load_json(path: Path) -> dict | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def stage_json_path(meta: dict, stage_key: str, json_key: str, fallback_name: str) -> Path:
    stage = meta.get(stage_key)
    if isinstance(stage, dict):
        rel = stage.get(json_key)
        if isinstance(rel, str) and rel.strip():
            return ROOT / rel
    return CYCLES_ROOT / meta.get("cycle", "") / fallback_name


def stage_commit(meta: dict, stage_key: str) -> str | None:
    stage = meta.get(stage_key)
    if not isinstance(stage, dict):
        return None
    return normalize_commit(stage.get("git_commit"))


def inspect_cycle(cycle_id: str, tracked_ids: set[str]) -> CycleInfo:
    path = CYCLES_ROOT / cycle_id
    tracked = cycle_id in tracked_ids
    if not path.exists():
        return CycleInfo(
            cycle_id=cycle_id,
            path=path,
            tracked=tracked,
            complete=False,
            key=None,
            pre_commit=None,
            post_commit=None,
            reason="missing cycle directory",
        )

    meta_path = path / "cycle.json"
    if not meta_path.exists():
        return CycleInfo(
            cycle_id=cycle_id,
            path=path,
            tracked=tracked,
            complete=False,
            key=None,
            pre_commit=None,
            post_commit=None,
            reason="missing cycle.json",
        )

    meta = load_json(meta_path)
    if meta is None:
        return CycleInfo(
            cycle_id=cycle_id,
            path=path,
            tracked=tracked,
            complete=False,
            key=None,
            pre_commit=None,
            post_commit=None,
            reason="invalid cycle.json",
        )

    pre_commit = stage_commit(meta, "pre")
    post_commit = stage_commit(meta, "post")
    missing: list[str] = []

    required_paths = [
        stage_json_path(meta, "pre", "fullscale_json", "pre_fullscale.json"),
        stage_json_path(meta, "pre", "e2e_json", "pre_e2e.json"),
        stage_json_path(meta, "post", "fullscale_json", "post_fullscale.json"),
        stage_json_path(meta, "post", "e2e_json", "post_e2e.json"),
        path / "cycle_delta.md",
    ]
    for required in required_paths:
        if not required.exists():
            try:
                missing.append(str(required.relative_to(ROOT)))
            except ValueError:
                missing.append(str(required))

    if missing:
        return CycleInfo(
            cycle_id=cycle_id,
            path=path,
            tracked=tracked,
            complete=False,
            key=None,
            pre_commit=pre_commit,
            post_commit=post_commit,
            reason=f"incomplete artifacts ({'; '.join(missing)})",
        )

    if pre_commit is None or post_commit is None:
        return CycleInfo(
            cycle_id=cycle_id,
            path=path,
            tracked=tracked,
            complete=False,
            key=None,
            pre_commit=pre_commit,
            post_commit=post_commit,
            reason="missing pre/post git_commit in cycle.json",
        )

    key = f"{pre_commit}->{post_commit}"
    return CycleInfo(
        cycle_id=cycle_id,
        path=path,
        tracked=tracked,
        complete=True,
        key=key,
        pre_commit=pre_commit,
        post_commit=post_commit,
        reason="ok",
    )


def classify_cycles(infos: list[CycleInfo], max_per_key: int) -> tuple[list[CycleInfo], list[CycleInfo]]:
    incomplete = [info for info in infos if not info.complete]
    by_key: dict[str, list[CycleInfo]] = {}
    for info in infos:
        if not info.complete or info.key is None:
            continue
        by_key.setdefault(info.key, []).append(info)

    duplicates: list[CycleInfo] = []
    for key in by_key:
        rows = sorted(by_key[key], key=lambda info: info.cycle_id)
        if len(rows) <= max_per_key:
            continue
        duplicates.extend(rows[: len(rows) - max_per_key])

    return incomplete, duplicates


def prune_cycles(to_delete: list[CycleInfo], include_tracked: bool) -> tuple[list[str], list[str]]:
    deleted: list[str] = []
    skipped: list[str] = []
    for info in to_delete:
        if info.tracked and not include_tracked:
            skipped.append(f"{info.cycle_id} (tracked in git)")
            continue
        if not info.path.exists():
            continue
        try:
            shutil.rmtree(info.path)
            deleted.append(info.cycle_id)
        except OSError as exc:
            skipped.append(f"{info.cycle_id} ({exc})")
    return deleted, skipped


def summarize(infos: list[CycleInfo]) -> None:
    print("Perf-cycle scope summary:")
    print(f"  cycles inspected: {len(infos)}")
    tracked = sum(1 for info in infos if info.tracked)
    complete = sum(1 for info in infos if info.complete)
    print(f"  tracked cycles in scope: {tracked}")
    print(f"  complete cycles in scope: {complete}")
    for info in sorted(infos, key=lambda row: row.cycle_id):
        tracked_mark = "tracked" if info.tracked else "local"
        status = "complete" if info.complete else "incomplete"
        key = info.key or "n/a"
        print(f"  - {info.cycle_id}: {status}, {tracked_mark}, key={key}")
        if not info.complete:
            print(f"      reason: {info.reason}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Enforce perf-cycle retention quality within a scope."
    )
    parser.add_argument("--from-ref", type=str, default=None, help="Start git ref")
    parser.add_argument("--to-ref", type=str, default="HEAD", help="End git ref")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Inspect staged changes instead of worktree",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Inspect all cycle directories under benchmarks/perf_cycles",
    )
    parser.add_argument(
        "--max-per-key",
        type=int,
        default=1,
        help="Maximum complete cycles per (pre_commit -> post_commit) key",
    )
    parser.add_argument(
        "--apply-prune",
        action="store_true",
        help="Delete cycles that violate the rule (local scope only)",
    )
    parser.add_argument(
        "--include-tracked",
        action="store_true",
        help="Allow pruning tracked cycle dirs (dangerous; default skips tracked)",
    )
    parser.add_argument(
        "--allow-env",
        type=str,
        default="EXCEL_DIFF_ALLOW_MULTI_CYCLE",
        help="Environment variable to bypass guard when set to 1/true",
    )
    parser.add_argument(
        "--allow-token",
        type=str,
        default="",
        help="Commit-message token that bypasses guard in --from-ref mode",
    )
    args = parser.parse_args()

    if args.max_per_key < 1:
        print("ERROR: --max-per-key must be >= 1")
        return 2

    allow_env = os.environ.get(args.allow_env, "").lower() in {"1", "true", "yes"}
    if allow_env:
        print(f"Perf-cycle scope guard bypassed via {args.allow_env}=1")
        return 0

    if args.apply_prune and args.from_ref:
        print("ERROR: --apply-prune is not supported with --from-ref/--to-ref mode")
        return 2

    if args.apply_prune and args.staged:
        print("ERROR: --apply-prune is not supported with --staged mode")
        return 2

    if args.staged and args.all:
        print("ERROR: --staged and --all are mutually exclusive")
        return 2

    if args.from_ref and args.all:
        print("ERROR: --from-ref and --all are mutually exclusive")
        return 2

    try:
        if args.from_ref:
            paths = changed_paths_from_refs(args.from_ref, args.to_ref)
            if args.allow_token:
                messages = commit_messages(args.from_ref, args.to_ref)
                if args.allow_token in messages:
                    print(
                        f"Perf-cycle scope guard bypassed via commit token {args.allow_token!r} "
                        f"in range {args.from_ref}..{args.to_ref}"
                    )
                    return 0
            cycle_ids = cycle_ids_from_paths(paths)
        elif args.staged:
            cycle_ids = cycle_ids_from_paths(changed_paths_staged())
        elif args.all:
            cycle_ids = cycle_ids_all()
        else:
            cycle_ids = cycle_ids_from_paths(changed_paths_worktree())
    except RuntimeError as exc:
        print(f"ERROR: {exc}")
        return 2

    if not cycle_ids:
        print("Perf-cycle scope summary:")
        print("  no perf-cycle directories in scope")
        return 0

    tracked_ids = tracked_cycle_ids()
    infos = [inspect_cycle(cycle_id, tracked_ids) for cycle_id in cycle_ids]

    if args.apply_prune:
        incomplete, duplicates = classify_cycles(infos, args.max_per_key)
        to_delete = sorted(
            {info.cycle_id: info for info in (incomplete + duplicates)}.values(),
            key=lambda row: row.cycle_id,
        )
        if to_delete:
            deleted, skipped = prune_cycles(to_delete, include_tracked=args.include_tracked)
            if deleted:
                print("Pruned cycles:")
                for cycle_id in deleted:
                    print(f"  - {cycle_id}")
            if skipped:
                print("Skipped prune:")
                for item in skipped:
                    print(f"  - {item}")

        remaining_ids = [cycle_id for cycle_id in cycle_ids if (CYCLES_ROOT / cycle_id).exists()]
        infos = [inspect_cycle(cycle_id, tracked_ids) for cycle_id in remaining_ids]

    summarize(infos)

    incomplete, duplicates = classify_cycles(infos, args.max_per_key)
    if incomplete or duplicates:
        print("\nPerf-cycle scope guard failed:")
        if incomplete:
            print("  incomplete cycles:")
            for info in incomplete:
                print(f"    - {info.cycle_id}: {info.reason}")
        if duplicates:
            print("  duplicate cycles for same commit-pair key:")
            for info in duplicates:
                print(f"    - {info.cycle_id}: {info.key}")
        print(
            f"\nFix by pruning extras/incomplete cycles, or bypass with {args.allow_env}=1. "
            "In ref-range mode you can also use --allow-token with an explicit commit token."
        )
        return 1

    print("Perf-cycle scope guard passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
