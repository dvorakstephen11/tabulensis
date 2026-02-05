#!/usr/bin/env python3
"""
Aggregate and summarize historical perf metrics across benchmark result files
and perf-cycle artifacts.

This script builds multi-metric trend data (CSV + markdown summary) from:
  - benchmarks/results/*.json
  - benchmarks/perf_cycles/<cycle_id>/{pre_*.json,post_*.json}

Optional plots are generated when matplotlib is available.

Usage:
  python3 scripts/perf_history_trends.py
  python3 scripts/perf_history_trends.py --output-dir benchmarks/history --plots
  python3 scripts/perf_history_trends.py --output-dir benchmarks/history --tracked-only
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

METRIC_FIELDS = [
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

POINT_FIELDS = [
    "source_kind",
    "source_path",
    "cycle_id",
    "phase",
    "suite",
    "run_id",
    "timestamp",
    "git_commit",
    "git_branch",
    "test_name",
] + METRIC_FIELDS

RUN_FIELDS = [
    "source_kind",
    "source_path",
    "cycle_id",
    "phase",
    "suite",
    "run_id",
    "timestamp",
    "git_commit",
    "git_branch",
    "test_count",
] + METRIC_FIELDS


def parse_ts(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def format_pct(delta: float, base: float) -> str:
    if base == 0:
        return "n/a"
    return f"{(delta / base) * 100:+.1f}%"


def safe_int(value: Any) -> int:
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    return 0


def parse_metric_value(metrics: dict[str, Any], metric: str) -> int | None:
    if metric not in metrics:
        return None
    return safe_int(metrics.get(metric))


def load_json(path: Path) -> dict[str, Any] | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def infer_suite(payload: dict[str, Any], path: Path) -> str:
    suite = payload.get("suite")
    if isinstance(suite, str) and suite:
        return suite
    name = path.name.lower()
    if "e2e" in name:
        return "e2e"
    if payload.get("full_scale") is True or "fullscale" in name or "full-scale" in name:
        return "full-scale"
    if "gate" in name:
        return "gate"
    return "quick"


@dataclass(frozen=True)
class RunKey:
    source_kind: str
    source_path: str
    cycle_id: str
    phase: str
    suite: str
    run_id: str
    timestamp: str
    git_commit: str
    git_branch: str


def points_from_result_json(path: Path, root: Path) -> list[dict[str, Any]]:
    payload = load_json(path)
    if not payload:
        return []
    tests = payload.get("tests")
    if not isinstance(tests, dict):
        return []

    ts = payload.get("timestamp")
    if not isinstance(ts, str) or not ts:
        return []

    suite = infer_suite(payload, path)
    rel_path = str(path.relative_to(root))
    run_id = path.stem
    git_commit = str(payload.get("git_commit", "unknown"))
    git_branch = str(payload.get("git_branch", "unknown"))

    out: list[dict[str, Any]] = []
    for test_name, metrics in tests.items():
        if not isinstance(metrics, dict):
            continue
        row: dict[str, Any] = {
            "source_kind": "results",
            "source_path": rel_path,
            "cycle_id": "",
            "phase": "single",
            "suite": suite,
            "run_id": run_id,
            "timestamp": ts,
            "git_commit": git_commit,
            "git_branch": git_branch,
            "test_name": str(test_name),
        }
        for metric in METRIC_FIELDS:
            row[metric] = parse_metric_value(metrics, metric)
        out.append(row)
    return out


def points_from_perf_cycle_json(path: Path, root: Path, cycle_id: str) -> list[dict[str, Any]]:
    payload = load_json(path)
    if not payload:
        return []
    tests = payload.get("tests")
    if not isinstance(tests, dict):
        return []

    ts = payload.get("timestamp")
    if not isinstance(ts, str) or not ts:
        return []

    suite = infer_suite(payload, path)
    phase = "pre" if path.name.startswith("pre_") else "post" if path.name.startswith("post_") else "single"
    rel_path = str(path.relative_to(root))
    run_id = f"{cycle_id}:{phase}:{suite}"
    git_commit = str(payload.get("git_commit", "unknown"))
    git_branch = str(payload.get("git_branch", "unknown"))

    out: list[dict[str, Any]] = []
    for test_name, metrics in tests.items():
        if not isinstance(metrics, dict):
            continue
        row: dict[str, Any] = {
            "source_kind": "perf_cycle",
            "source_path": rel_path,
            "cycle_id": cycle_id,
            "phase": phase,
            "suite": suite,
            "run_id": run_id,
            "timestamp": ts,
            "git_commit": git_commit,
            "git_branch": git_branch,
            "test_name": str(test_name),
        }
        for metric in METRIC_FIELDS:
            row[metric] = parse_metric_value(metrics, metric)
        out.append(row)
    return out


def tracked_paths(root: Path) -> set[str]:
    result = subprocess.run(
        ["git", "ls-files", "benchmarks/results", "benchmarks/perf_cycles"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return set()
    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


def collect_points(
    results_dir: Path,
    perf_cycles_dir: Path,
    root: Path,
    tracked_only: bool = False,
) -> list[dict[str, Any]]:
    points: list[dict[str, Any]] = []
    tracked = tracked_paths(root) if tracked_only else None

    if results_dir.exists():
        for path in sorted(results_dir.glob("*.json")):
            rel = str(path.relative_to(root))
            if tracked is not None and rel not in tracked:
                continue
            points.extend(points_from_result_json(path, root))

    if perf_cycles_dir.exists():
        for cycle_dir in sorted(p for p in perf_cycles_dir.iterdir() if p.is_dir()):
            for path in sorted(cycle_dir.glob("pre_*.json")):
                rel = str(path.relative_to(root))
                if tracked is not None and rel not in tracked:
                    continue
                points.extend(points_from_perf_cycle_json(path, root, cycle_dir.name))
            for path in sorted(cycle_dir.glob("post_*.json")):
                rel = str(path.relative_to(root))
                if tracked is not None and rel not in tracked:
                    continue
                points.extend(points_from_perf_cycle_json(path, root, cycle_dir.name))

    points.sort(key=lambda r: (r["timestamp"], r["run_id"], r["suite"], r["test_name"]))
    return points


def build_run_aggregates(points: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_run: dict[RunKey, dict[str, Any]] = {}

    for row in points:
        key = RunKey(
            source_kind=row["source_kind"],
            source_path=row["source_path"],
            cycle_id=row["cycle_id"],
            phase=row["phase"],
            suite=row["suite"],
            run_id=row["run_id"],
            timestamp=row["timestamp"],
            git_commit=row["git_commit"],
            git_branch=row["git_branch"],
        )
        agg = by_run.get(key)
        if agg is None:
            agg = {
                "source_kind": key.source_kind,
                "source_path": key.source_path,
                "cycle_id": key.cycle_id,
                "phase": key.phase,
                "suite": key.suite,
                "run_id": key.run_id,
                "timestamp": key.timestamp,
                "git_commit": key.git_commit,
                "git_branch": key.git_branch,
                "test_count": 0,
            }
            agg["_metric_present_counts"] = {}
            for metric in METRIC_FIELDS:
                agg[metric] = 0
                agg["_metric_present_counts"][metric] = 0
            by_run[key] = agg

        agg["test_count"] += 1
        for metric in METRIC_FIELDS:
            value = row.get(metric)
            if value is None:
                continue
            agg[metric] += safe_int(value)
            agg["_metric_present_counts"][metric] += 1

    runs = list(by_run.values())
    for row in runs:
        present = row.pop("_metric_present_counts")
        for metric in METRIC_FIELDS:
            if present[metric] == 0:
                row[metric] = None
    runs.sort(key=lambda r: (r["timestamp"], r["run_id"], r["suite"]))
    return runs


def write_csv(path: Path, rows: list[dict[str, Any]], fieldnames: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            out = {}
            for field in fieldnames:
                value = row.get(field)
                out[field] = "" if value is None else value
            writer.writerow(out)


def build_metric_trend_rows(run_aggs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_suite_metric: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for row in run_aggs:
        for metric in METRIC_FIELDS:
            by_suite_metric[(row["suite"], metric)].append(row)

    trend_rows: list[dict[str, Any]] = []
    for (suite, metric), rows in sorted(by_suite_metric.items()):
        rows = sorted(
            (r for r in rows if r.get(metric) is not None),
            key=lambda r: r["timestamp"],
        )
        if len(rows) < 2:
            continue
        first = rows[0]
        last = rows[-1]
        first_val = safe_int(first.get(metric, 0))
        last_val = safe_int(last.get(metric, 0))
        delta = last_val - first_val
        trend_rows.append(
            {
                "suite": suite,
                "metric": metric,
                "runs": len(rows),
                "first_timestamp": first["timestamp"],
                "last_timestamp": last["timestamp"],
                "first_run_id": first["run_id"],
                "last_run_id": last["run_id"],
                "first_value": first_val,
                "last_value": last_val,
                "delta": delta,
                "delta_pct": "" if first_val == 0 else f"{(delta / first_val) * 100:.3f}",
                "min_value": min(safe_int(r.get(metric, 0)) for r in rows),
                "max_value": max(safe_int(r.get(metric, 0)) for r in rows),
            }
        )

    return trend_rows


def build_test_metric_trend_rows(points: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_key: dict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    for row in points:
        suite = row["suite"]
        test_name = row["test_name"]
        for metric in METRIC_FIELDS:
            by_key[(suite, test_name, metric)].append(row)

    out: list[dict[str, Any]] = []
    for (suite, test_name, metric), rows in sorted(by_key.items()):
        rows = sorted(
            (r for r in rows if r.get(metric) is not None),
            key=lambda r: r["timestamp"],
        )
        if len(rows) < 2:
            continue
        first = rows[0]
        last = rows[-1]
        first_val = safe_int(first.get(metric, 0))
        last_val = safe_int(last.get(metric, 0))
        delta = last_val - first_val
        out.append(
            {
                "suite": suite,
                "test_name": test_name,
                "metric": metric,
                "points": len(rows),
                "first_timestamp": first["timestamp"],
                "last_timestamp": last["timestamp"],
                "first_value": first_val,
                "last_value": last_val,
                "delta": delta,
                "delta_pct": "" if first_val == 0 else f"{(delta / first_val) * 100:.3f}",
                "min_value": min(safe_int(r.get(metric, 0)) for r in rows),
                "max_value": max(safe_int(r.get(metric, 0)) for r in rows),
            }
        )
    return out


def maybe_generate_plots(run_aggs: list[dict[str, Any]], output_dir: Path, enabled: bool) -> list[str]:
    if not enabled:
        return ["plot generation disabled"]

    try:
        import matplotlib.pyplot as plt
        import matplotlib.dates as mdates
    except Exception:
        return ["matplotlib not available; skipping plots"]

    notes: list[str] = []
    by_suite: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in run_aggs:
        by_suite[row["suite"]].append(row)
    for suite in by_suite:
        by_suite[suite].sort(key=lambda r: r["timestamp"])

    plotted = 0
    for metric in METRIC_FIELDS:
        fig, ax = plt.subplots(figsize=(12, 6))
        any_data = False
        for suite, rows in sorted(by_suite.items()):
            filtered = [r for r in rows if r.get(metric) is not None]
            if not filtered:
                continue
            x = [parse_ts(r["timestamp"]) for r in filtered]
            y = [safe_int(r.get(metric)) for r in filtered]
            if not any(v != 0 for v in y):
                continue
            any_data = True
            ax.plot(x, y, marker="o", linewidth=1.8, markersize=3.5, label=suite)

        if not any_data:
            plt.close(fig)
            continue

        ax.set_title(f"Run-level trend: {metric}")
        ax.set_xlabel("Timestamp")
        ax.set_ylabel(metric)
        ax.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m-%d"))
        ax.tick_params(axis="x", rotation=35)
        ax.grid(True, alpha=0.3)
        ax.legend()
        fig.tight_layout()

        out = output_dir / "plots" / f"trend_{metric}.png"
        out.parent.mkdir(parents=True, exist_ok=True)
        fig.savefig(out, dpi=150, bbox_inches="tight")
        plt.close(fig)
        plotted += 1

    notes.append(f"generated {plotted} plot(s)")
    return notes


def render_summary(
    points: list[dict[str, Any]],
    run_aggs: list[dict[str, Any]],
    metric_trends: list[dict[str, Any]],
    test_metric_trends: list[dict[str, Any]],
    plot_notes: list[str],
) -> str:
    now = datetime.now(timezone.utc).isoformat()

    ts_values = [parse_ts(r["timestamp"]) for r in points]
    min_ts = min(ts_values)
    max_ts = max(ts_values)
    span_days = (max_ts - min_ts).total_seconds() / 86400.0

    suites = sorted({r["suite"] for r in points})
    tests = sorted({r["test_name"] for r in points})
    runs = sorted({r["run_id"] for r in points})

    suite_run_counts: dict[str, int] = defaultdict(int)
    for row in run_aggs:
        suite_run_counts[row["suite"]] += 1

    lines: list[str] = []
    lines.append("# Perf History Trend Summary")
    lines.append("")
    lines.append(f"Generated: `{now}`")
    lines.append("")
    lines.append("## Coverage")
    lines.append("")
    lines.append(f"- Points: **{len(points)}**")
    lines.append(f"- Runs: **{len(runs)}**")
    lines.append(f"- Suites: **{len(suites)}** ({', '.join(suites)})")
    lines.append(f"- Unique tests: **{len(tests)}**")
    lines.append(f"- Time range: **{min_ts.isoformat()}** to **{max_ts.isoformat()}**")
    lines.append(f"- History span: **{span_days:.1f} days**")
    lines.append("")
    lines.append("Suite run counts:")
    for suite in suites:
        lines.append(f"- `{suite}`: {suite_run_counts.get(suite, 0)} runs")

    lines.append("")
    lines.append("## Run-Level Metric Trends")
    lines.append("")
    lines.append("Each row compares first vs latest run within a suite for one metric.")
    lines.append("")
    lines.append("| Suite | Metric | Runs | First | Latest | Delta | Delta % |")
    lines.append("| --- | --- | ---: | ---: | ---: | ---: | ---: |")

    for row in sorted(metric_trends, key=lambda r: (r["suite"], r["metric"])):
        delta_pct = "n/a" if row["delta_pct"] == "" else f"{float(row['delta_pct']):+.1f}%"
        lines.append(
            f"| `{row['suite']}` | `{row['metric']}` | {row['runs']} | {row['first_value']} | {row['last_value']} | {row['delta']:+} | {delta_pct} |"
        )

    lines.append("")
    lines.append("## Largest Test-Level Total-Time Changes")
    lines.append("")

    tt = [r for r in test_metric_trends if r["metric"] == "total_time_ms"]
    tt_improve = sorted((r for r in tt if r["delta"] < 0), key=lambda r: r["delta"])[:10]
    tt_regress = sorted((r for r in tt if r["delta"] > 0), key=lambda r: r["delta"], reverse=True)[
        :10
    ]

    lines.append("Top improvements (most negative delta):")
    if tt_improve:
        for row in tt_improve:
            lines.append(
                f"- `{row['suite']}/{row['test_name']}`: {row['first_value']} -> {row['last_value']} ({row['delta']:+})"
            )
    else:
        lines.append("- none")

    lines.append("")
    lines.append("Top regressions (most positive delta):")
    if tt_regress:
        for row in tt_regress:
            lines.append(
                f"- `{row['suite']}/{row['test_name']}`: {row['first_value']} -> {row['last_value']} ({row['delta']:+})"
            )
    else:
        lines.append("- none")

    lines.append("")
    lines.append("## Plot Output")
    lines.append("")
    for note in plot_notes:
        lines.append(f"- {note}")

    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build multi-metric perf history trend data across results and perf cycles"
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path("benchmarks/results"),
        help="Directory containing benchmark result JSON files",
    )
    parser.add_argument(
        "--perf-cycles-dir",
        type=Path,
        default=Path("benchmarks/perf_cycles"),
        help="Directory containing perf cycle folders",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("benchmarks/history"),
        help="Output directory for aggregated history artifacts",
    )
    parser.add_argument(
        "--plots",
        action="store_true",
        help="Generate plot PNGs if matplotlib is available",
    )
    parser.add_argument(
        "--tracked-only",
        action="store_true",
        help="Only include git-tracked benchmark artifacts",
    )
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    results_dir = args.results_dir if args.results_dir.is_absolute() else root / args.results_dir
    perf_cycles_dir = (
        args.perf_cycles_dir if args.perf_cycles_dir.is_absolute() else root / args.perf_cycles_dir
    )
    output_dir = args.output_dir if args.output_dir.is_absolute() else root / args.output_dir

    points = collect_points(
        results_dir=results_dir,
        perf_cycles_dir=perf_cycles_dir,
        root=root,
        tracked_only=args.tracked_only,
    )
    if not points:
        print("No historical perf points found.")
        return 1

    run_aggs = build_run_aggregates(points)
    metric_trends = build_metric_trend_rows(run_aggs)
    test_metric_trends = build_test_metric_trend_rows(points)

    output_dir.mkdir(parents=True, exist_ok=True)

    points_csv = output_dir / "history_points.csv"
    runs_csv = output_dir / "history_run_aggregates.csv"
    metric_csv = output_dir / "history_metric_trends.csv"
    test_metric_csv = output_dir / "history_test_metric_trends.csv"
    summary_md = output_dir / "history_trend_summary.md"

    write_csv(points_csv, points, POINT_FIELDS)
    write_csv(runs_csv, run_aggs, RUN_FIELDS)

    metric_fields = [
        "suite",
        "metric",
        "runs",
        "first_timestamp",
        "last_timestamp",
        "first_run_id",
        "last_run_id",
        "first_value",
        "last_value",
        "delta",
        "delta_pct",
        "min_value",
        "max_value",
    ]
    test_metric_fields = [
        "suite",
        "test_name",
        "metric",
        "points",
        "first_timestamp",
        "last_timestamp",
        "first_value",
        "last_value",
        "delta",
        "delta_pct",
        "min_value",
        "max_value",
    ]
    write_csv(metric_csv, metric_trends, metric_fields)
    write_csv(test_metric_csv, test_metric_trends, test_metric_fields)

    plot_notes = maybe_generate_plots(run_aggs, output_dir, args.plots)

    summary = render_summary(points, run_aggs, metric_trends, test_metric_trends, plot_notes)
    summary_md.write_text(summary, encoding="utf-8")

    print(f"Wrote: {points_csv}")
    print(f"Wrote: {runs_csv}")
    print(f"Wrote: {metric_csv}")
    print(f"Wrote: {test_metric_csv}")
    print(f"Wrote: {summary_md}")
    for note in plot_notes:
        print(f"Note: {note}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
