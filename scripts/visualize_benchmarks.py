#!/usr/bin/env python3
"""
Visualize benchmark trends from combined_results.csv.

Usage:
    python scripts/visualize_benchmarks.py [--input FILE] [--output-dir DIR] [--show]

Options:
    --input       Input CSV file (default: benchmarks/results/combined_results.csv)
    --output-dir  Directory to save plots (default: benchmarks/results/plots)
    --show        Display plots interactively instead of saving
"""

import argparse
import sys
from datetime import datetime
from pathlib import Path

try:
    import matplotlib.pyplot as plt
    import matplotlib.dates as mdates
    import pandas as pd
except ImportError as e:
    print(f"Missing required dependency: {e}")
    print("Install with: pip install matplotlib pandas")
    sys.exit(1)


COLORS = [
    "#2ecc71", "#3498db", "#9b59b6", "#e74c3c", "#f39c12",
    "#1abc9c", "#e67e22", "#34495e", "#16a085", "#c0392b",
]


def load_data(csv_path: Path) -> pd.DataFrame:
    df = pd.read_csv(csv_path)
    df["timestamp"] = pd.to_datetime(df["timestamp"])
    df = df[df["test_name"] != "_SUMMARY_"]
    df = df.sort_values("timestamp")
    return df


def plot_time_trends(df: pd.DataFrame, output_dir: Path, show: bool = False):
    fig, ax = plt.subplots(figsize=(14, 8))

    quick_df = df[df["full_scale"] == False]
    full_df = df[df["full_scale"] == True]

    for i, (scale_df, scale_name) in enumerate([(quick_df, "Quick"), (full_df, "Full-Scale")]):
        if scale_df.empty:
            continue

        test_names = scale_df["test_name"].unique()
        for j, test_name in enumerate(test_names):
            test_data = scale_df[scale_df["test_name"] == test_name]
            color = COLORS[j % len(COLORS)]
            linestyle = "-" if scale_name == "Quick" else "--"
            marker = "o" if scale_name == "Quick" else "s"
            label = f"{test_name} ({scale_name})"
            ax.plot(
                test_data["timestamp"],
                test_data["total_time_ms"],
                marker=marker,
                linestyle=linestyle,
                color=color,
                label=label,
                markersize=6,
                linewidth=2,
                alpha=0.8,
            )

    ax.set_xlabel("Timestamp", fontsize=12)
    ax.set_ylabel("Total Time (ms)", fontsize=12)
    ax.set_title("Benchmark Performance Over Time", fontsize=14, fontweight="bold")
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%m/%d %H:%M"))
    ax.tick_params(axis="x", rotation=45)
    ax.legend(bbox_to_anchor=(1.02, 1), loc="upper left", fontsize=9)
    ax.grid(True, alpha=0.3)
    ax.set_yscale("log")
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "time_trends.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'time_trends.png'}")
    plt.close(fig)


def plot_speedup_heatmap(df: pd.DataFrame, output_dir: Path, show: bool = False):
    quick_df = df[df["full_scale"] == False].copy()
    if quick_df.empty:
        print("No quick-scale data for speedup heatmap")
        return

    runs = quick_df.groupby("source_file")["timestamp"].first().sort_values()
    if len(runs) < 2:
        print("Need at least 2 runs for speedup comparison")
        return

    pivot = quick_df.pivot_table(
        index="test_name",
        columns="source_file",
        values="total_time_ms",
        aggfunc="first",
    )
    pivot = pivot[runs.index]

    speedup = pd.DataFrame(index=pivot.index)
    run_files = list(pivot.columns)
    for i in range(1, len(run_files)):
        prev_run = run_files[i - 1]
        curr_run = run_files[i]
        col_name = f"{curr_run[:10]}"
        speedup[col_name] = ((pivot[prev_run] - pivot[curr_run]) / pivot[prev_run] * 100).round(1)

    if speedup.empty or speedup.shape[1] == 0:
        print("Not enough data for speedup heatmap")
        return

    fig, ax = plt.subplots(figsize=(max(10, len(speedup.columns) * 1.5), max(6, len(speedup) * 0.6)))

    im = ax.imshow(speedup.values, cmap="RdYlGn", aspect="auto", vmin=-50, vmax=50)

    ax.set_xticks(range(len(speedup.columns)))
    ax.set_xticklabels(speedup.columns, rotation=45, ha="right", fontsize=9)
    ax.set_yticks(range(len(speedup.index)))
    ax.set_yticklabels(speedup.index, fontsize=9)

    for i in range(len(speedup.index)):
        for j in range(len(speedup.columns)):
            val = speedup.iloc[i, j]
            if pd.notna(val):
                color = "white" if abs(val) > 25 else "black"
                text = f"{val:+.0f}%" if val != 0 else "0%"
                ax.text(j, i, text, ha="center", va="center", color=color, fontsize=8)

    cbar = fig.colorbar(im, ax=ax, shrink=0.8)
    cbar.set_label("Speedup % (positive = faster)", fontsize=10)

    ax.set_title("Performance Change Between Runs (Quick Tests)", fontsize=14, fontweight="bold")
    ax.set_xlabel("Run", fontsize=12)
    ax.set_ylabel("Test", fontsize=12)
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "speedup_heatmap.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'speedup_heatmap.png'}")
    plt.close(fig)


def plot_latest_comparison(df: pd.DataFrame, output_dir: Path, show: bool = False):
    latest_runs = df.groupby("full_scale")["source_file"].apply(lambda x: x.iloc[-1] if len(x) > 0 else None)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    for idx, (full_scale, ax) in enumerate(zip([False, True], axes)):
        if full_scale not in latest_runs.index or latest_runs[full_scale] is None:
            ax.text(0.5, 0.5, "No data", ha="center", va="center", transform=ax.transAxes)
            ax.set_title(f"{'Full-Scale' if full_scale else 'Quick'} Tests - No Data")
            continue

        latest_file = latest_runs[full_scale]
        latest_data = df[(df["source_file"] == latest_file) & (df["full_scale"] == full_scale)]

        if latest_data.empty:
            continue

        tests = latest_data["test_name"].values
        times = latest_data["total_time_ms"].values

        colors = [COLORS[i % len(COLORS)] for i in range(len(tests))]
        bars = ax.barh(tests, times, color=colors, alpha=0.8)

        for bar, time in zip(bars, times):
            ax.text(
                bar.get_width() + max(times) * 0.01,
                bar.get_y() + bar.get_height() / 2,
                f"{time:,.0f}ms",
                va="center",
                fontsize=9,
            )

        scale_name = "Full-Scale (50K rows)" if full_scale else "Quick (1-2K rows)"
        timestamp = latest_data["timestamp"].iloc[0].strftime("%Y-%m-%d %H:%M")
        ax.set_title(f"{scale_name}\n{latest_file} ({timestamp})", fontsize=11, fontweight="bold")
        ax.set_xlabel("Time (ms)", fontsize=10)
        ax.set_xlim(0, max(times) * 1.15)
        ax.grid(True, axis="x", alpha=0.3)

    fig.suptitle("Latest Benchmark Results", fontsize=14, fontweight="bold", y=1.02)
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "latest_comparison.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'latest_comparison.png'}")
    plt.close(fig)


def plot_metric_breakdown(df: pd.DataFrame, output_dir: Path, show: bool = False):
    metrics = ["move_detection_time_ms", "alignment_time_ms", "cell_diff_time_ms"]
    available_metrics = [m for m in metrics if m in df.columns and df[m].notna().any()]

    if not available_metrics:
        print("No detailed timing metrics available for breakdown chart")
        return

    latest_quick = df[df["full_scale"] == False].groupby("test_name").last().reset_index()
    latest_full = df[df["full_scale"] == True].groupby("test_name").last().reset_index()

    for scale_name, scale_df in [("Quick", latest_quick), ("Full-Scale", latest_full)]:
        if scale_df.empty:
            continue

        scale_df = scale_df[scale_df[available_metrics].notna().any(axis=1)]
        if scale_df.empty:
            continue

        fig, ax = plt.subplots(figsize=(12, 6))

        tests = scale_df["test_name"].values
        x = range(len(tests))
        width = 0.25

        metric_labels = {
            "move_detection_time_ms": "Fingerprinting + Move Detection",
            "alignment_time_ms": "Alignment (incl. diff)",
            "cell_diff_time_ms": "Cell Diff",
        }

        for i, metric in enumerate(available_metrics):
            values = scale_df[metric].fillna(0).values
            offset = (i - len(available_metrics) / 2 + 0.5) * width
            bars = ax.bar([xi + offset for xi in x], values, width, label=metric_labels.get(metric, metric), color=COLORS[i], alpha=0.8)

        ax.set_xlabel("Test", fontsize=12)
        ax.set_ylabel("Time (ms)", fontsize=12)
        ax.set_title(f"Timing Breakdown by Phase ({scale_name} Tests)", fontsize=14, fontweight="bold")
        ax.set_xticks(x)
        ax.set_xticklabels(tests, rotation=45, ha="right", fontsize=9)
        ax.legend()
        ax.grid(True, axis="y", alpha=0.3)
        fig.tight_layout()

        suffix = "quick" if scale_name == "Quick" else "fullscale"
        if show:
            plt.show()
        else:
            fig.savefig(output_dir / f"metric_breakdown_{suffix}.png", dpi=150, bbox_inches="tight")
            print(f"Saved: {output_dir / f'metric_breakdown_{suffix}.png'}")
        plt.close(fig)


def plot_commit_comparison(df: pd.DataFrame, output_dir: Path, show: bool = False):
    quick_df = df[df["full_scale"] == False].copy()
    if quick_df.empty:
        print("No quick-scale data for commit comparison")
        return

    commit_totals = quick_df.groupby(["git_commit", "source_file"])["total_time_ms"].sum().reset_index()
    commit_totals = commit_totals.sort_values("source_file")

    if len(commit_totals) < 2:
        print("Need at least 2 commits for comparison")
        return

    fig, ax = plt.subplots(figsize=(12, 6))

    commits = commit_totals["git_commit"].values
    totals = commit_totals["total_time_ms"].values
    files = commit_totals["source_file"].values

    colors = [COLORS[i % len(COLORS)] for i in range(len(commits))]
    bars = ax.bar(range(len(commits)), totals, color=colors, alpha=0.8)

    for i, (bar, total, commit, fname) in enumerate(zip(bars, totals, commits, files)):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + max(totals) * 0.01,
            f"{total:,.0f}ms",
            ha="center",
            va="bottom",
            fontsize=9,
        )

    ax.set_xlabel("Commit", fontsize=12)
    ax.set_ylabel("Total Time (ms)", fontsize=12)
    ax.set_title("Total Test Suite Time by Commit (Quick Tests)", fontsize=14, fontweight="bold")
    ax.set_xticks(range(len(commits)))
    labels = [f"{c[:8]}\n{f[:10]}" for c, f in zip(commits, files)]
    ax.set_xticklabels(labels, rotation=0, fontsize=8)
    ax.grid(True, axis="y", alpha=0.3)

    if len(totals) >= 2:
        first_total = totals[0]
        last_total = totals[-1]
        overall_change = ((last_total - first_total) / first_total) * 100
        direction = "faster" if overall_change < 0 else "slower"
        ax.text(
            0.98, 0.98,
            f"Overall: {abs(overall_change):.1f}% {direction}",
            transform=ax.transAxes,
            ha="right", va="top",
            fontsize=11,
            fontweight="bold",
            color="green" if overall_change < 0 else "red",
            bbox=dict(boxstyle="round", facecolor="white", alpha=0.8),
        )

    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "commit_comparison.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'commit_comparison.png'}")
    plt.close(fig)


def generate_summary_report(df: pd.DataFrame, output_dir: Path):
    lines = [
        "# Benchmark Trend Summary",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "## Overview",
        "",
        f"- Total benchmark runs: {df['source_file'].nunique()}",
        f"- Quick-scale runs: {df[df['full_scale'] == False]['source_file'].nunique()}",
        f"- Full-scale runs: {df[df['full_scale'] == True]['source_file'].nunique()}",
        f"- Unique tests: {df['test_name'].nunique()}",
        f"- Date range: {df['timestamp'].min().strftime('%Y-%m-%d')} to {df['timestamp'].max().strftime('%Y-%m-%d')}",
        "",
    ]

    for scale_name, full_scale in [("Quick", False), ("Full-Scale", True)]:
        scale_df = df[df["full_scale"] == full_scale]
        if scale_df.empty:
            continue

        lines.extend([f"## {scale_name} Tests Performance", ""])

        runs = scale_df.groupby("source_file")["timestamp"].first().sort_values()
        if len(runs) >= 2:
            first_run = runs.index[0]
            last_run = runs.index[-1]

            first_total = scale_df[scale_df["source_file"] == first_run]["total_time_ms"].sum()
            last_total = scale_df[scale_df["source_file"] == last_run]["total_time_ms"].sum()
            change = ((last_total - first_total) / first_total) * 100

            lines.extend([
                f"- First run total: {first_total:,.0f}ms ({first_run})",
                f"- Latest run total: {last_total:,.0f}ms ({last_run})",
                f"- Overall change: {change:+.1f}% ({'faster' if change < 0 else 'slower'})",
                "",
            ])

        lines.append("### Per-Test Trends")
        lines.append("")
        lines.append("| Test | First (ms) | Latest (ms) | Change |")
        lines.append("|:-----|----------:|------------:|-------:|")

        for test_name in scale_df["test_name"].unique():
            test_data = scale_df[scale_df["test_name"] == test_name].sort_values("timestamp")
            if len(test_data) >= 2:
                first_time = test_data.iloc[0]["total_time_ms"]
                last_time = test_data.iloc[-1]["total_time_ms"]
                pct_change = ((last_time - first_time) / first_time) * 100
                lines.append(f"| {test_name} | {first_time:,.0f} | {last_time:,.0f} | {pct_change:+.1f}% |")
            elif len(test_data) == 1:
                lines.append(f"| {test_name} | {test_data.iloc[0]['total_time_ms']:,.0f} | - | N/A |")

        lines.extend(["", ""])

    report_path = output_dir / "trend_summary.md"
    report_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"Saved: {report_path}")


def main():
    parser = argparse.ArgumentParser(description="Visualize benchmark trends")
    parser.add_argument(
        "--input",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results" / "combined_results.csv",
        help="Input CSV file",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Output directory for plots",
    )
    parser.add_argument(
        "--show",
        action="store_true",
        help="Display plots interactively",
    )
    args = parser.parse_args()

    if not args.input.exists():
        print(f"ERROR: Input file not found: {args.input}")
        print("Run scripts/combine_results_to_csv.py first to generate the combined CSV.")
        return 1

    if args.output_dir is None:
        args.output_dir = args.input.parent / "plots"

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading data from: {args.input}")
    df = load_data(args.input)
    print(f"Loaded {len(df)} data points from {df['source_file'].nunique()} benchmark runs")
    print()

    print("Generating visualizations...")
    plot_time_trends(df, args.output_dir, args.show)
    plot_speedup_heatmap(df, args.output_dir, args.show)
    plot_latest_comparison(df, args.output_dir, args.show)
    plot_metric_breakdown(df, args.output_dir, args.show)
    plot_commit_comparison(df, args.output_dir, args.show)
    generate_summary_report(df, args.output_dir)

    print()
    print(f"All outputs saved to: {args.output_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
