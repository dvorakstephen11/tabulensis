#!/usr/bin/env python3
"""
Export performance metrics from excel_diff tests to JSON.

This script runs the performance test suite and captures the PERF_METRIC output,
saving timestamped results to benchmarks/results/ for historical tracking.

Usage:
    python scripts/export_perf_metrics.py [--full-scale] [--output-dir DIR]

Options:
    --full-scale    Run the 50K row tests (slower but comprehensive)
    --output-dir    Override the output directory (default: benchmarks/results)
"""

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def get_git_commit():
    """Get the current git commit hash."""
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
    """Get the current git branch name."""
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


def parse_perf_metrics(stdout: str) -> dict:
    """Parse PERF_METRIC lines from test output."""
    metrics = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {key: int(val) for key, val in re.findall(r"(\w+)=([0-9]+)", rest)}

        # Ensure required keys exist even if the output is partially missing.
        data.setdefault("total_time_ms", 0)
        data.setdefault("rows_processed", 0)
        data.setdefault("cells_compared", 0)

        metrics[test_name] = data

    return metrics


def run_perf_tests(full_scale: bool = False) -> tuple[dict, bool]:
    """Run performance tests and return parsed metrics."""
    core_dir = Path(__file__).parent.parent / "core"
    if not core_dir.exists():
        core_dir = Path("core")

    cmd = [
        "cargo",
        "test",
        "--release",
        "--features",
        "perf-metrics",
    ]

    if full_scale:
        cmd.extend(["--", "--ignored", "--nocapture", "--test-threads=1"])
    else:
        cmd.extend(["perf_", "--", "--nocapture", "--test-threads=1"])

    print(f"Running: {' '.join(cmd)}")
    print(f"Working directory: {core_dir}")
    print()

    try:
        result = subprocess.run(
            cmd,
            cwd=core_dir,
            capture_output=True,
            text=True,
            timeout=600 if full_scale else 120,
        )
    except subprocess.TimeoutExpired:
        print("ERROR: Tests timed out")
        return {}, False

    print(result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr, file=sys.stderr)

    success = result.returncode == 0
    metrics = parse_perf_metrics(result.stdout)

    return metrics, success


def save_results(metrics: dict, output_dir: Path, full_scale: bool):
    """Save metrics to a timestamped JSON file."""
    timestamp = datetime.now(timezone.utc)
    filename = timestamp.strftime("%Y-%m-%d_%H%M%S")
    if full_scale:
        filename += "_fullscale"
    filename += ".json"

    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / filename

    result = {
        "timestamp": timestamp.isoformat(),
        "git_commit": get_git_commit(),
        "git_branch": get_git_branch(),
        "full_scale": full_scale,
        "tests": metrics,
        "summary": {
            "total_tests": len(metrics),
            "total_time_ms": sum(m["total_time_ms"] for m in metrics.values()),
            "total_rows_processed": sum(m["rows_processed"] for m in metrics.values()),
            "total_cells_compared": sum(m["cells_compared"] for m in metrics.values()),
        },
    }

    with open(output_path, "w") as f:
        json.dump(result, f, indent=2)

    print(f"\nResults saved to: {output_path}")
    return output_path


def print_summary(metrics: dict):
    """Print a summary table of metrics."""
    print("\n" + "=" * 70)
    print("Performance Metrics Summary")
    print("=" * 70)
    print(
        f"{'Test':<40} {'Total':>10} {'Move':>10} {'Align':>10} {'Cell':>10} {'Rows':>10} {'Cells':>12}"
    )
    print("-" * 70)

    for test_name, data in sorted(metrics.items()):
        move = data.get("move_detection_time_ms", 0)
        align = data.get("alignment_time_ms", 0)
        cell = data.get("cell_diff_time_ms", 0)
        print(
            f"{test_name:<40} {data['total_time_ms']:>10,} {move:>10,} {align:>10,} {cell:>10,} {data.get('rows_processed', 0):>10,} {data.get('cells_compared', 0):>12,}"
        )

    print("-" * 70)
    total_time = sum(m["total_time_ms"] for m in metrics.values())
    total_move = sum(m.get("move_detection_time_ms", 0) for m in metrics.values())
    total_align = sum(m.get("alignment_time_ms", 0) for m in metrics.values())
    total_cell = sum(m.get("cell_diff_time_ms", 0) for m in metrics.values())
    total_rows = sum(m["rows_processed"] for m in metrics.values())
    total_cells = sum(m["cells_compared"] for m in metrics.values())
    print(
        f"{'TOTAL':<40} {total_time:>10,} {total_move:>10,} {total_align:>10,} {total_cell:>10,} {total_rows:>10,} {total_cells:>12,}"
    )
    print("=" * 70)


def main():
    parser = argparse.ArgumentParser(
        description="Export performance metrics from excel_diff tests"
    )
    parser.add_argument(
        "--full-scale",
        action="store_true",
        help="Run the 50K row tests (slower but comprehensive)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Output directory for JSON results",
    )
    args = parser.parse_args()

    print("=" * 70)
    print("Excel Diff Performance Metrics Export")
    print("=" * 70)
    print(f"Mode: {'Full-scale (50K rows)' if args.full_scale else 'Quick (1K rows)'}")
    print(f"Output: {args.output_dir}")
    print(f"Git commit: {get_git_commit()}")
    print(f"Git branch: {get_git_branch()}")
    print()

    metrics, success = run_perf_tests(args.full_scale)

    if not metrics:
        print("ERROR: No metrics captured from test output")
        return 1

    print_summary(metrics)
    save_results(metrics, args.output_dir, args.full_scale)

    if not success:
        print("\nWARNING: Some tests may have failed")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())

