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
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


BASELINE_SLACK_E2E = 0.20

E2E_THRESHOLDS = {
    "e2e_p1_dense": {
        "max_total_time_s": 25,
        "max_parse_time_s": 20,
        "max_diff_time_s": 10,
    },
    "e2e_p2_noise": {
        "max_total_time_s": 20,
        "max_parse_time_s": 15,
        "max_diff_time_s": 10,
    },
    "e2e_p3_repetitive": {
        "max_total_time_s": 30,
        "max_parse_time_s": 25,
        "max_diff_time_s": 10,
    },
    "e2e_p4_sparse": {
        "max_total_time_s": 5,
        "max_parse_time_s": 3,
        "max_diff_time_s": 2,
    },
    "e2e_p5_identical": {
        "max_total_time_s": 80,
        "max_parse_time_s": 75,
        "max_diff_time_s": 10,
    },
}


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
        data.setdefault("parse_time_ms", 0)
        data.setdefault("diff_time_ms", 0)
        data.setdefault("peak_memory_bytes", 0)
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


def load_baseline_file(path: Path) -> tuple[dict | None, Path | None]:
    if not path.exists():
        return None, None
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (json.JSONDecodeError, OSError):
        return None, None
    return data, path


def load_baseline_dir(results_dir: Path) -> tuple[dict | None, Path | None]:
    if not results_dir.exists():
        return None, None

    candidates = sorted(results_dir.glob("*.json"), key=lambda p: p.stat().st_mtime)
    if not candidates:
        return None, None
    path = candidates[-1]
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (json.JSONDecodeError, OSError):
        return None, None
    return data, path


def get_effective_thresholds(thresholds: dict) -> dict:
    slack_factor = float(os.environ.get("EXCEL_DIFF_E2E_SLACK_FACTOR", "1.0"))
    effective = {}
    for test_name, caps in thresholds.items():
        scaled = {}
        for key, value in caps.items():
            scaled[key] = value * slack_factor
        effective[test_name] = scaled

    if slack_factor != 1.0:
        print(f"  Slack factor: {slack_factor}x applied to absolute caps")

    return effective


def enforce_thresholds(metrics: dict, thresholds: dict) -> list[str]:
    failures = []
    print("Absolute threshold checks:")
    for test_name, caps in thresholds.items():
        data = metrics.get(test_name, {})
        total_time_s = data.get("total_time_ms", 0) / 1000.0
        parse_time_s = data.get("parse_time_ms", 0) / 1000.0
        diff_time_s = data.get("diff_time_ms", 0) / 1000.0

        max_total = caps.get("max_total_time_s")
        max_parse = caps.get("max_parse_time_s")
        max_diff = caps.get("max_diff_time_s")

        if max_total is not None and total_time_s > max_total:
            failures.append(
                f"{test_name}: total_time {total_time_s:.3f}s > {max_total:.1f}s"
            )
        if max_parse is not None and parse_time_s > max_parse:
            failures.append(
                f"{test_name}: parse_time {parse_time_s:.3f}s > {max_parse:.1f}s"
            )
        if max_diff is not None and diff_time_s > max_diff:
            failures.append(
                f"{test_name}: diff_time {diff_time_s:.3f}s > {max_diff:.1f}s"
            )

        print(
            f"  {test_name}: total={total_time_s:.3f}s (cap {max_total:.1f}s), "
            f"parse={parse_time_s:.3f}s (cap {max_parse:.1f}s), "
            f"diff={diff_time_s:.3f}s (cap {max_diff:.1f}s)"
        )
    print()
    return failures


def enforce_baseline(metrics: dict, baseline: dict, expected_tests: set[str]) -> list[str]:
    failures = []
    baseline_tests = baseline.get("tests", {})

    print("Baseline regression checks:")
    for test_name in sorted(expected_tests):
        base = baseline_tests.get(test_name)
        if not base:
            print(f"  WARNING: No baseline for {test_name}; skipping")
            continue

        current = metrics.get(test_name, {})
        for metric_key in ("total_time_ms", "parse_time_ms", "diff_time_ms"):
            base_val = base.get(metric_key)
            curr_val = current.get(metric_key)
            if base_val is None or curr_val is None:
                print(f"  WARNING: Missing {metric_key} for {test_name}; skipping")
                continue

            cap = base_val * (1.0 + BASELINE_SLACK_E2E)
            if curr_val > cap:
                failures.append(
                    f"{test_name}: {metric_key} {curr_val} > {base_val} (+{int(BASELINE_SLACK_E2E*100)}%)"
                )

        base_peak = base.get("peak_memory_bytes")
        curr_peak = current.get("peak_memory_bytes")
        if base_peak and curr_peak:
            cap = base_peak * (1.0 + BASELINE_SLACK_E2E)
            if curr_peak > cap:
                failures.append(
                    f"{test_name}: peak_memory_bytes {curr_peak} > {base_peak} (+{int(BASELINE_SLACK_E2E*100)}%)"
                )
    print()
    return failures


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
    parser.add_argument(
        "--baseline",
        type=Path,
        default=None,
        help="Pinned baseline JSON file (overrides baseline-dir and suite lookup)",
    )
    parser.add_argument(
        "--baseline-dir",
        type=Path,
        default=repo_root / "benchmarks" / "results_e2e",
        help="Directory containing baseline JSON results",
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

    expected_tests = set(E2E_THRESHOLDS.keys())
    missing_tests = expected_tests - set(metrics.keys())
    if missing_tests:
        print(f"ERROR: Missing metrics for expected tests: {missing_tests}")
        return 1

    _, payload = save_results(metrics, args.output_dir)
    write_latest(args.latest_json, payload)
    print(f"Latest JSON written to: {args.latest_json}")

    if args.export_csv:
        export_csv(args.export_csv, metrics)
        print(f"Latest CSV written to: {args.export_csv}")

    print("\nThreshold configuration:")
    effective_thresholds = get_effective_thresholds(E2E_THRESHOLDS)
    for test_name, caps in effective_thresholds.items():
        print(
            f"  {test_name}: total<={caps['max_total_time_s']}s "
            f"parse<={caps['max_parse_time_s']}s diff<={caps['max_diff_time_s']}s"
        )
    print()

    failures = enforce_thresholds(metrics, effective_thresholds)

    baseline = None
    baseline_path = None
    if args.baseline:
        baseline, baseline_path = load_baseline_file(args.baseline)
    else:
        pinned = repo_root / "benchmarks" / "baselines" / "e2e.json"
        if pinned.exists():
            baseline, baseline_path = load_baseline_file(pinned)
        else:
            baseline, baseline_path = load_baseline_dir(args.baseline_dir)

    if baseline and baseline_path:
        print(f"Baseline: {baseline_path}")
        failures.extend(enforce_baseline(metrics, baseline, expected_tests))
    else:
        if args.baseline:
            print(f"WARNING: Baseline file not found: {args.baseline}")
        else:
            print(f"WARNING: No baseline results found in {args.baseline_dir}")

    if failures:
        print("=" * 70)
        print("E2E PERF FAILURES:")
        for failure in failures:
            print(f"  - {failure}")
        print("=" * 70)
        return 1

    if not success:
        print("\nWARNING: Some tests may have failed")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
