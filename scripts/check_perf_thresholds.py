#!/usr/bin/env python3
"""
Performance threshold checker for excel_diff.

This script verifies that performance tests complete within acceptable time bounds.
It runs `cargo test --release --features perf-metrics perf_` and validates that
each test completes within its configured threshold.

Thresholds are based on the mini-spec table from next_sprint_plan.md:
| Fixture | Rows | Cols | Max Time | Max Memory |
|---------|------|------|----------|------------|
| p1_large_dense | 50,000 | 100 | 5s | 500MB |
| p2_large_noise | 50,000 | 100 | 10s | 600MB |
| p3_adversarial_repetitive | 50,000 | 50 | 15s | 400MB |
| p4_99_percent_blank | 50,000 | 100 | 2s | 200MB |
| p5_identical | 50,000 | 100 | 1s | 300MB |

Note: The Rust tests use smaller grids for CI speed, so these thresholds are
conservative. Memory tracking is planned for a future phase.

Environment variables for threshold configuration:
  EXCEL_DIFF_PERF_P1_MAX_TIME_S - Override max time for perf_p1_large_dense
  EXCEL_DIFF_PERF_P2_MAX_TIME_S - Override max time for perf_p2_large_noise
  EXCEL_DIFF_PERF_P3_MAX_TIME_S - Override max time for perf_p3_adversarial_repetitive
  EXCEL_DIFF_PERF_P4_MAX_TIME_S - Override max time for perf_p4_99_percent_blank
  EXCEL_DIFF_PERF_P5_MAX_TIME_S - Override max time for perf_p5_identical
  EXCEL_DIFF_PERF_SLACK_FACTOR - Multiply all thresholds by this factor (default: 1.0)
"""

import os
import re
import subprocess
import sys
import time
from pathlib import Path

PERF_TEST_TIMEOUT_SECONDS = 120

THRESHOLDS = {
    "perf_p1_large_dense": {"max_time_s": 30},
    "perf_p2_large_noise": {"max_time_s": 30},
    "perf_p3_adversarial_repetitive": {"max_time_s": 60},
    "perf_p4_99_percent_blank": {"max_time_s": 15},
    "perf_p5_identical": {"max_time_s": 10},
}

ENV_VAR_MAP = {
    "perf_p1_large_dense": "EXCEL_DIFF_PERF_P1_MAX_TIME_S",
    "perf_p2_large_noise": "EXCEL_DIFF_PERF_P2_MAX_TIME_S",
    "perf_p3_adversarial_repetitive": "EXCEL_DIFF_PERF_P3_MAX_TIME_S",
    "perf_p4_99_percent_blank": "EXCEL_DIFF_PERF_P4_MAX_TIME_S",
    "perf_p5_identical": "EXCEL_DIFF_PERF_P5_MAX_TIME_S",
}


def get_effective_thresholds():
    """Get thresholds with environment variable overrides applied."""
    effective = {}
    slack_factor = float(os.environ.get("EXCEL_DIFF_PERF_SLACK_FACTOR", "1.0"))

    for test_name, config in THRESHOLDS.items():
        max_time_s = config["max_time_s"]

        env_var = ENV_VAR_MAP.get(test_name)
        if env_var and env_var in os.environ:
            try:
                max_time_s = float(os.environ[env_var])
                print(f"  Override: {test_name} max_time_s={max_time_s} (from {env_var})")
            except ValueError:
                print(f"  WARNING: Invalid value for {env_var}, using default")

        effective[test_name] = {"max_time_s": max_time_s * slack_factor}

    if slack_factor != 1.0:
        print(f"  Slack factor: {slack_factor}x applied to all thresholds")

    return effective


def parse_perf_metrics(stdout: str) -> dict:
    """Parse PERF_METRIC lines from test output.

    Expected format: PERF_METRIC <test_name> total_time_ms=<value> [other_metrics...]

    Returns dict mapping test_name -> {"total_time_ms": int, ...}
    """
    metrics = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+total_time_ms=(\d+)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if match:
            test_name = match.group(1)
            total_time_ms = int(match.group(2))
            metrics[test_name] = {"total_time_ms": total_time_ms}

    return metrics


def run_perf_tests():
    """Run the performance tests and verify they complete within thresholds."""
    print("=" * 60)
    print("Performance Threshold Check")
    print("=" * 60)

    print("\nLoading thresholds...")
    effective_thresholds = get_effective_thresholds()
    print()

    core_dir = Path(__file__).parent.parent / "core"
    if not core_dir.exists():
        core_dir = Path("core")

    start_time = time.time()
    try:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "--release",
                "--features",
                "perf-metrics",
                "perf_",
                "--",
                "--nocapture",
            ],
            cwd=core_dir,
            capture_output=True,
            text=True,
            timeout=PERF_TEST_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(f"ERROR: Performance tests exceeded timeout of {PERF_TEST_TIMEOUT_SECONDS}s")
        return 1

    elapsed = time.time() - start_time
    print(f"Total perf suite time: {elapsed:.2f}s")
    print()

    if result.returncode != 0:
        print("ERROR: Performance tests failed!")
        print("STDOUT:", result.stdout)
        print("STDERR:", result.stderr)
        return 1

    passed_tests = []
    for line in result.stdout.split("\n"):
        if "test perf_" in line and "... ok" in line:
            test_name = line.split("test ")[1].split(" ...")[0].strip()
            passed_tests.append(test_name)

    print(f"Passed tests: {len(passed_tests)}")
    for test in passed_tests:
        print(f"  ✓ {test}")
    print()

    expected_tests = set(THRESHOLDS.keys())
    actual_tests = set(passed_tests)
    missing_tests = expected_tests - actual_tests

    if missing_tests:
        print(f"ERROR: Some expected perf tests did not run: {missing_tests}")
        return 1

    metrics = parse_perf_metrics(result.stdout)
    print(f"Parsed metrics for {len(metrics)} tests:")
    for test_name, data in metrics.items():
        total_time_s = data["total_time_ms"] / 1000.0
        print(f"  {test_name}: {total_time_s:.3f}s")
    print()

    missing_metrics = expected_tests - set(metrics.keys())
    if missing_metrics:
        print(f"ERROR: Missing PERF_METRIC output for tests: {missing_metrics}")
        return 1

    failures = []
    print("Threshold checks:")
    for test_name, threshold in effective_thresholds.items():
        max_time_s = threshold["max_time_s"]
        actual_time_ms = metrics[test_name]["total_time_ms"]
        actual_time_s = actual_time_ms / 1000.0

        if actual_time_s > max_time_s:
            status = "FAIL"
            failures.append((test_name, actual_time_s, max_time_s))
        else:
            status = "PASS"

        print(f"  {test_name}: {actual_time_s:.3f}s / {max_time_s:.1f}s [{status}]")

    print()

    if failures:
        print("=" * 60)
        print("THRESHOLD VIOLATIONS:")
        for test_name, actual, max_time in failures:
            print(f"  {test_name}: {actual:.3f}s exceeded max of {max_time:.1f}s")
        print("=" * 60)
        return 1

    print("=" * 60)
    print("All performance tests passed within thresholds!")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(run_perf_tests())
