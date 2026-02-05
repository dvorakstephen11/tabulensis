#!/usr/bin/env python3
"""Noise-aware signal report for a perf cycle using run-level artifacts."""

from __future__ import annotations

import argparse
import json
import math
import statistics
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_METRICS: dict[str, list[str]] = {
    "full-scale": ["total_time_ms"],
    "e2e": ["total_time_ms", "parse_time_ms", "diff_time_ms"],
}


@dataclass(frozen=True)
class SignalRow:
    suite: str
    test_name: str
    metric: str
    pre_median: int
    post_median: int
    delta: int
    delta_pct: float | None
    pre_iqr: float
    post_iqr: float
    effect: float
    confidence: str
    pre_runs: int
    post_runs: int


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def percentile(sorted_vals: list[int], p: float) -> float:
    if not sorted_vals:
        return 0.0
    if len(sorted_vals) == 1:
        return float(sorted_vals[0])
    pos = (len(sorted_vals) - 1) * p
    lo = int(math.floor(pos))
    hi = int(math.ceil(pos))
    if lo == hi:
        return float(sorted_vals[lo])
    frac = pos - lo
    return sorted_vals[lo] + (sorted_vals[hi] - sorted_vals[lo]) * frac


def iqr(values: list[int]) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    q1 = percentile(ordered, 0.25)
    q3 = percentile(ordered, 0.75)
    return q3 - q1


def confidence_from_effect(effect: float) -> str:
    if effect >= 8.0:
        return "high"
    if effect >= 3.0:
        return "medium"
    return "low"


def metric_values_from_payloads(
    payloads: list[dict[str, Any]],
) -> dict[str, dict[str, list[int]]]:
    out: dict[str, dict[str, list[int]]] = {}
    for payload in payloads:
        tests = payload.get("tests", {})
        if not isinstance(tests, dict):
            continue
        for test_name, metrics in tests.items():
            if not isinstance(metrics, dict):
                continue
            by_metric = out.setdefault(str(test_name), {})
            for metric, value in metrics.items():
                if not isinstance(value, (int, float)):
                    continue
                by_metric.setdefault(str(metric), []).append(int(value))
    return out


def run_json_paths(meta: dict[str, Any], stage_key: str, suite: str) -> list[Path]:
    stage = meta.get(stage_key)
    if not isinstance(stage, dict):
        return []

    if suite == "full-scale":
        run_list_key = "fullscale_run_json"
        fallback_key = "fullscale_json"
    else:
        run_list_key = "e2e_run_json"
        fallback_key = "e2e_json"

    rels = stage.get(run_list_key)
    out: list[Path] = []
    if isinstance(rels, list):
        for rel in rels:
            if isinstance(rel, str) and rel.strip():
                out.append(ROOT / rel)

    if out:
        return out

    fallback = stage.get(fallback_key)
    if isinstance(fallback, str) and fallback.strip():
        return [ROOT / fallback]
    return []


def build_rows(
    suite: str,
    metrics: list[str],
    pre_payloads: list[dict[str, Any]],
    post_payloads: list[dict[str, Any]],
) -> list[SignalRow]:
    pre = metric_values_from_payloads(pre_payloads)
    post = metric_values_from_payloads(post_payloads)
    tests = sorted(set(pre.keys()) | set(post.keys()))

    rows: list[SignalRow] = []
    for test_name in tests:
        pre_metrics = pre.get(test_name, {})
        post_metrics = post.get(test_name, {})
        for metric in metrics:
            pre_vals = pre_metrics.get(metric, [])
            post_vals = post_metrics.get(metric, [])
            if not pre_vals or not post_vals:
                continue
            pre_median = int(round(statistics.median(pre_vals)))
            post_median = int(round(statistics.median(post_vals)))
            delta = post_median - pre_median
            delta_pct = None if pre_median == 0 else (delta / pre_median) * 100.0
            pre_iqr = iqr(pre_vals)
            post_iqr = iqr(post_vals)
            noise = max(pre_iqr, post_iqr, 1.0)
            effect = abs(delta) / noise
            rows.append(
                SignalRow(
                    suite=suite,
                    test_name=test_name,
                    metric=metric,
                    pre_median=pre_median,
                    post_median=post_median,
                    delta=delta,
                    delta_pct=delta_pct,
                    pre_iqr=pre_iqr,
                    post_iqr=post_iqr,
                    effect=effect,
                    confidence=confidence_from_effect(effect),
                    pre_runs=len(pre_vals),
                    post_runs=len(post_vals),
                )
            )

    rows.sort(key=lambda row: (row.suite, row.metric, row.test_name))
    return rows


def fmt_delta_pct(value: float | None) -> str:
    if value is None:
        return "n/a"
    return f"{value:+.1f}%"


def render_summary(cycle: str, meta: dict[str, Any], rows: list[SignalRow]) -> str:
    now = datetime.now(timezone.utc).isoformat()
    pre = meta.get("pre", {}) if isinstance(meta.get("pre"), dict) else {}
    post = meta.get("post", {}) if isinstance(meta.get("post"), dict) else {}
    config = meta.get("config", {}) if isinstance(meta.get("config"), dict) else {}

    lines: list[str] = []
    lines.append("# Perf Cycle Signal Report")
    lines.append("")
    lines.append(f"Generated: `{now}`")
    lines.append(f"Cycle: `{cycle}`")
    lines.append(
        f"Commits: pre `{pre.get('git_commit', 'unknown')}` -> post `{post.get('git_commit', 'unknown')}`"
    )
    lines.append(
        f"Aggregation: `{config.get('aggregation', 'median')}` over `{config.get('runs', 'unknown')}` run(s)"
    )
    lines.append("")
    lines.append("Confidence model:")
    lines.append("- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`")
    lines.append("- `high` >= 8, `medium` >= 3, `low` < 3")
    lines.append("- Use confidence to separate likely signal from runtime noise")
    lines.append("")

    if not rows:
        lines.append("No comparable run-level metrics found.")
        lines.append("")
        return "\n".join(lines)

    total_time_rows = [row for row in rows if row.metric == "total_time_ms"]
    high_improve = sorted(
        (row for row in total_time_rows if row.delta < 0 and row.confidence == "high"),
        key=lambda row: row.delta,
    )
    high_regress = sorted(
        (row for row in total_time_rows if row.delta > 0 and row.confidence == "high"),
        key=lambda row: row.delta,
        reverse=True,
    )

    lines.append("## High-Confidence Summary (`total_time_ms`)")
    lines.append("")
    lines.append(f"- High-confidence improvements: **{len(high_improve)}**")
    lines.append(f"- High-confidence regressions: **{len(high_regress)}**")
    lines.append("")

    if high_improve:
        lines.append("Top high-confidence improvements:")
        for row in high_improve[:8]:
            lines.append(
                f"- `{row.suite}/{row.test_name}`: {row.pre_median} -> {row.post_median} "
                f"({row.delta:+}, {fmt_delta_pct(row.delta_pct)}, effect={row.effect:.2f})"
            )
        lines.append("")

    if high_regress:
        lines.append("Top high-confidence regressions:")
        for row in high_regress[:8]:
            lines.append(
                f"- `{row.suite}/{row.test_name}`: {row.pre_median} -> {row.post_median} "
                f"({row.delta:+}, {fmt_delta_pct(row.delta_pct)}, effect={row.effect:.2f})"
            )
        lines.append("")

    by_suite_metric: dict[tuple[str, str], list[SignalRow]] = {}
    for row in rows:
        by_suite_metric.setdefault((row.suite, row.metric), []).append(row)

    for suite_metric in sorted(by_suite_metric.keys()):
        suite, metric = suite_metric
        lines.append(f"## {suite} `{metric}`")
        lines.append("")
        lines.append(
            "| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |"
        )
        lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |")
        for row in sorted(by_suite_metric[suite_metric], key=lambda item: item.test_name):
            lines.append(
                f"| `{row.test_name}` | {row.pre_median} | {row.post_median} | "
                f"{row.delta:+} | {fmt_delta_pct(row.delta_pct)} | "
                f"{row.pre_iqr:.1f} | {row.post_iqr:.1f} | {row.effect:.2f} | {row.confidence} |"
            )
        lines.append("")

    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate a noise-aware signal report from perf-cycle run-level artifacts."
    )
    parser.add_argument("--cycle", type=str, required=True, help="Cycle id")
    parser.add_argument(
        "--output-md",
        type=str,
        default="",
        help="Output markdown path (default: benchmarks/perf_cycles/<cycle>/cycle_signal.md)",
    )
    parser.add_argument(
        "--output-json",
        type=str,
        default="",
        help="Output JSON path (default: benchmarks/perf_cycles/<cycle>/cycle_signal.json)",
    )
    args = parser.parse_args()

    cycle_dir = ROOT / "benchmarks" / "perf_cycles" / args.cycle
    meta_path = cycle_dir / "cycle.json"
    if not meta_path.exists():
        print(f"ERROR: missing cycle metadata: {meta_path}")
        return 2

    meta = load_json(meta_path)
    all_rows: list[SignalRow] = []
    for suite, metrics in DEFAULT_METRICS.items():
        pre_paths = run_json_paths(meta, "pre", suite)
        post_paths = run_json_paths(meta, "post", suite)
        if not pre_paths or not post_paths:
            continue

        pre_payloads = []
        post_payloads = []
        for path in pre_paths:
            if path.exists():
                pre_payloads.append(load_json(path))
        for path in post_paths:
            if path.exists():
                post_payloads.append(load_json(path))
        if not pre_payloads or not post_payloads:
            continue
        all_rows.extend(build_rows(suite, metrics, pre_payloads, post_payloads))

    summary_md = render_summary(args.cycle, meta, all_rows)
    out_md = Path(args.output_md) if args.output_md else cycle_dir / "cycle_signal.md"
    out_json = Path(args.output_json) if args.output_json else cycle_dir / "cycle_signal.json"

    payload = {
        "cycle": args.cycle,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "config": meta.get("config", {}),
        "pre": meta.get("pre", {}),
        "post": meta.get("post", {}),
        "rows": [
            {
                "suite": row.suite,
                "test_name": row.test_name,
                "metric": row.metric,
                "pre_median": row.pre_median,
                "post_median": row.post_median,
                "delta": row.delta,
                "delta_pct": row.delta_pct,
                "pre_iqr": row.pre_iqr,
                "post_iqr": row.post_iqr,
                "effect": row.effect,
                "confidence": row.confidence,
                "pre_runs": row.pre_runs,
                "post_runs": row.post_runs,
            }
            for row in all_rows
        ],
    }

    write_text(out_md, summary_md)
    write_json(out_json, payload)
    print(f"Signal report written to {out_md}")
    print(f"Signal JSON written to {out_json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
