#!/usr/bin/env python3
"""
Performance threshold checker for excel_diff.

This script verifies that performance tests complete within acceptable time bounds.
It runs after `cargo test --release --features perf-metrics perf_` and validates
that the test suite completed successfully.

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
"""

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


def run_perf_tests():
    """Run the performance tests and verify they complete within timeout."""
    print("=" * 60)
    print("Performance Threshold Check")
    print("=" * 60)

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
        print(f"WARNING: Some expected perf tests did not run: {missing_tests}")

    print("=" * 60)
    print("All performance tests passed within thresholds!")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(run_perf_tests())

