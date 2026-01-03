#!/usr/bin/env python3
"""
Compare performance results between two benchmark runs.

Usage:
    python scripts/compare_perf_results.py [baseline.json] [current.json]
    python scripts/compare_perf_results.py --latest  # Compare two most recent results

If no arguments provided, compares the two most recent results in benchmarks/results/.
"""

import argparse
import json
import sys
from pathlib import Path


def load_result(path: Path) -> dict:
    """Load a benchmark result JSON file."""
    with open(path) as f:
        return json.load(f)


def get_latest_results(results_dir: Path, n: int = 2) -> list[Path]:
    """Get the N most recent result files."""
    files = sorted(results_dir.glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)
    return files[:n]


def format_delta(baseline: float, current: float) -> str:
    """Format a percentage delta with color indicator."""
    if baseline == 0:
        return "N/A"
    delta = ((current - baseline) / baseline) * 100
    if abs(delta) < 1:
        return f"  {delta:+.1f}%"
    elif delta < 0:
        return f"  {delta:+.1f}% (faster)"
    else:
        return f"  {delta:+.1f}% (SLOWER)"


def compare_results(baseline: dict, current: dict):
    """Compare two benchmark results and print a comparison table."""
    print("=" * 90)
    print("Performance Comparison")
    print("=" * 90)
    print(f"Baseline: {baseline.get('git_commit', 'unknown')} ({baseline.get('timestamp', 'unknown')[:19]})")
    print(f"Current:  {current.get('git_commit', 'unknown')} ({current.get('timestamp', 'unknown')[:19]})")
    print()

    baseline_tests = baseline.get("tests", {})
    current_tests = current.get("tests", {})

    all_tests = sorted(set(baseline_tests.keys()) | set(current_tests.keys()))

    if not all_tests:
        print("No tests found in either result file.")
        return

    print(f"{'Test':<35} {'Baseline':>10} {'Current':>10} {'Delta':>20}")
    print("-" * 90)

    regressions = []
    improvements = []

    for test_name in all_tests:
        base_data = baseline_tests.get(test_name, {})
        curr_data = current_tests.get(test_name, {})

        base_time = base_data.get("total_time_ms", 0)
        curr_time = curr_data.get("total_time_ms", 0)

        if base_time == 0:
            delta_str = "NEW"
        elif curr_time == 0:
            delta_str = "REMOVED"
        else:
            delta_pct = ((curr_time - base_time) / base_time) * 100
            delta_str = format_delta(base_time, curr_time)

            if delta_pct > 10:
                regressions.append((test_name, delta_pct))
            elif delta_pct < -10:
                improvements.append((test_name, delta_pct))

        base_str = f"{base_time:,}ms" if base_time else "—"
        curr_str = f"{curr_time:,}ms" if curr_time else "—"

        print(f"{test_name:<35} {base_str:>10} {curr_str:>10} {delta_str:>20}")

    print("-" * 90)

    base_total = baseline.get("summary", {}).get("total_time_ms", 0)
    curr_total = current.get("summary", {}).get("total_time_ms", 0)
    print(f"{'TOTAL':<35} {base_total:>10,}ms {curr_total:>10,}ms {format_delta(base_total, curr_total):>20}")
    print("=" * 90)

    if regressions:
        print("\n⚠️  REGRESSIONS (>10% slower):")
        for name, delta in sorted(regressions, key=lambda x: -x[1]):
            print(f"   {name}: +{delta:.1f}%")

    if improvements:
        print("\n✅ IMPROVEMENTS (>10% faster):")
        for name, delta in sorted(improvements, key=lambda x: x[1]):
            print(f"   {name}: {delta:.1f}%")

    if not regressions and not improvements:
        print("\n✓ No significant changes detected (within ±10%)")


def main():
    parser = argparse.ArgumentParser(description="Compare performance benchmark results")
    parser.add_argument("baseline", nargs="?", type=Path, help="Baseline result JSON file")
    parser.add_argument("current", nargs="?", type=Path, help="Current result JSON file")
    parser.add_argument("--latest", action="store_true", help="Compare two most recent results")
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Results directory",
    )
    args = parser.parse_args()

    if args.latest or (args.baseline is None and args.current is None):
        files = get_latest_results(args.results_dir, 2)
        if len(files) < 2:
            print(f"ERROR: Need at least 2 result files in {args.results_dir}")
            print(f"Found: {len(files)} files")
            return 1
        baseline_path = files[1]
        current_path = files[0]
    else:
        if not args.baseline or not args.current:
            parser.error("Must provide both baseline and current files, or use --latest")
        baseline_path = args.baseline
        current_path = args.current

    if not baseline_path.exists():
        print(f"ERROR: Baseline file not found: {baseline_path}")
        return 1
    if not current_path.exists():
        print(f"ERROR: Current file not found: {current_path}")
        return 1

    baseline = load_result(baseline_path)
    current = load_result(current_path)

    compare_results(baseline, current)
    return 0


if __name__ == "__main__":
    sys.exit(main())

