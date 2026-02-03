#!/usr/bin/env python3
"""
Run a required perf cycle around Rust code changes.

Workflow:
  1) Before edits:  python3 scripts/perf_cycle.py pre
  2) After edits:   python3 scripts/perf_cycle.py post --cycle <cycle_id>

This runs the full-scale perf suite + e2e suite twice and produces a delta report.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


RUST_SENTINELS = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def run(cmd: list[str], cwd: Path) -> None:
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def git_cmd(root: Path, args: list[str]) -> str:
    try:
        result = subprocess.run(
            ["git"] + args,
            cwd=root,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass
    return "unknown"


def rust_changes(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=root,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []

    paths = []
    for line in result.stdout.splitlines():
        if not line:
            continue
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        if path.endswith(".rs") or Path(path).name in RUST_SENTINELS:
            paths.append(path)
    return sorted(set(paths))


def now_id() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")


def ensure_cycle_dir(root: Path, cycle: str) -> Path:
    path = root / "benchmarks" / "perf_cycles" / cycle
    path.mkdir(parents=True, exist_ok=True)
    return path


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def run_fullscale(root: Path, cycle_dir: Path, label: str, parallel: bool) -> Path:
    output_json = cycle_dir / f"{label}_fullscale.json"
    output_csv = cycle_dir / f"{label}_fullscale.csv"

    cmd = [
        sys.executable,
        "scripts/check_perf_thresholds.py",
        "--suite",
        "full-scale",
    ]
    if parallel:
        cmd.append("--parallel")
    cmd.extend(
        [
            "--require-baseline",
            "--baseline",
            "benchmarks/baselines/full-scale.json",
            "--export-json",
            str(output_json),
            "--export-csv",
            str(output_csv),
        ]
    )
    run(cmd, root)
    return output_json


def run_e2e(root: Path, cycle_dir: Path, label: str, skip_fixtures: bool) -> Path:
    output_json = cycle_dir / f"{label}_e2e.json"
    output_csv = cycle_dir / f"{label}_e2e.csv"
    output_dir = cycle_dir / f"results_e2e_{label}"

    cmd = [
        sys.executable,
        "scripts/export_e2e_metrics.py",
        "--baseline",
        "benchmarks/baselines/e2e.json",
        "--latest-json",
        str(output_json),
        "--export-csv",
        str(output_csv),
        "--output-dir",
        str(output_dir),
    ]
    if skip_fixtures:
        cmd.append("--skip-fixtures")
    run(cmd, root)
    return output_json


def format_delta(pre: int | None, post: int | None) -> str:
    if pre is None or post is None:
        return "n/a"
    if pre == 0:
        return f"{post - pre:+} ms"
    delta = post - pre
    pct = (delta / pre) * 100.0
    return f"{delta:+} ms ({pct:+.1f}%)"


def delta_table(pre_tests: dict, post_tests: dict, keys: list[str], fields: list[str]) -> list[list[str]]:
    rows = []
    for key in keys:
        pre = pre_tests.get(key, {})
        post = post_tests.get(key, {})
        row = [f"`{key}`"]
        for field in fields:
            pre_val = pre.get(field)
            post_val = post.get(field)
            row.append(str(pre_val) if pre_val is not None else "n/a")
            row.append(str(post_val) if post_val is not None else "n/a")
            row.append(format_delta(pre_val, post_val))
        rows.append(row)
    return rows


def render_markdown_summary(
    cycle: str,
    pre_meta: dict,
    post_meta: dict,
    full_rows: list[list[str]],
    e2e_rows: list[list[str]],
) -> str:
    lines = [
        f"# Perf Cycle Delta Summary",
        "",
        f"Cycle: `{cycle}`",
        f"Pre: `{pre_meta.get('git_commit')}` ({pre_meta.get('git_branch')}) at {pre_meta.get('timestamp')}",
        f"Post: `{post_meta.get('git_commit')}` ({post_meta.get('git_branch')}) at {post_meta.get('timestamp')}",
        "",
        "## Full-scale (total_time_ms)",
        "| Test | Pre | Post | Delta |",
        "| --- | --- | --- | --- |",
    ]
    for row in full_rows:
        lines.append(f"| {row[0]} | {row[1]} | {row[2]} | {row[3]} |")
    lines.extend(
        [
            "",
            "## E2E (total/parse/diff time)",
            "| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |",
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for row in e2e_rows:
        lines.append(
            f"| {row[0]} | {row[1]} | {row[2]} | {row[3]} | {row[4]} | {row[5]} | {row[6]} | {row[7]} | {row[8]} | {row[9]} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run perf cycle (pre/post) for Rust changes")
    sub = parser.add_subparsers(dest="command", required=True)

    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--cycle", type=str, default=None, help="Cycle id (timestamp)")
    common.add_argument(
        "--skip-fixtures",
        action="store_true",
        help="Skip fixture generation for e2e",
    )
    common.add_argument(
        "--no-parallel",
        action="store_true",
        help="Disable the parallel feature for perf tests",
    )
    common.add_argument(
        "--allow-dirty",
        action="store_true",
        help="Allow Rust changes to exist when starting a pre cycle",
    )

    sub.add_parser("pre", parents=[common], help="Run pre-change perf + e2e baseline")
    sub.add_parser("post", parents=[common], help="Run post-change perf + e2e and compare")

    args = parser.parse_args()

    root = repo_root()
    cycle = args.cycle or now_id()
    cycle_dir = ensure_cycle_dir(root, cycle)
    meta_path = cycle_dir / "cycle.json"

    rust_dirty = rust_changes(root)
    if args.command == "pre" and rust_dirty and not args.allow_dirty:
        print("ERROR: Rust changes detected. Run pre cycle before editing Rust files.")
        print("Changed Rust paths:")
        for path in rust_dirty:
            print(f"  - {path}")
        print("If you must proceed anyway, re-run with --allow-dirty.")
        return 2

    if args.command == "post" and not rust_dirty:
        print("WARNING: No Rust changes detected in working tree.")

    git_commit = git_cmd(root, ["rev-parse", "HEAD"])[:12]
    git_branch = git_cmd(root, ["rev-parse", "--abbrev-ref", "HEAD"])
    timestamp = datetime.now(timezone.utc).isoformat()

    parallel = not args.no_parallel

    if args.command == "pre":
        if meta_path.exists():
            print(f"WARNING: cycle metadata already exists at {meta_path}")
        pre_full = run_fullscale(root, cycle_dir, "pre", parallel)
        pre_e2e = run_e2e(root, cycle_dir, "pre", args.skip_fixtures)
        meta = {
            "cycle": cycle,
            "pre": {
                "timestamp": timestamp,
                "git_commit": git_commit,
                "git_branch": git_branch,
                "fullscale_json": str(pre_full.relative_to(root)),
                "e2e_json": str(pre_e2e.relative_to(root)),
            },
        }
        write_json(meta_path, meta)
        print(f"Pre-cycle complete. Cycle id: {cycle}")
        return 0

    pre_full_path = cycle_dir / "pre_fullscale.json"
    pre_e2e_path = cycle_dir / "pre_e2e.json"
    if not pre_full_path.exists() or not pre_e2e_path.exists():
        print("ERROR: Missing pre-cycle results. Run: python3 scripts/perf_cycle.py pre")
        return 2

    post_full = run_fullscale(root, cycle_dir, "post", parallel)
    post_e2e = run_e2e(root, cycle_dir, "post", args.skip_fixtures)

    meta = load_json(meta_path) if meta_path.exists() else {"cycle": cycle}
    meta["post"] = {
        "timestamp": timestamp,
        "git_commit": git_commit,
        "git_branch": git_branch,
        "fullscale_json": str(post_full.relative_to(root)),
        "e2e_json": str(post_e2e.relative_to(root)),
    }
    write_json(meta_path, meta)

    pre_full = load_json(pre_full_path)
    post_full_data = load_json(post_full)
    pre_e2e = load_json(pre_e2e_path)
    post_e2e_data = load_json(post_e2e)

    full_keys = sorted(set(pre_full.get("tests", {}).keys()) | set(post_full_data.get("tests", {}).keys()))
    e2e_keys = sorted(set(pre_e2e.get("tests", {}).keys()) | set(post_e2e_data.get("tests", {}).keys()))

    full_rows = delta_table(
        pre_full.get("tests", {}),
        post_full_data.get("tests", {}),
        full_keys,
        ["total_time_ms"],
    )
    e2e_rows = delta_table(
        pre_e2e.get("tests", {}),
        post_e2e_data.get("tests", {}),
        e2e_keys,
        ["total_time_ms", "parse_time_ms", "diff_time_ms"],
    )

    summary_md = render_markdown_summary(
        cycle,
        meta.get("pre", {}),
        meta.get("post", {}),
        full_rows,
        e2e_rows,
    )
    summary_path = cycle_dir / "cycle_delta.md"
    write_text(summary_path, summary_md)

    summary_json = {
        "cycle": cycle,
        "pre": meta.get("pre", {}),
        "post": meta.get("post", {}),
        "fullscale": full_rows,
        "e2e": e2e_rows,
    }
    write_json(cycle_dir / "cycle_delta.json", summary_json)

    print(f"Post-cycle complete. Delta summary written to {summary_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
