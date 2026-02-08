#!/usr/bin/env python3
"""
Export real-world dataset perf metrics to JSON (and CSV).

This script orchestrates:
- ensuring required public datasets exist in corpus_public/ (optional download)
- generating derived variants (via mutate_openxml_xlsx.py) for cases that require it
- generating Rust test list (e2e_perf_real_world.rs) from cases.yaml
- running the ignored real-world perf tests and parsing PERF_METRIC lines

Outputs:
- timestamped JSON in benchmarks/results_real_world/
- latest JSON at benchmarks/latest_real_world.json (or --latest-json)
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from real_world_lib import (
    download_url_to_path,
    ingest_file_to_corpus,
    kind_to_ext,
    load_cases,
    load_registry,
    resolve_dataset_path,
    save_json,
    sha256_path,
    utc_now_iso,
)


PERF_METRIC_RE = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")


CSV_FIELDS = [
    "case_id",
    "dataset_a",
    "dataset_b",
    "dataset_a_sha256",
    "dataset_b_sha256",
    "old_bytes",
    "new_bytes",
    "total_input_bytes",
    "workload_id",
    "op_count",
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
]


def git_cmd(args: list[str]) -> str:
    try:
        result = subprocess.run(["git", *args], capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass
    return "unknown"


def parse_perf_metrics(text: str) -> dict[str, dict[str, int]]:
    metrics: dict[str, dict[str, int]] = {}
    for line in text.splitlines():
        m = PERF_METRIC_RE.search(line)
        if not m:
            continue
        case_id = m.group(1)
        rest = m.group(2)
        data = {k: int(v) for k, v in re.findall(r"(\w+)=([0-9]+)", rest)}
        metrics[case_id] = data
    return metrics


def ensure_public_datasets_present(
    *,
    registry_path: Path,
    dataset_ids: set[str],
    corpus_dir: Path,
    index_path: Path,
    tmp_dir: Path,
    timeout_seconds: int,
    user_agent: str,
    hard_max_bytes: int,
) -> None:
    reg = load_registry(registry_path)
    by_id: dict[str, dict[str, Any]] = {}
    for ds in reg.get("datasets", []):
        if isinstance(ds, dict):
            ds_id = str(ds.get("id") or "").strip()
            if ds_id:
                by_id[ds_id] = ds

    missing = [dsid for dsid in sorted(dataset_ids) if dsid not in by_id]
    if missing:
        raise ValueError(f"Dataset ids missing from registry {registry_path}: {missing}")

    for dsid in sorted(dataset_ids):
        ds = by_id[dsid]
        kind = str(ds.get("kind") or "").strip()
        url = str(ds.get("source_url") or "").strip()
        expected_sha = str(ds.get("sha256") or "").strip().lower()
        if not kind or not url or not expected_sha:
            raise ValueError(f"Registry entry for {dsid} missing kind/source_url/sha256")

        ext = kind_to_ext(kind)
        blob_name = f"sha256_{expected_sha}.{ext.lstrip('.')}"
        blob_path = corpus_dir / blob_name
        if blob_path.exists():
            # Ensure index has dataset_id mapping.
            ingest_file_to_corpus(
                blob_path,
                corpus_dir=corpus_dir,
                index_path=index_path,
                dataset_id=dsid,
                source_tag=str(ds.get("source_homepage") or "").strip() or None,
            )
            continue

        tmp_path = tmp_dir / f"{dsid}{ext}"
        result = download_url_to_path(
            url,
            tmp_path,
            timeout_seconds=timeout_seconds,
            max_bytes=hard_max_bytes,
            expected_sha256=expected_sha,
            user_agent=user_agent,
        )
        ingest_file_to_corpus(
            result.path,
            corpus_dir=corpus_dir,
            index_path=index_path,
            dataset_id=dsid,
            source_tag=str(ds.get("source_homepage") or "").strip() or None,
        )


def load_or_init_derived_index(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "derived": []}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {"version": 1, "derived": []}
    if not isinstance(data, dict):
        return {"version": 1, "derived": []}
    if int(data.get("version", 0) or 0) != 1:
        return {"version": 1, "derived": []}
    if not isinstance(data.get("derived", []), list):
        data["derived"] = []
    return data


def save_derived_index(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def derived_b_dataset_id(case_id: str) -> str:
    return f"derived_b__{case_id}"


def materialize_derived_cases(
    *,
    cases: list[dict[str, Any]],
    corpus_dir: Path,
    index_path: Path,
    tmp_dir: Path,
) -> dict[str, dict[str, Any]]:
    """
    Materialize derived dataset_b entries into the public corpus.

    Returns a mapping of case_id -> provenance info.
    """
    tmp_dir.mkdir(parents=True, exist_ok=True)
    derived_index_path = corpus_dir / "derived_index.json"
    derived_index = load_or_init_derived_index(derived_index_path)

    provenance: dict[str, dict[str, Any]] = {}

    for case in cases:
        case_id = str(case.get("case_id") or "").strip()
        if not case_id:
            continue
        dataset_a = str(case.get("dataset_a") or "").strip()
        dataset_b = case.get("dataset_b")
        if not dataset_a or not isinstance(dataset_b, dict):
            continue

        derived_from = str(dataset_b.get("derived_from") or dataset_a).strip()
        derivation = dataset_b.get("derivation") or {}
        if not isinstance(derivation, dict):
            raise ValueError(f"case {case_id} dataset_b.derivation must be a mapping")

        tool = str(derivation.get("tool") or "mutate_openxml_xlsx.py").strip()
        mode = str(derivation.get("mode") or "").strip()
        seed = int(derivation.get("seed") or 0)
        edits = int(derivation.get("edits") or 10)
        row_count = int(derivation.get("row_count") or derivation.get("row-count") or 200)
        worksheet_part = str(derivation.get("worksheet_part") or derivation.get("worksheet-part") or "").strip()

        if mode not in {"cell_edit_numeric", "row_block_swap"}:
            raise ValueError(f"case {case_id} unsupported derivation mode: {mode}")

        src_path = resolve_dataset_path(dataset_id=derived_from, corpus_dir=corpus_dir, index_path=index_path)
        out_ext = src_path.suffix.lower()
        out_path = tmp_dir / f"{case_id}__derived_b{out_ext}"

        cmd = [
            sys.executable,
            f"scripts/{tool}",
            "--input",
            str(src_path),
            "--output",
            str(out_path),
            "--mode",
            mode,
            "--seed",
            str(seed),
        ]
        if worksheet_part:
            cmd.extend(["--worksheet-part", worksheet_part])
        if mode == "cell_edit_numeric":
            cmd.extend(["--edits", str(edits)])
        if mode == "row_block_swap":
            cmd.extend(["--row-count", str(row_count)])

        print("Running:", " ".join(cmd))
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print("STDOUT:", result.stdout)
            print("STDERR:", result.stderr, file=sys.stderr)
            raise RuntimeError(f"Derived mutation failed for case {case_id}")

        derived_id = derived_b_dataset_id(case_id)
        ingest_info = ingest_file_to_corpus(
            out_path,
            corpus_dir=corpus_dir,
            index_path=index_path,
            dataset_id=derived_id,
            source_tag="derived",
        )

        entry = {
            "case_id": case_id,
            "dataset_id": derived_id,
            "sha256": ingest_info["sha256"],
            "bytes": ingest_info["size_bytes"],
            "extension": ingest_info["extension"],
            "generated_at": utc_now_iso(),
            "derived_from": derived_from,
            "derived_from_sha256": sha256_path(src_path),
            "derivation": {
                "tool": tool,
                "mode": mode,
                "seed": seed,
                "edits": edits if mode == "cell_edit_numeric" else None,
                "row_count": row_count if mode == "row_block_swap" else None,
                "worksheet_part": worksheet_part or None,
            },
        }
        derived_index.setdefault("derived", []).append(entry)
        provenance[case_id] = entry

    save_derived_index(derived_index_path, derived_index)
    return provenance


def generate_rust_tests(cases_path: Path) -> None:
    cmd = [sys.executable, "scripts/generate_real_world_perf_tests.py", "--cases", str(cases_path)]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print("STDOUT:", result.stdout)
        print("STDERR:", result.stderr, file=sys.stderr)
        raise RuntimeError("Failed to generate Rust real-world perf tests")


def run_rust_tests(
    *,
    corpus_dir: Path,
    features: list[str],
    timeout_seconds: int,
) -> subprocess.CompletedProcess[str]:
    cmd = [
        "cargo",
        "test",
        "-p",
        "excel_diff",
        "--release",
        "--features",
        ",".join(features),
        "--test",
        "e2e_perf_real_world",
        "--",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]
    env = os.environ.copy()
    # Use an absolute path so the test works regardless of `cargo test` working directory.
    env["TABULENSIS_REAL_WORLD_CORPUS_DIR"] = str(corpus_dir.resolve())
    # This is safe even if not needed (core tests do not require licensing, but downstream tooling might).
    env.setdefault("TABULENSIS_LICENSE_SKIP", "1")

    print("Running:", " ".join(cmd))
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout_seconds, env=env)


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=CSV_FIELDS)
        writer.writeheader()
        for row in rows:
            writer.writerow({k: row.get(k, "") for k in CSV_FIELDS})


def main() -> int:
    parser = argparse.ArgumentParser(description="Export real-world perf metrics.")
    parser.add_argument(
        "--registry",
        type=Path,
        default=Path("datasets/real_world/registry.yaml"),
        help="Registry yaml path",
    )
    parser.add_argument(
        "--cases",
        type=Path,
        default=Path("datasets/real_world/cases.yaml"),
        help="Cases yaml path",
    )
    parser.add_argument(
        "--corpus-dir",
        type=Path,
        default=Path("corpus_public"),
        help="Corpus directory",
    )
    parser.add_argument(
        "--index",
        type=Path,
        default=None,
        help="Optional corpus index path (defaults to <corpus-dir>/index.json)",
    )
    parser.add_argument(
        "--tmp-dir",
        type=Path,
        default=Path("tmp/real_world_run"),
        help="Working directory for downloads and derived artifacts",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("benchmarks/results_real_world"),
        help="Directory for timestamped JSON results",
    )
    parser.add_argument(
        "--latest-json",
        type=Path,
        default=Path("benchmarks/latest_real_world.json"),
        help="Write the latest JSON here (in addition to timestamped output)",
    )
    parser.add_argument(
        "--export-csv",
        type=Path,
        default=None,
        help="Optional CSV export path",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=None,
        help="Optional baseline JSON to compare against",
    )
    parser.add_argument(
        "--baseline-slack",
        type=float,
        default=0.20,
        help="Allowed slack ratio vs baseline for total_time_ms and peak_memory_bytes",
    )
    parser.add_argument(
        "--skip-baseline-check",
        action="store_true",
        help="Skip baseline comparison even if --baseline is provided",
    )
    parser.add_argument(
        "--skip-download",
        action="store_true",
        help="Do not attempt to download missing registry datasets; require corpus already populated",
    )
    parser.add_argument(
        "--allow-empty",
        action="store_true",
        help="Succeed (and write an empty payload) when cases.yaml defines no cases",
    )
    parser.add_argument(
        "--parallel",
        action="store_true",
        help="Enable the parallel feature in Rust perf run",
    )
    parser.add_argument(
        "--extra-features",
        type=str,
        default="",
        help="Comma-separated extra Cargo features to enable (in addition to perf-metrics)",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=3600,
        help="Timeout for the Rust perf test run",
    )
    parser.add_argument(
        "--download-timeout-seconds",
        type=int,
        default=180,
        help="Per-download timeout seconds",
    )
    parser.add_argument(
        "--user-agent",
        type=str,
        default="tabulensis-real-world-downloader/1.0",
        help="User-Agent for dataset downloads",
    )
    parser.add_argument(
        "--hard-max-bytes",
        type=int,
        default=1024 * 1024 * 1024,
        help="Hard per-dataset size cap in bytes for downloads",
    )
    args = parser.parse_args()

    corpus_dir: Path = args.corpus_dir
    index_path: Path = args.index or (corpus_dir / "index.json")
    tmp_dir: Path = args.tmp_dir
    tmp_dir.mkdir(parents=True, exist_ok=True)

    data = load_cases(args.cases)
    raw_cases = data.get("cases", [])
    cases: list[dict[str, Any]] = [c for c in raw_cases if isinstance(c, dict)]

    if not cases:
        payload = {
            "timestamp": utc_now_iso(),
            "git_commit": git_cmd(["rev-parse", "HEAD"])[:12],
            "git_branch": git_cmd(["rev-parse", "--abbrev-ref", "HEAD"]),
            "suite": "real-world",
            "tests": {},
            "case_provenance": {},
            "summary": {"total_tests": 0},
        }
        args.latest_json.parent.mkdir(parents=True, exist_ok=True)
        save_json(args.latest_json, payload)
        if args.allow_empty:
            print("No cases defined; wrote empty latest JSON and exiting 0.")
            return 0
        print("ERROR: No cases defined in cases.yaml")
        return 2

    # Determine which registry datasets must exist (dataset_a and dataset_b when dataset_b is a string id).
    required_registry_ids: set[str] = set()
    for case in cases:
        dataset_a = str(case.get("dataset_a") or "").strip()
        if dataset_a:
            required_registry_ids.add(dataset_a)
        dataset_b = case.get("dataset_b")
        if isinstance(dataset_b, str) and dataset_b.strip():
            required_registry_ids.add(dataset_b.strip())
        elif isinstance(dataset_b, dict):
            derived_from = str(dataset_b.get("derived_from") or dataset_a).strip()
            if derived_from:
                required_registry_ids.add(derived_from)

    if not args.skip_download:
        ensure_public_datasets_present(
            registry_path=args.registry,
            dataset_ids=required_registry_ids,
            corpus_dir=corpus_dir,
            index_path=index_path,
            tmp_dir=tmp_dir / "downloads",
            timeout_seconds=int(args.download_timeout_seconds),
            user_agent=str(args.user_agent),
            hard_max_bytes=int(args.hard_max_bytes),
        )
    else:
        # Just verify.
        for dsid in sorted(required_registry_ids):
            _ = resolve_dataset_path(dataset_id=dsid, corpus_dir=corpus_dir, index_path=index_path)

    derived_prov = materialize_derived_cases(
        cases=cases,
        corpus_dir=corpus_dir,
        index_path=index_path,
        tmp_dir=tmp_dir / "derived",
    )

    generate_rust_tests(args.cases)

    features = ["perf-metrics"]
    if args.parallel:
        features.append("parallel")
    if args.extra_features.strip():
        for f in args.extra_features.split(","):
            f = f.strip()
            if f:
                features.append(f)

    result = run_rust_tests(corpus_dir=corpus_dir, features=features, timeout_seconds=int(args.timeout_seconds))
    stdout = result.stdout + "\n" + result.stderr
    if result.returncode != 0:
        print("ERROR: Rust real-world perf tests failed.")
        print("STDOUT/STDERR:", stdout)
        return 2

    metrics = parse_perf_metrics(stdout)
    if not metrics:
        print("ERROR: No PERF_METRIC lines captured from real-world perf tests.")
        return 2

    # Build provenance per case.
    case_prov: dict[str, dict[str, Any]] = {}
    csv_rows: list[dict[str, Any]] = []
    for case in cases:
        case_id = str(case.get("case_id") or "").strip()
        if not case_id or case_id not in metrics:
            continue
        dataset_a = str(case.get("dataset_a") or "").strip()
        dataset_b_raw = case.get("dataset_b")
        if isinstance(dataset_b_raw, str):
            dataset_b = dataset_b_raw.strip()
        else:
            dataset_b = derived_b_dataset_id(case_id)

        path_a = resolve_dataset_path(dataset_id=dataset_a, corpus_dir=corpus_dir, index_path=index_path)
        path_b = resolve_dataset_path(dataset_id=dataset_b, corpus_dir=corpus_dir, index_path=index_path)

        prov = {
            "dataset_a": dataset_a,
            "dataset_b": dataset_b,
            "dataset_a_sha256": sha256_path(path_a),
            "dataset_b_sha256": sha256_path(path_b),
            "dataset_a_bytes": int(path_a.stat().st_size),
            "dataset_b_bytes": int(path_b.stat().st_size),
            "workload": str(case.get("workload") or "diff_streaming_fast"),
        }
        if case_id in derived_prov:
            prov["derived"] = derived_prov[case_id]
        case_prov[case_id] = prov

        row = {"case_id": case_id, **prov}
        # Add numeric metrics.
        row.update(metrics[case_id])
        csv_rows.append(row)

    payload = {
        "timestamp": utc_now_iso(),
        "git_commit": git_cmd(["rev-parse", "HEAD"])[:12],
        "git_branch": git_cmd(["rev-parse", "--abbrev-ref", "HEAD"]),
        "suite": "real-world",
        "features": features,
        "tests": metrics,
        "case_provenance": case_prov,
        "summary": {
            "total_tests": len(metrics),
            "total_time_ms": sum(v.get("total_time_ms", 0) for v in metrics.values()),
            "max_peak_memory_bytes": max((v.get("peak_memory_bytes", 0) for v in metrics.values()), default=0),
        },
    }

    # Write timestamped results and latest JSON.
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")
    out_path = args.output_dir / f"{ts}.json"
    save_json(out_path, payload)
    save_json(args.latest_json, payload)
    print(f"Wrote: {out_path}")
    print(f"Wrote: {args.latest_json}")

    if args.export_csv:
        write_csv(args.export_csv, csv_rows)
        print(f"Wrote: {args.export_csv}")

    # Optional baseline check.
    if args.baseline and not args.skip_baseline_check:
        baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
        base_tests = baseline.get("tests", {}) if isinstance(baseline, dict) else {}
        slack = float(args.baseline_slack)
        failures = []
        for case_id, m in metrics.items():
            base = base_tests.get(case_id)
            if not isinstance(base, dict):
                continue
            for key in ["total_time_ms", "peak_memory_bytes"]:
                a = int(m.get(key, 0))
                b = int(base.get(key, 0))
                if b <= 0:
                    continue
                if a > int(b * (1.0 + slack)):
                    failures.append((case_id, key, a, b))
        if failures:
            print("ERROR: Baseline regressions detected:")
            for case_id, key, a, b in failures:
                print(f"  - {case_id} {key}: {a} > {b} * (1+{slack})")
            return 3

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
