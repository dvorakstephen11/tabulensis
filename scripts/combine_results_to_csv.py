#!/usr/bin/env python3
"""
Combine benchmark JSON results into a single CSV for comparison over time.

Usage:
    python scripts/combine_results_to_csv.py [--output FILE] [--results-dir DIR]

Options:
    --output      Output CSV file path (default: benchmarks/results/combined_results.csv)
    --results-dir Directory containing JSON results (default: benchmarks/results)
"""

import argparse
import csv
import json
import sys
from pathlib import Path


ALL_TEST_FIELDS = [
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


def load_json_results(results_dir: Path) -> list[dict]:
    """Load all JSON result files from the results directory."""
    results = []
    for json_file in sorted(results_dir.glob("*.json")):
        try:
            with open(json_file) as f:
                data = json.load(f)
                data["_source_file"] = json_file.name
                results.append(data)
        except (json.JSONDecodeError, IOError) as e:
            print(f"Warning: Could not load {json_file}: {e}", file=sys.stderr)
    return results


def flatten_results(results: list[dict]) -> list[dict]:
    """Flatten nested test results into individual rows."""
    rows = []
    for result in results:
        timestamp = result.get("timestamp", "")
        git_commit = result.get("git_commit", "")
        git_branch = result.get("git_branch", "")
        full_scale = result.get("full_scale", False)
        source_file = result.get("_source_file", "")

        tests = result.get("tests", {})
        for test_name, test_data in tests.items():
            row = {
                "source_file": source_file,
                "timestamp": timestamp,
                "git_commit": git_commit,
                "git_branch": git_branch,
                "full_scale": full_scale,
                "test_name": test_name,
            }
            for field in ALL_TEST_FIELDS:
                row[field] = test_data.get(field, "")
            rows.append(row)

        summary = result.get("summary", {})
        if summary:
            row = {
                "source_file": source_file,
                "timestamp": timestamp,
                "git_commit": git_commit,
                "git_branch": git_branch,
                "full_scale": full_scale,
                "test_name": "_SUMMARY_",
                "total_time_ms": summary.get("total_time_ms", ""),
                "rows_processed": summary.get("total_rows_processed", ""),
                "cells_compared": summary.get("total_cells_compared", ""),
            }
            for field in ALL_TEST_FIELDS:
                if field not in row:
                    row[field] = ""
            rows.append(row)

    return rows


def write_csv(rows: list[dict], output_path: Path):
    """Write flattened results to CSV."""
    if not rows:
        print("No data to write.", file=sys.stderr)
        return

    fieldnames = [
        "source_file",
        "timestamp",
        "git_commit",
        "git_branch",
        "full_scale",
        "test_name",
    ] + ALL_TEST_FIELDS

    with open(output_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(
        description="Combine benchmark JSON results into a single CSV"
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Directory containing JSON results",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Output CSV file path",
    )
    args = parser.parse_args()

    if args.output is None:
        args.output = args.results_dir / "combined_results.csv"

    if not args.results_dir.exists():
        print(f"ERROR: Results directory not found: {args.results_dir}", file=sys.stderr)
        return 1

    print(f"Loading results from: {args.results_dir}")
    results = load_json_results(args.results_dir)

    if not results:
        print("ERROR: No JSON result files found.", file=sys.stderr)
        return 1

    print(f"Found {len(results)} result files")

    rows = flatten_results(results)
    print(f"Generated {len(rows)} rows")

    write_csv(rows, args.output)
    print(f"CSV written to: {args.output}")

    return 0


if __name__ == "__main__":
    sys.exit(main())

