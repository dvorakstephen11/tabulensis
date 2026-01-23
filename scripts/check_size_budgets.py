#!/usr/bin/env python3
"""
Check size reports against baselines and hard caps.

Usage:
  python scripts/check_size_budgets.py target/size_reports/cli.json
  python scripts/check_size_budgets.py --baseline-dir benchmarks/baselines/size \
    --budgets benchmarks/size_budgets.json target/size_reports/cli.json
"""

import argparse
import json
import sys
from pathlib import Path

DEFAULT_SLACK_RATIO = 0.02
DEFAULT_SLACK_BYTES = 0


def load_json(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def load_budgets(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        return load_json(path)
    except (json.JSONDecodeError, OSError) as exc:
        print(f"ERROR: Failed to read budgets file {path}: {exc}", file=sys.stderr)
        return {}


def normalize_budget(entry) -> dict:
    if entry is None:
        return {}
    if isinstance(entry, (int, float)):
        return {"hard_cap_bytes": int(entry)}
    if isinstance(entry, dict):
        return entry
    return {}


def get_budget(budgets: dict, label: str, metric: str) -> dict:
    label_cfg = budgets.get(label)
    if not isinstance(label_cfg, dict):
        return {}
    return normalize_budget(label_cfg.get(metric))


def format_bytes(value: int) -> str:
    return f"{value:,}"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check size reports against baseline and hard caps"
    )
    parser.add_argument(
        "reports",
        nargs="+",
        type=Path,
        help="Size report JSON file(s) from scripts/size_report.py",
    )
    parser.add_argument(
        "--baseline-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "baselines" / "size",
        help="Directory containing baseline size JSON files",
    )
    parser.add_argument(
        "--budgets",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "size_budgets.json",
        help="JSON file describing hard caps and slack for each label",
    )
    parser.add_argument(
        "--require-baseline",
        action="store_true",
        help="Fail if a baseline is missing for a report",
    )
    parser.add_argument(
        "--slack-ratio",
        type=float,
        default=DEFAULT_SLACK_RATIO,
        help="Default allowed growth ratio vs baseline when not specified in budgets",
    )
    parser.add_argument(
        "--slack-bytes",
        type=int,
        default=DEFAULT_SLACK_BYTES,
        help="Default allowed growth in bytes vs baseline when not specified in budgets",
    )
    args = parser.parse_args()

    budgets = load_budgets(args.budgets)
    if not budgets:
        if args.budgets.exists():
            print(f"WARNING: No budgets loaded from {args.budgets}")
        else:
            print(f"WARNING: Budgets file not found at {args.budgets}")

    failures = []
    had_baseline = False

    for report_path in args.reports:
        if not report_path.exists():
            print(f"ERROR: Report file not found: {report_path}", file=sys.stderr)
            return 2

        report = load_json(report_path)
        label = report.get("label")
        if not isinstance(label, str) or not label.strip():
            print(f"ERROR: Report {report_path} missing valid 'label'", file=sys.stderr)
            return 2

        baseline_path = args.baseline_dir / f"{label}.json"
        baseline = None
        if baseline_path.exists():
            baseline = load_json(baseline_path)
            had_baseline = True
        elif args.require_baseline:
            print(f"ERROR: Missing baseline {baseline_path}", file=sys.stderr)
            return 2
        else:
            print(f"WARNING: Missing baseline {baseline_path}")

        print("=" * 60)
        print(f"Size budget check: {label}")
        print("=" * 60)

        for metric in ("raw_bytes", "zip_bytes"):
            if metric not in report:
                continue
            current_val = report[metric]
            if not isinstance(current_val, int):
                print(
                    f"ERROR: {report_path} has non-integer {metric}",
                    file=sys.stderr,
                )
                return 2

            budget = get_budget(budgets, label, metric)
            hard_cap = budget.get("hard_cap_bytes")
            slack_ratio = budget.get("slack_ratio", args.slack_ratio)
            slack_bytes = budget.get("slack_bytes", args.slack_bytes)

            if hard_cap is not None:
                if current_val > hard_cap:
                    failures.append(
                        (
                            label,
                            metric,
                            current_val,
                            hard_cap,
                            "hard_cap",
                        )
                    )
                status = "PASS" if current_val <= hard_cap else "FAIL"
                print(
                    f"  {metric}: {format_bytes(current_val)} <= {format_bytes(hard_cap)} [{status}]"
                )
            else:
                print(f"  {metric}: {format_bytes(current_val)} (no hard cap)")

            if baseline:
                base_val = baseline.get(metric)
                if isinstance(base_val, int):
                    allowed = int(base_val * (1.0 + float(slack_ratio)) + slack_bytes)
                    if current_val > allowed:
                        failures.append(
                            (
                                label,
                                metric,
                                current_val,
                                allowed,
                                "baseline",
                            )
                        )
                        status = "FAIL"
                    else:
                        status = "PASS"
                    print(
                        f"    baseline: {format_bytes(base_val)} -> allowed {format_bytes(allowed)} [{status}]"
                    )
                else:
                    msg = f"    baseline missing {metric}; skipping regression check"
                    if args.require_baseline:
                        print(f"ERROR: {msg}", file=sys.stderr)
                        return 2
                    print(f"WARNING: {msg}")

        print()

    if failures:
        print("=" * 60)
        print("SIZE FAILURES")
        for label, metric, current_val, cap, kind in failures:
            print(
                f"  {label} {metric}: {format_bytes(current_val)} exceeded {format_bytes(cap)} ({kind})"
            )
        print("=" * 60)
        return 1

    if args.require_baseline and not had_baseline:
        print("ERROR: No baselines found and --require-baseline is set", file=sys.stderr)
        return 2

    print("=" * 60)
    print("All size checks passed")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
