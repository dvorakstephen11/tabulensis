#!/usr/bin/env python3
"""
Export end-to-end workbook open + diff metrics to versioned JSON.

This script generates the e2e fixtures, runs the ignored e2e perf tests, captures
PERF_METRIC output, and writes timestamped results plus a latest JSON (and CSV).

Usage:
    python scripts/export_e2e_metrics.py [--output-dir DIR] [--export-csv PATH] [--skip-fixtures]
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


CSV_FIELDS = [
    "test_name",
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


def get_git_commit() -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()[:12]
    except Exception:
        pass
    return "unknown"


def get_git_branch() -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass
    return "unknown"


def run_command(
    cmd: list[str], cwd: Path, *, capture_output: bool = False, timeout: int | None = None
) -> subprocess.CompletedProcess[str]:
    print(f"Running: {' '.join(cmd)}")
    return subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=capture_output,
        text=True,
        timeout=timeout,
    )


def install_fixture_generator(repo_root: Path) -> None:
    python = sys.executable
    requirements = repo_root / "fixtures" / "requirements.txt"
    result = run_command(
        [python, "-m", "pip", "install", "-r", str(requirements)],
        cwd=repo_root,
    )
    if result.returncode != 0:
        raise RuntimeError("Failed to install fixture generator requirements.")

    result = run_command(
        [python, "-m", "pip", "install", "-e", "fixtures", "--no-deps"],
        cwd=repo_root,
    )
    if result.returncode != 0:
        raise RuntimeError("Failed to install fixture generator package.")


def generate_fixtures(repo_root: Path, manifest: Path) -> None:
    result = run_command(
        ["generate-fixtures", "--manifest", str(manifest), "--force"],
        cwd=repo_root,
    )
    if result.returncode != 0:
        raise RuntimeError("Failed to generate e2e fixtures.")


def parse_perf_metrics(stdout: str) -> dict:
    metrics: dict[str, dict[str, int]] = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {key: int(val) for key, val in re.findall(r"(\w+)=([0-9]+)", rest)}

        data.setdefault("total_time_ms", 0)
        data.setdefault("rows_processed", 0)
        data.setdefault("cells_compared", 0)

        metrics[test_name] = data

    return metrics


def run_e2e_tests(core_dir: Path) -> tuple[dict, bool]:
    cmd = [
        "cargo",
        "test",
        "--release",
        "--features",
        "perf-metrics",
        "--test",
        "e2e_perf_workbook_open",
        "e2e_",
        "--",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]

    try:
        result = run_command(cmd, cwd=core_dir, capture_output=True, timeout=1800)
    except subprocess.TimeoutExpired:
        print("ERROR: Tests timed out")
        return {}, False
    print(result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr, file=sys.stderr)

    metrics = parse_perf_metrics(result.stdout)
    success = result.returncode == 0
    return metrics, success


def save_results(metrics: dict, output_dir: Path) -> tuple[Path, dict]:
    timestamp = datetime.now(timezone.utc)
    filename = timestamp.strftime("%Y-%m-%d_%H%M%S") + ".json"

    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / filename

    result = {
        "timestamp": timestamp.isoformat(),
        "git_commit": get_git_commit(),
        "git_branch": get_git_branch(),
        "tests": metrics,
        "summary": {
            "total_tests": len(metrics),
            "total_time_ms": sum(m.get("total_time_ms", 0) for m in metrics.values()),
            "total_rows_processed": sum(
                m.get("rows_processed", 0) for m in metrics.values()
            ),
            "total_cells_compared": sum(
                m.get("cells_compared", 0) for m in metrics.values()
            ),
        },
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)

    print(f"\nResults saved to: {output_path}")
    return output_path, result


def write_latest(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2)


def export_csv(path: Path, metrics: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_FIELDS)
        writer.writeheader()
        for test_name, data in sorted(metrics.items()):
            row = {"test_name": test_name}
            for field in CSV_FIELDS:
                if field == "test_name":
                    continue
                row[field] = data.get(field, 0)
            writer.writerow(row)


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    core_dir = repo_root / "core"
    manifest = repo_root / "fixtures" / "manifest_perf_e2e.yaml"

    parser = argparse.ArgumentParser(
        description="Export end-to-end workbook open perf metrics"
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=repo_root / "benchmarks" / "results_e2e",
        help="Output directory for timestamped JSON results",
    )
    parser.add_argument(
        "--latest-json",
        type=Path,
        default=repo_root / "benchmarks" / "latest_e2e.json",
        help="Path to write latest JSON results",
    )
    parser.add_argument(
        "--export-csv",
        type=Path,
        default=None,
        help="Optional path to write latest CSV results",
    )
    parser.add_argument(
        "--skip-fixtures",
        action="store_true",
        help="Skip fixture generator installation and generation",
    )
    args = parser.parse_args()

    print("=" * 70)
    print("Excel Diff E2E Metrics Export")
    print("=" * 70)
    print(f"Output: {args.output_dir}")
    print(f"Latest JSON: {args.latest_json}")
    if args.export_csv:
        print(f"Latest CSV: {args.export_csv}")
    print(f"Git commit: {get_git_commit()}")
    print(f"Git branch: {get_git_branch()}")
    print()

    try:
        if not args.skip_fixtures:
            install_fixture_generator(repo_root)
            generate_fixtures(repo_root, manifest)
    except RuntimeError as exc:
        print(f"ERROR: {exc}")
        return 1

    metrics, success = run_e2e_tests(core_dir)
    if not metrics:
        print("ERROR: No metrics captured from test output")
        return 1

    _, payload = save_results(metrics, args.output_dir)
    write_latest(args.latest_json, payload)
    print(f"Latest JSON written to: {args.latest_json}")

    if args.export_csv:
        export_csv(args.export_csv, metrics)
        print(f"Latest CSV written to: {args.export_csv}")

    if not success:
        print("\nWARNING: Some tests may have failed")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
