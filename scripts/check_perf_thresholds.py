#!/usr/bin/env python3
"""
Performance threshold checker for excel_diff.

This script runs perf tests and enforces:
  - Absolute time caps for selected tests
  - Baseline regression checks for total time and peak memory

Usage:
  python scripts/check_perf_thresholds.py [--full-scale] [--export-json PATH] [--export-csv PATH]
"""

import argparse
import csv
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

PERF_TEST_TIMEOUT_SECONDS = 120
FULL_SCALE_TIMEOUT_SECONDS = 600

QUICK_THRESHOLDS = {
    "perf_p1_large_dense": {"max_time_s": 5},
    "perf_p2_large_noise": {"max_time_s": 10},
    "perf_p3_adversarial_repetitive": {"max_time_s": 15},
    "perf_p4_99_percent_blank": {"max_time_s": 2},
    "perf_p5_identical": {"max_time_s": 1},
}

FULL_SCALE_THRESHOLDS = {
    "perf_50k_dense_single_edit": {"max_time_s": 30},
    "perf_50k_completely_different": {"max_time_s": 60},
    "perf_50k_adversarial_repetitive": {"max_time_s": 120},
    "perf_50k_99_percent_blank": {"max_time_s": 30},
    "perf_50k_identical": {"max_time_s": 15},
}

ENV_VAR_MAP = {
    "perf_p1_large_dense": "EXCEL_DIFF_PERF_P1_MAX_TIME_S",
    "perf_p2_large_noise": "EXCEL_DIFF_PERF_P2_MAX_TIME_S",
    "perf_p3_adversarial_repetitive": "EXCEL_DIFF_PERF_P3_MAX_TIME_S",
    "perf_p4_99_percent_blank": "EXCEL_DIFF_PERF_P4_MAX_TIME_S",
    "perf_p5_identical": "EXCEL_DIFF_PERF_P5_MAX_TIME_S",
}

QUICK_PATTERNS = ("perf_p1_", "perf_p2_", "perf_p3_", "perf_p4_", "perf_p5_")
FULL_SCALE_PATTERNS = ("perf_50k_", "perf_100k_", "perf_many_sheets")

BASELINE_SLACK_QUICK = 0.10
BASELINE_SLACK_FULL = 0.15

CSV_FIELDS = [
    "test_name",
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


def get_git_commit():
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


def get_git_branch():
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


def get_effective_thresholds(thresholds, env_var_map=None):
    effective = {}
    slack_factor = float(os.environ.get("EXCEL_DIFF_PERF_SLACK_FACTOR", "1.0"))

    for test_name, config in thresholds.items():
        max_time_s = config["max_time_s"]

        if env_var_map and test_name in env_var_map:
            env_var = env_var_map[test_name]
            if env_var in os.environ:
                try:
                    max_time_s = float(os.environ[env_var])
                    print(
                        f"  Override: {test_name} max_time_s={max_time_s} (from {env_var})"
                    )
                except ValueError:
                    print(f"  WARNING: Invalid value for {env_var}, using default")

        effective[test_name] = {"max_time_s": max_time_s * slack_factor}

    if slack_factor != 1.0:
        print(f"  Slack factor: {slack_factor}x applied to absolute caps")

    return effective


def parse_perf_metrics(stdout: str) -> dict:
    metrics = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {key: int(val) for key, val in re.findall(r"(\w+)=([0-9]+)", rest)}
        data.setdefault("total_time_ms", 0)
        metrics[test_name] = data

    return metrics


def matches_patterns(name: str, patterns: tuple[str, ...]) -> bool:
    return any(name.startswith(prefix) for prefix in patterns)


def collect_passed_tests(stdout: str) -> list[str]:
    tests = []
    pending_test = None
    for line in stdout.split("\n"):
        start_match = re.search(r"test\s+(\S+)\s+\.\.\.", line)
        if start_match:
            if re.search(r"\b(ok|ignored)\b", line):
                tests.append(start_match.group(1))
                pending_test = None
            else:
                pending_test = start_match.group(1)
        elif pending_test and line.strip() in ("ok", "ignored"):
            tests.append(pending_test)
            pending_test = None
    return tests


def export_json(path: Path, metrics: dict, full_scale: bool):
    timestamp = datetime.now(timezone.utc).isoformat()
    payload = {
        "timestamp": timestamp,
        "git_commit": get_git_commit(),
        "git_branch": get_git_branch(),
        "full_scale": full_scale,
        "tests": metrics,
        "summary": {
            "total_tests": len(metrics),
            "total_time_ms": sum(m.get("total_time_ms", 0) for m in metrics.values()),
            "total_rows_processed": sum(m.get("rows_processed", 0) for m in metrics.values()),
            "total_cells_compared": sum(m.get("cells_compared", 0) for m in metrics.values()),
        },
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2)


def export_csv(path: Path, metrics: dict):
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


def parse_baseline_timestamp(value: str, fallback: float) -> float:
    if not value:
        return fallback
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return fallback


def load_baseline(results_dir: Path, full_scale: bool):
    if not results_dir.exists():
        return None, None

    candidates = []
    for json_file in results_dir.glob("*.json"):
        try:
            with open(json_file, "r", encoding="utf-8") as f:
                data = json.load(f)
        except (json.JSONDecodeError, OSError):
            continue

        is_full_scale = data.get("full_scale")
        if is_full_scale is None:
            is_full_scale = "_fullscale" in json_file.name

        if bool(is_full_scale) != full_scale:
            continue

        ts = parse_baseline_timestamp(
            data.get("timestamp", ""), json_file.stat().st_mtime
        )
        candidates.append((ts, json_file, data))

    if not candidates:
        return None, None

    candidates.sort(key=lambda item: item[0], reverse=True)
    _, path, data = candidates[0]
    return data, path


def main():
    parser = argparse.ArgumentParser(
        description="Run perf tests and enforce performance thresholds"
    )
    parser.add_argument(
        "--full-scale",
        action="store_true",
        help="Run the ignored full-scale perf tests",
    )
    parser.add_argument(
        "--export-json",
        type=Path,
        default=None,
        help="Write perf results to JSON (export_perf_metrics schema)",
    )
    parser.add_argument(
        "--export-csv",
        type=Path,
        default=None,
        help="Write perf results to CSV",
    )
    parser.add_argument(
        "--baseline-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Directory containing baseline JSON results",
    )
    args = parser.parse_args()

    suite_name = "full-scale" if args.full_scale else "quick"
    thresholds = FULL_SCALE_THRESHOLDS if args.full_scale else QUICK_THRESHOLDS
    patterns = FULL_SCALE_PATTERNS if args.full_scale else QUICK_PATTERNS
    baseline_slack = BASELINE_SLACK_FULL if args.full_scale else BASELINE_SLACK_QUICK
    env_map = None if args.full_scale else ENV_VAR_MAP

    print("=" * 60)
    print(f"Performance Threshold Check ({suite_name})")
    print("=" * 60)
    print("\nLoading thresholds...")
    effective_thresholds = get_effective_thresholds(thresholds, env_map)
    print()

    core_dir = Path(__file__).parent.parent / "core"
    if not core_dir.exists():
        core_dir = Path("core")

    cmd = [
        "cargo",
        "test",
        "--release",
        "--features",
        "perf-metrics",
        "perf_",
        "--",
        "--nocapture",
        "--test-threads=1",
    ]

    if args.full_scale:
        cmd = [
            "cargo",
            "test",
            "--release",
            "--features",
            "perf-metrics",
            "perf_",
            "--",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ]

    timeout = FULL_SCALE_TIMEOUT_SECONDS if args.full_scale else PERF_TEST_TIMEOUT_SECONDS

    start_time = time.time()
    try:
        result = subprocess.run(
            cmd,
            cwd=core_dir,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        print(f"ERROR: Performance tests exceeded timeout of {timeout}s")
        return 1

    elapsed = time.time() - start_time
    print(f"Total perf suite time: {elapsed:.2f}s")
    print()

    if result.returncode != 0:
        print("ERROR: Performance tests failed!")
        print("STDOUT:", result.stdout)
        print("STDERR:", result.stderr)
        return 1

    passed_tests = collect_passed_tests(result.stdout)
    suite_passed = {t for t in passed_tests if matches_patterns(t, patterns)}

    print(f"Passed suite tests: {len(suite_passed)}")
    for test in sorted(suite_passed):
        print(f"  - {test}")
    print()

    expected_tests = set(effective_thresholds.keys())
    missing_tests = expected_tests - suite_passed
    if missing_tests:
        print(f"ERROR: Some expected perf tests did not run: {missing_tests}")
        return 1

    metrics = parse_perf_metrics(result.stdout)
    suite_metrics = {k: v for k, v in metrics.items() if matches_patterns(k, patterns)}

    if not suite_metrics:
        print("ERROR: No PERF_METRIC output captured for suite tests")
        return 1

    if args.export_json:
        export_json(args.export_json, suite_metrics, args.full_scale)
        print(f"Wrote JSON results to {args.export_json}")

    if args.export_csv:
        export_csv(args.export_csv, suite_metrics)
        print(f"Wrote CSV results to {args.export_csv}")

    missing_metrics = expected_tests - set(suite_metrics.keys())
    if missing_metrics:
        print(f"ERROR: Missing PERF_METRIC output for tests: {missing_metrics}")
        return 1

    failures = []
    print("Absolute threshold checks:")
    for test_name, threshold in effective_thresholds.items():
        max_time_s = threshold["max_time_s"]
        actual_time_ms = suite_metrics[test_name]["total_time_ms"]
        actual_time_s = actual_time_ms / 1000.0

        if actual_time_s > max_time_s:
            status = "FAIL"
            failures.append((test_name, actual_time_s, max_time_s))
        else:
            status = "PASS"

        print(f"  {test_name}: {actual_time_s:.3f}s / {max_time_s:.1f}s [{status}]")

    print()

    baseline_failures = []
    baseline, baseline_path = load_baseline(args.baseline_dir, args.full_scale)
    if baseline and baseline_path:
        print(f"Baseline: {baseline_path}")
        baseline_tests = baseline.get("tests", {})

        for test_name in expected_tests:
            if test_name not in baseline_tests:
                print(f"  WARNING: No baseline for {test_name}; skipping regression check")
                continue

            base = baseline_tests[test_name]
            current = suite_metrics.get(test_name, {})
            base_time = base.get("total_time_ms")
            curr_time = current.get("total_time_ms")
            if base_time is None or curr_time is None:
                print(f"  WARNING: Missing total_time_ms for {test_name}; skipping")
                continue

            time_cap = base_time * (1.0 + baseline_slack)
            if curr_time > time_cap:
                baseline_failures.append(
                    (
                        test_name,
                        "total_time_ms",
                        curr_time,
                        base_time,
                        baseline_slack,
                    )
                )

            base_peak = base.get("peak_memory_bytes")
            curr_peak = current.get("peak_memory_bytes")
            if base_peak is None or curr_peak is None or base_peak <= 0:
                continue

            peak_cap = base_peak * (1.0 + baseline_slack)
            if curr_peak > peak_cap:
                baseline_failures.append(
                    (
                        test_name,
                        "peak_memory_bytes",
                        curr_peak,
                        base_peak,
                        baseline_slack,
                    )
                )

    else:
        print(f"WARNING: No baseline results found in {args.baseline_dir}")

    if failures or baseline_failures:
        print("=" * 60)
        print("PERF FAILURES:")
        for test_name, actual, max_time in failures:
            print(f"  {test_name}: {actual:.3f}s exceeded max of {max_time:.1f}s")
        if baseline_failures:
            print("Baseline regressions:")
            for test_name, metric, curr, base, slack in baseline_failures:
                print(
                    f"  {test_name}: {metric} {curr} > {base} (+{int(slack*100)}%)"
                )
        print("=" * 60)
        return 1

    print("=" * 60)
    print("All performance checks passed")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(main())
