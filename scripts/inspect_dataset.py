#!/usr/bin/env python3
"""
Inspect a dataset file (xlsx/xlsm/xltx/xltm/pbix/pbit) and write a small JSON summary.

This script is designed to:
- be safe on untrusted files (no execution; zip stats only by default)
- help select datasets that stress specific performance axes
"""

from __future__ import annotations

import argparse
import json
import re
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from real_world_lib import (
    kind_to_ext,
    load_registry,
    resolve_dataset_path,
    load_corpus_index,
    sha256_path,
)


DIMENSION_RE = re.compile(r'<dimension\s+[^>]*ref="([^"]+)"')


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def safe_read_prefix(zf: zipfile.ZipFile, name: str, max_bytes: int) -> bytes:
    with zf.open(name, "r") as handle:
        return handle.read(max_bytes)


def compute_zip_stats(zf: zipfile.ZipFile) -> dict[str, Any]:
    infos = zf.infolist()
    entry_count = len(infos)
    total_uncompressed = 0
    max_entry_uncompressed = 0
    max_ratio = 0.0
    max_ratio_entry = ""

    for info in infos:
        total_uncompressed += int(info.file_size)
        max_entry_uncompressed = max(max_entry_uncompressed, int(info.file_size))
        if info.compress_size > 0:
            ratio = float(info.file_size) / float(info.compress_size)
            if ratio > max_ratio:
                max_ratio = ratio
                max_ratio_entry = info.filename

    return {
        "entry_count": entry_count,
        "total_uncompressed_bytes": total_uncompressed,
        "max_entry_uncompressed_bytes": max_entry_uncompressed,
        "max_compression_ratio": max_ratio,
        "max_compression_ratio_entry": max_ratio_entry,
    }


def parse_sheet_count(workbook_xml: bytes) -> int | None:
    # Avoid full XML parsing; sheet elements are simple.
    # Example: <sheet name="..." sheetId="1" r:id="rId1"/>
    try:
        text = workbook_xml.decode("utf-8", errors="ignore")
    except Exception:
        return None
    return text.count("<sheet ")


def find_dimensions(zf: zipfile.ZipFile, worksheet_names: list[str]) -> dict[str, str]:
    dims: dict[str, str] = {}
    for name in worksheet_names:
        try:
            prefix = safe_read_prefix(zf, name, 1024 * 1024)  # 1 MiB
        except Exception:
            continue
        m = DIMENSION_RE.search(prefix.decode("utf-8", errors="ignore"))
        if m:
            dims[name] = m.group(1)
    return dims


def inspect_openxml_like(path: Path, *, deep: bool) -> dict[str, Any]:
    with zipfile.ZipFile(path, "r") as zf:
        stats = compute_zip_stats(zf)
        names = set(zf.namelist())

        workbook_path = "xl/workbook.xml"
        shared_strings_path = "xl/sharedStrings.xml"
        styles_path = "xl/styles.xml"

        workbook_info: dict[str, Any] = {"path": workbook_path, "present": workbook_path in names}
        if workbook_path in names:
            wb_bytes = safe_read_prefix(zf, workbook_path, 256 * 1024)
            workbook_info["uncompressed_bytes_prefix"] = len(wb_bytes)
            workbook_info["sheet_count_estimate"] = parse_sheet_count(wb_bytes)

        worksheets = sorted([n for n in names if n.startswith("xl/worksheets/") and n.endswith(".xml")])
        worksheet_stats = []
        for ws in worksheets:
            try:
                info = zf.getinfo(ws)
            except KeyError:
                continue
            worksheet_stats.append(
                {
                    "path": ws,
                    "compressed_bytes": int(info.compress_size),
                    "uncompressed_bytes": int(info.file_size),
                }
            )

        dims = find_dimensions(zf, worksheets)

        shared_info: dict[str, Any] = {
            "path": shared_strings_path,
            "present": shared_strings_path in names,
        }
        if shared_strings_path in names:
            info = zf.getinfo(shared_strings_path)
            shared_info["compressed_bytes"] = int(info.compress_size)
            shared_info["uncompressed_bytes"] = int(info.file_size)

        styles_info: dict[str, Any] = {"path": styles_path, "present": styles_path in names}
        if styles_path in names:
            info = zf.getinfo(styles_path)
            styles_info["compressed_bytes"] = int(info.compress_size)
            styles_info["uncompressed_bytes"] = int(info.file_size)

        # Deep counts are best-effort and intentionally limited.
        # (Add more when we need it; keep default inspection cheap.)
        if deep and shared_strings_path in names:
            # Count <si> elements (cheap-ish, but still scans the file).
            count = 0
            with zf.open(shared_strings_path, "r") as handle:
                for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                    count += chunk.count(b"<si")
            shared_info["si_count_estimate"] = int(count)

        result: dict[str, Any] = {
            "kind": "openxml",
            "zip": stats,
            "workbook": workbook_info,
            "worksheets": worksheet_stats,
            "worksheet_dimensions": dims,
            "shared_strings": shared_info,
            "styles": styles_info,
        }
        return result


def inspect_pbix_like(path: Path) -> dict[str, Any]:
    with zipfile.ZipFile(path, "r") as zf:
        stats = compute_zip_stats(zf)
        names = set(zf.namelist())
        # Power BI packages commonly include DataMashup (legacy).
        dm_paths = [n for n in names if n.lower().endswith("datamashup")]
        dm_entries = []
        for n in sorted(dm_paths):
            info = zf.getinfo(n)
            dm_entries.append(
                {
                    "path": n,
                    "compressed_bytes": int(info.compress_size),
                    "uncompressed_bytes": int(info.file_size),
                }
            )
        return {
            "kind": "pbix",
            "zip": stats,
            "datamashup_entries": dm_entries,
        }


def main() -> int:
    parser = argparse.ArgumentParser(description="Inspect a real-world dataset and write JSON summary.")
    parser.add_argument(
        "--registry",
        type=Path,
        default=None,
        help="Optional registry.yaml (required for --dataset-id resolution)",
    )
    parser.add_argument(
        "--cases",
        type=Path,
        default=None,
        help="Unused for now (reserved)",
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
        "--dataset-id",
        type=str,
        default="",
        help="Dataset id to inspect (resolved via corpus index)",
    )
    parser.add_argument(
        "--path",
        type=Path,
        default=None,
        help="Inspect a direct path (bypasses dataset-id resolution)",
    )
    parser.add_argument(
        "--deep",
        action="store_true",
        help="Enable deeper inspection (may be slower)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write JSON output to this path",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write to datasets/real_world/inspections/<dataset-id>.json (requires --dataset-id)",
    )
    args = parser.parse_args()

    corpus_dir: Path = args.corpus_dir
    index_path: Path = args.index or (corpus_dir / "index.json")

    dataset_id = args.dataset_id.strip() or None
    path = args.path

    registry_entry: dict[str, Any] | None = None
    if dataset_id:
        path = resolve_dataset_path(dataset_id=dataset_id, corpus_dir=corpus_dir, index_path=index_path)
        if args.registry:
            reg = load_registry(args.registry)
            for ds in reg.get("datasets", []):
                if isinstance(ds, dict) and str(ds.get("id") or "") == dataset_id:
                    registry_entry = ds
                    break

    if not path:
        raise SystemExit("ERROR: Provide --dataset-id or --path")
    if not path.exists():
        raise FileNotFoundError(str(path))

    ext = path.suffix.lower()
    file_sha = sha256_path(path)
    payload: dict[str, Any] = {
        "timestamp": utc_now_iso(),
        "path": str(path),
        "sha256": file_sha,
        "bytes": int(path.stat().st_size),
    }
    if dataset_id:
        payload["dataset_id"] = dataset_id
    if registry_entry:
        payload["registry"] = {
            k: registry_entry.get(k)
            for k in [
                "id",
                "kind",
                "source_url",
                "source_homepage",
                "retrieved_at",
                "sha256",
                "bytes",
                "license",
                "license_url",
                "tags",
                "notes",
            ]
            if k in registry_entry
        }

    try:
        if ext in {".xlsx", ".xlsm", ".xltx", ".xltm"}:
            payload["inspection"] = inspect_openxml_like(path, deep=bool(args.deep))
        elif ext in {".pbix", ".pbit"}:
            payload["inspection"] = inspect_pbix_like(path)
        else:
            payload["inspection"] = {"kind": "unknown"}
    except zipfile.BadZipFile:
        payload["inspection"] = {"kind": "not-zip", "error": "BadZipFile"}

    out_path = args.out
    if args.write:
        if not dataset_id:
            raise SystemExit("ERROR: --write requires --dataset-id")
        out_path = Path("datasets/real_world/inspections") / f"{dataset_id}.json"

    if out_path:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"Wrote: {out_path}")
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
