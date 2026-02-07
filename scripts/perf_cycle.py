#!/usr/bin/env python3
"""
Run a full pre/post perf cycle for major perf-risk Rust changes.

Workflow:
  1) Before edits:  python3 scripts/perf_cycle.py pre
  2) After edits:   python3 scripts/perf_cycle.py post --cycle <cycle_id>

By default each suite is executed 3 times and aggregated via median to reduce noise.
Use quick/gate suites for routine low-risk changes; reserve this script for major changes.
Post runs also emit cycle_signal.{md,json} for noise-aware confidence scoring.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import statistics
import subprocess
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path


RUST_SENTINELS = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}

FULLSCALE_CSV_FIELDS = [
    "total_time_ms",
    "parse_time_ms",
    "diff_time_ms",
    "move_detection_time_ms",
    "alignment_time_ms",
    "cell_diff_time_ms",
    "peak_memory_bytes",
    "rows_processed",
    "cells_compared",
    "anchors_found",
    "moves_detected",
]

E2E_CSV_FIELDS = [
    "total_time_ms",
    "parse_time_ms",
    "diff_time_ms",
    "signature_build_time_ms",
    "move_detection_time_ms",
    "alignment_time_ms",
    "cell_diff_time_ms",
    "op_emit_time_ms",
    "report_serialize_time_ms",
    "peak_memory_bytes",
    "grid_storage_bytes",
    "string_pool_bytes",
    "op_buffer_bytes",
    "alignment_buffer_bytes",
    "rows_processed",
    "cells_compared",
    "anchors_found",
    "moves_detected",
    "hash_lookups_est",
    "allocations_est",
    "old_bytes",
    "new_bytes",
    "total_input_bytes",
]

CLI_JSONL_CSV_FIELDS = [
    "total_time_ms",
    "op_emit_time_ms",
    "diff_time_ms",
    "op_count",
]

PERF_METRIC_PATTERN = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def run(cmd: list[str], cwd: Path) -> None:
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(result.returncode)

def run_capture(cmd: list[str], cwd: Path, timeout_s: int) -> subprocess.CompletedProcess:
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=timeout_s,
    )
    if result.returncode != 0:
        # Surface failing output to help diagnose flaky perf runs.
        if result.stdout:
            print(result.stdout)
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        raise SystemExit(result.returncode)
    return result


def run_optional(cmd: list[str], cwd: Path) -> bool:
    print(f"Running (optional): {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    return result.returncode == 0


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


def git_status_lines(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=root,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return [line for line in result.stdout.splitlines() if line]


def porcelain_paths(lines: list[str]) -> list[str]:
    paths: list[str] = []
    for line in lines:
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        if path:
            paths.append(path)
    return sorted(set(paths))


def rust_changes(root: Path) -> list[str]:
    paths = porcelain_paths(git_status_lines(root))
    rust_paths = [p for p in paths if p.endswith(".rs") or Path(p).name in RUST_SENTINELS]
    return sorted(set(rust_paths))


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


def median_int(values: list[int]) -> int:
    if not values:
        return 0
    return int(round(statistics.median(values)))

def parse_perf_metrics(stdout: str) -> dict[str, dict[str, int]]:
    metrics: dict[str, dict[str, int]] = {}
    for line in stdout.splitlines():
        match = PERF_METRIC_PATTERN.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {
            key: int(val)
            for key, val in re.findall(r"(\w+)=([0-9]+)", rest)
        }

        data.setdefault("total_time_ms", 0)
        data.setdefault("rows_processed", 0)
        data.setdefault("cells_compared", 0)

        metrics[test_name] = data

    return metrics


def aggregate_run_payloads(
    root: Path,
    run_json_paths: list[Path],
    output_json: Path,
    output_csv: Path,
    suite_name: str,
    full_scale: bool,
    parallel: bool,
    csv_fields_hint: list[str],
) -> dict:
    run_payloads = [load_json(path) for path in run_json_paths]

    all_tests = sorted(
        {
            test_name
            for payload in run_payloads
            for test_name in payload.get("tests", {}).keys()
        }
    )

    aggregated_tests: dict[str, dict[str, int]] = {}
    for test_name in all_tests:
        metric_names = sorted(
            {
                metric
                for payload in run_payloads
                for metric, value in payload.get("tests", {}).get(test_name, {}).items()
                if isinstance(value, (int, float))
            }
        )
        aggregated_metric: dict[str, int] = {}
        for metric in metric_names:
            values = [
                int(payload.get("tests", {}).get(test_name, {}).get(metric, 0))
                for payload in run_payloads
                if isinstance(payload.get("tests", {}).get(test_name, {}).get(metric), (int, float))
            ]
            aggregated_metric[metric] = median_int(values)

        aggregated_metric.setdefault("total_time_ms", 0)
        aggregated_tests[test_name] = aggregated_metric

    payload = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "git_commit": git_cmd(root, ["rev-parse", "HEAD"])[:12],
        "git_branch": git_cmd(root, ["rev-parse", "--abbrev-ref", "HEAD"]),
        "suite": suite_name,
        "full_scale": full_scale,
        "parallel": parallel,
        "aggregation": {
            "method": "median",
            "runs": len(run_json_paths),
            "source_run_json": [str(path.relative_to(root)) for path in run_json_paths],
        },
        "tests": aggregated_tests,
        "summary": {
            "total_tests": len(aggregated_tests),
            "total_time_ms": sum(m.get("total_time_ms", 0) for m in aggregated_tests.values()),
            "total_rows_processed": sum(
                m.get("rows_processed", 0) for m in aggregated_tests.values()
            ),
            "total_cells_compared": sum(
                m.get("cells_compared", 0) for m in aggregated_tests.values()
            ),
        },
    }
    write_json(output_json, payload)

    all_metric_fields = sorted(
        {
            field
            for data in aggregated_tests.values()
            for field, value in data.items()
            if isinstance(value, (int, float))
        }
    )
    ordered_metric_fields = [
        field for field in csv_fields_hint if field in all_metric_fields
    ] + [field for field in all_metric_fields if field not in csv_fields_hint]

    output_csv.parent.mkdir(parents=True, exist_ok=True)
    with open(output_csv, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=["test_name"] + ordered_metric_fields)
        writer.writeheader()
        for test_name, data in sorted(aggregated_tests.items()):
            row = {"test_name": test_name}
            for field in ordered_metric_fields:
                row[field] = data.get(field, 0)
            writer.writerow(row)

    return payload


def run_fullscale(
    root: Path,
    cycle_dir: Path,
    label: str,
    parallel: bool,
    runs: int,
) -> tuple[Path, list[Path]]:
    output_json = cycle_dir / f"{label}_fullscale.json"
    output_csv = cycle_dir / f"{label}_fullscale.csv"

    run_json_paths: list[Path] = []
    for idx in range(1, runs + 1):
        run_json = cycle_dir / f"{label}_fullscale_run{idx}.json"
        run_csv = cycle_dir / f"{label}_fullscale_run{idx}.csv"

        cmd = [
            sys.executable,
            "scripts/check_perf_thresholds.py",
            "--suite",
            "full-scale",
            "--skip-baseline-check",
            "--export-json",
            str(run_json),
            "--export-csv",
            str(run_csv),
        ]
        if parallel:
            cmd.append("--parallel")
        run(cmd, root)
        run_json_paths.append(run_json)

    aggregate_run_payloads(
        root,
        run_json_paths,
        output_json,
        output_csv,
        suite_name="full-scale",
        full_scale=True,
        parallel=parallel,
        csv_fields_hint=FULLSCALE_CSV_FIELDS,
    )

    return output_json, run_json_paths


def run_e2e(
    root: Path,
    cycle_dir: Path,
    label: str,
    skip_fixtures: bool,
    runs: int,
) -> tuple[Path, list[Path]]:
    output_json = cycle_dir / f"{label}_e2e.json"
    output_csv = cycle_dir / f"{label}_e2e.csv"

    run_json_paths: list[Path] = []
    for idx in range(1, runs + 1):
        run_json = cycle_dir / f"{label}_e2e_run{idx}.json"
        run_csv = cycle_dir / f"{label}_e2e_run{idx}.csv"
        run_output_dir = cycle_dir / f"results_e2e_{label}_run{idx}"
        if run_output_dir.exists():
            # Keep perf-cycle artifacts deterministic (avoid accumulating old timestamped JSONs).
            shutil.rmtree(run_output_dir, ignore_errors=True)

        cmd = [
            sys.executable,
            "scripts/export_e2e_metrics.py",
            "--skip-baseline-check",
            "--latest-json",
            str(run_json),
            "--export-csv",
            str(run_csv),
            "--output-dir",
            str(run_output_dir),
        ]
        # Avoid paying fixture-generation cost repeatedly in median runs.
        if skip_fixtures or idx > 1:
            cmd.append("--skip-fixtures")
        run(cmd, root)
        run_json_paths.append(run_json)

    aggregate_run_payloads(
        root,
        run_json_paths,
        output_json,
        output_csv,
        suite_name="e2e",
        full_scale=False,
        parallel=False,
        csv_fields_hint=E2E_CSV_FIELDS,
    )

    return output_json, run_json_paths


def run_cli_jsonl(
    root: Path,
    cycle_dir: Path,
    label: str,
    runs: int,
) -> tuple[Path, list[Path]]:
    output_json = cycle_dir / f"{label}_cli_jsonl.json"
    output_csv = cycle_dir / f"{label}_cli_jsonl.csv"

    run_json_paths: list[Path] = []
    for idx in range(1, runs + 1):
        run_json = cycle_dir / f"{label}_cli_jsonl_run{idx}.json"
        run_csv = cycle_dir / f"{label}_cli_jsonl_run{idx}.csv"

        cmd = [
            "cargo",
            "test",
            "-p",
            "tabulensis-cli",
            "--release",
            "--features",
            "perf-metrics",
            "--test",
            "perf_cli_jsonl_emit",
            "--",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ]
        result = run_capture(cmd, root, timeout_s=600)
        # Cargo writes build noise to stderr, and test output can land in either stream.
        metrics = parse_perf_metrics(result.stdout + "\n" + result.stderr)
        if not metrics:
            print("ERROR: No PERF_METRIC lines found in cli perf output.")
            raise SystemExit(2)

        payload = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "git_commit": git_cmd(root, ["rev-parse", "HEAD"])[:12],
            "git_branch": git_cmd(root, ["rev-parse", "--abbrev-ref", "HEAD"]),
            "suite": "cli-jsonl",
            "full_scale": False,
            "parallel": False,
            "tests": metrics,
            "summary": {
                "total_tests": len(metrics),
                "total_time_ms": sum(m.get("total_time_ms", 0) for m in metrics.values()),
            },
        }
        write_json(run_json, payload)

        # For quick inspection and to keep parity with the other suites, also write per-run CSV.
        all_metric_fields = sorted(
            {
                field
                for data in metrics.values()
                for field, value in data.items()
                if isinstance(value, (int, float))
            }
        )
        ordered_metric_fields = [
            field for field in CLI_JSONL_CSV_FIELDS if field in all_metric_fields
        ] + [field for field in all_metric_fields if field not in CLI_JSONL_CSV_FIELDS]

        run_csv.parent.mkdir(parents=True, exist_ok=True)
        with open(run_csv, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=["test_name"] + ordered_metric_fields)
            writer.writeheader()
            for test_name, data in sorted(metrics.items()):
                row = {"test_name": test_name}
                for field in ordered_metric_fields:
                    row[field] = data.get(field, 0)
                writer.writerow(row)

        run_json_paths.append(run_json)

    aggregate_run_payloads(
        root,
        run_json_paths,
        output_json,
        output_csv,
        suite_name="cli-jsonl",
        full_scale=False,
        parallel=False,
        csv_fields_hint=CLI_JSONL_CSV_FIELDS,
    )

    return output_json, run_json_paths


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
    meta: dict,
    pre_meta: dict,
    post_meta: dict,
    full_rows: list[list[str]],
    e2e_rows: list[list[str]],
    cli_jsonl_rows: list[list[str]],
) -> str:
    config = meta.get("config", {})
    runs = config.get("runs", 1)
    aggregation = config.get("aggregation", "single")

    lines = [
        f"# Perf Cycle Delta Summary",
        "",
        f"Cycle: `{cycle}`",
        f"Aggregation: `{aggregation}` over **{runs} run(s)**",
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
    lines.extend(
        [
            "## CLI JSONL Emit (total/op_emit time)",
            "| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for row in cli_jsonl_rows:
        lines.append(
            f"| {row[0]} | {row[1]} | {row[2]} | {row[3]} | {row[4]} | {row[5]} | {row[6]} |"
        )
    lines.append("")
    return "\n".join(lines)


def stage_meta(
    root: Path,
    timestamp: str,
    full_json: Path,
    e2e_json: Path,
    cli_jsonl_json: Path,
    full_run_jsons: list[Path],
    e2e_run_jsons: list[Path],
    cli_jsonl_run_jsons: list[Path],
    parallel: bool,
    skip_fixtures: bool,
    runs: int,
    rust_dirty: list[str],
    dirty_paths: list[str],
) -> dict:
    return {
        "timestamp": timestamp,
        "git_commit": git_cmd(root, ["rev-parse", "HEAD"])[:12],
        "git_branch": git_cmd(root, ["rev-parse", "--abbrev-ref", "HEAD"]),
        "fullscale_json": str(full_json.relative_to(root)),
        "e2e_json": str(e2e_json.relative_to(root)),
        "cli_jsonl_json": str(cli_jsonl_json.relative_to(root)),
        "fullscale_run_json": [str(p.relative_to(root)) for p in full_run_jsons],
        "e2e_run_json": [str(p.relative_to(root)) for p in e2e_run_jsons],
        "cli_jsonl_run_json": [str(p.relative_to(root)) for p in cli_jsonl_run_jsons],
        "runs": runs,
        "aggregation": "median",
        "parallel": parallel,
        "skip_fixtures": skip_fixtures,
        "dirty_worktree": bool(dirty_paths),
        "dirty_paths_count": len(dirty_paths),
        "dirty_paths_sample": dirty_paths[:200],
        "dirty_rust_paths": rust_dirty,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run full perf cycle (pre/post) for major perf-risk Rust changes"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--cycle", type=str, default=None, help="Cycle id (timestamp)")
    common.add_argument(
        "--runs",
        type=int,
        default=3,
        help="Number of runs per suite (median-aggregated; default: 3)",
    )
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

    if args.runs < 1:
        print("ERROR: --runs must be >= 1")
        return 2

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
    if args.command == "post" and rust_dirty:
        print(
            "WARNING: Rust changes detected in working tree. Consider committing before running "
            "post so perf-cycle artifacts record an accurate post SHA."
        )

    parallel = not args.no_parallel
    dirty_paths = porcelain_paths(git_status_lines(root))

    if args.command == "pre":
        if meta_path.exists():
            print(f"WARNING: cycle metadata already exists at {meta_path}")

        pre_full, pre_full_runs = run_fullscale(
            root, cycle_dir, "pre", parallel=parallel, runs=args.runs
        )
        pre_e2e, pre_e2e_runs = run_e2e(
            root,
            cycle_dir,
            "pre",
            skip_fixtures=args.skip_fixtures,
            runs=args.runs,
        )
        pre_cli_jsonl, pre_cli_jsonl_runs = run_cli_jsonl(
            root,
            cycle_dir,
            "pre",
            runs=args.runs,
        )

        timestamp = datetime.now(timezone.utc).isoformat()
        meta = {
            "cycle": cycle,
            "config": {
                "runs": args.runs,
                "aggregation": "median",
                "parallel": parallel,
                "skip_fixtures": args.skip_fixtures,
            },
            "pre": stage_meta(
                root,
                timestamp,
                pre_full,
                pre_e2e,
                pre_cli_jsonl,
                pre_full_runs,
                pre_e2e_runs,
                pre_cli_jsonl_runs,
                parallel=parallel,
                skip_fixtures=args.skip_fixtures,
                runs=args.runs,
                rust_dirty=rust_dirty,
                dirty_paths=dirty_paths,
            ),
        }
        write_json(meta_path, meta)
        print(f"Pre-cycle complete. Cycle id: {cycle}")
        return 0

    pre_full_path = cycle_dir / "pre_fullscale.json"
    pre_e2e_path = cycle_dir / "pre_e2e.json"
    pre_cli_jsonl_path = cycle_dir / "pre_cli_jsonl.json"
    if not pre_full_path.exists() or not pre_e2e_path.exists() or not pre_cli_jsonl_path.exists():
        print("ERROR: Missing pre-cycle results. Run: python3 scripts/perf_cycle.py pre")
        return 2

    post_full, post_full_runs = run_fullscale(
        root, cycle_dir, "post", parallel=parallel, runs=args.runs
    )
    post_e2e, post_e2e_runs = run_e2e(
        root,
        cycle_dir,
        "post",
        skip_fixtures=args.skip_fixtures,
        runs=args.runs,
    )
    post_cli_jsonl, post_cli_jsonl_runs = run_cli_jsonl(
        root,
        cycle_dir,
        "post",
        runs=args.runs,
    )

    timestamp = datetime.now(timezone.utc).isoformat()
    meta = load_json(meta_path) if meta_path.exists() else {"cycle": cycle}
    meta.setdefault(
        "config",
        {
            "runs": args.runs,
            "aggregation": "median",
            "parallel": parallel,
            "skip_fixtures": args.skip_fixtures,
        },
    )

    if "pre" not in meta:
        # Recover minimal provenance if pre metadata file was missing or overwritten.
        pre_full_data = load_json(pre_full_path)
        pre_e2e_data = load_json(pre_e2e_path)
        meta["pre"] = {
            "timestamp": pre_e2e_data.get("timestamp") or pre_full_data.get("timestamp"),
            "git_commit": pre_e2e_data.get("git_commit") or pre_full_data.get("git_commit"),
            "git_branch": pre_e2e_data.get("git_branch") or pre_full_data.get("git_branch"),
            "fullscale_json": str(pre_full_path.relative_to(root)),
            "e2e_json": str(pre_e2e_path.relative_to(root)),
            "runs": meta.get("config", {}).get("runs", args.runs),
            "aggregation": meta.get("config", {}).get("aggregation", "median"),
            "reconstructed": True,
        }

    meta["post"] = stage_meta(
        root,
        timestamp,
        post_full,
        post_e2e,
        post_cli_jsonl,
        post_full_runs,
        post_e2e_runs,
        post_cli_jsonl_runs,
        parallel=parallel,
        skip_fixtures=args.skip_fixtures,
        runs=args.runs,
        rust_dirty=rust_dirty,
        dirty_paths=dirty_paths,
    )
    write_json(meta_path, meta)

    pre_full = load_json(pre_full_path)
    post_full_data = load_json(post_full)
    pre_e2e = load_json(pre_e2e_path)
    post_e2e_data = load_json(post_e2e)
    pre_cli_jsonl = load_json(pre_cli_jsonl_path)
    post_cli_jsonl_data = load_json(post_cli_jsonl)

    full_keys = sorted(
        set(pre_full.get("tests", {}).keys())
        | set(post_full_data.get("tests", {}).keys())
    )
    e2e_keys = sorted(
        set(pre_e2e.get("tests", {}).keys())
        | set(post_e2e_data.get("tests", {}).keys())
    )
    cli_jsonl_keys = sorted(
        set(pre_cli_jsonl.get("tests", {}).keys())
        | set(post_cli_jsonl_data.get("tests", {}).keys())
    )

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
    cli_jsonl_rows = delta_table(
        pre_cli_jsonl.get("tests", {}),
        post_cli_jsonl_data.get("tests", {}),
        cli_jsonl_keys,
        ["total_time_ms", "op_emit_time_ms"],
    )

    summary_md = render_markdown_summary(
        cycle,
        meta,
        meta.get("pre", {}),
        meta.get("post", {}),
        full_rows,
        e2e_rows,
        cli_jsonl_rows,
    )
    summary_path = cycle_dir / "cycle_delta.md"
    write_text(summary_path, summary_md)

    summary_json = {
        "cycle": cycle,
        "config": meta.get("config", {}),
        "pre": meta.get("pre", {}),
        "post": meta.get("post", {}),
        "fullscale": full_rows,
        "e2e": e2e_rows,
        "cli_jsonl": cli_jsonl_rows,
    }
    write_json(cycle_dir / "cycle_delta.json", summary_json)

    signal_ok = run_optional(
        [
            sys.executable,
            "scripts/perf_cycle_signal.py",
            "--cycle",
            cycle,
        ],
        root,
    )
    if not signal_ok:
        print("WARNING: signal report generation failed; continuing without cycle_signal output.")

    print(f"Post-cycle complete. Delta summary written to {summary_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
