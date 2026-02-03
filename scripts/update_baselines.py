import argparse
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


SUITE_SUFFIX = {
    "quick": "quick",
    "gate": "gate",
    "full-scale": "fullscale",
}


def run(cmd: list[str]) -> None:
    print(f"+ {' '.join(shlex.quote(part) for part in cmd)}", flush=True)
    subprocess.run(cmd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run perf suite and update pinned baselines.")
    parser.add_argument(
        "--suite",
        required=True,
        choices=sorted(SUITE_SUFFIX.keys()),
        help="Perf suite to run (quick, gate, full-scale).",
    )
    parser.add_argument(
        "--test-target",
        default=None,
        help="Optional test target to pass through (e.g., perf_large_grid_tests).",
    )
    parser.add_argument(
        "--skip-baseline",
        action="store_true",
        help="Skip baseline regression checks (still enforces absolute caps).",
    )
    parser.add_argument(
        "--parallel",
        action="store_true",
        help="Enable the parallel feature when running perf tests.",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    latest_suffix = SUITE_SUFFIX[args.suite]
    latest_json = repo_root / "benchmarks" / f"latest_{latest_suffix}.json"
    latest_csv = repo_root / "benchmarks" / f"latest_{latest_suffix}.csv"
    baseline_json = repo_root / "benchmarks" / "baselines" / f"{args.suite}.json"

    try:
        cmd = [
            sys.executable,
            "scripts/check_perf_thresholds.py",
            "--suite",
            args.suite,
            "--export-json",
            str(latest_json),
            "--export-csv",
            str(latest_csv),
        ]
        if args.parallel:
            cmd.append("--parallel")
        if args.test_target:
            cmd.extend(["--test-target", args.test_target])
        if args.skip_baseline:
            cmd.extend(
                [
                    "--baseline",
                    str(repo_root / "benchmarks" / "baselines" / "_skip_baseline.json"),
                ]
            )
        run(cmd)

        if baseline_json.exists():
            run(
                [
                    sys.executable,
                    "scripts/compare_perf_results.py",
                    str(baseline_json),
                    str(latest_json),
                ]
            )
        else:
            print(f"Baseline not found at {baseline_json}; creating a new one.")

        baseline_json.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(latest_json, baseline_json)
        print(f"Updated baseline: {baseline_json}")
    except subprocess.CalledProcessError as exc:
        return exc.returncode

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
