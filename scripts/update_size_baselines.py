#!/usr/bin/env python3
"""
Update size baselines from size report JSON files.

Usage:
  python scripts/update_size_baselines.py target/size_reports/cli.json
"""

import argparse
import json
import sys
from pathlib import Path


def load_json(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def main() -> int:
    parser = argparse.ArgumentParser(description="Update size baseline JSONs")
    parser.add_argument(
        "reports",
        nargs="+",
        type=Path,
        help="Size report JSON file(s) to promote to baselines",
    )
    parser.add_argument(
        "--baseline-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "baselines" / "size",
        help="Baseline directory to update",
    )
    args = parser.parse_args()

    args.baseline_dir.mkdir(parents=True, exist_ok=True)

    for report_path in args.reports:
        if not report_path.exists():
            print(f"ERROR: Report file not found: {report_path}", file=sys.stderr)
            return 2
        report = load_json(report_path)
        label = report.get("label")
        if not isinstance(label, str) or not label.strip():
            print(f"ERROR: Report {report_path} missing valid 'label'", file=sys.stderr)
            return 2
        out_path = args.baseline_dir / f"{label}.json"
        out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Updated baseline: {out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
