#!/usr/bin/env python3
"""
Download pinned public datasets from datasets/real_world/registry.yaml into corpus_public/.

This script:
- downloads each dataset (streaming)
- enforces sha256 pins (and optional size)
- ingests the bytes into corpus_public/ as sha256_<digest>.<ext>
- updates corpus_public/index.json with dataset_id attribution
"""

from __future__ import annotations

import argparse
from pathlib import Path

from real_world_lib import (
    DEFAULT_EXTS,
    download_url_to_path,
    ingest_file_to_corpus,
    kind_to_ext,
    load_registry,
    load_corpus_index,
    find_corpus_entry_by_sha256,
)


def main() -> int:
    parser = argparse.ArgumentParser(description="Download pinned real-world datasets into corpus_public/.")
    parser.add_argument(
        "--registry",
        type=Path,
        default=Path("datasets/real_world/registry.yaml"),
        help="Path to the dataset registry yaml",
    )
    parser.add_argument(
        "--corpus-dir",
        type=Path,
        default=Path("corpus_public"),
        help="Corpus directory (content-addressed store)",
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
        default=Path("tmp/real_world_downloads"),
        help="Temporary download directory",
    )
    parser.add_argument(
        "--dataset-id",
        action="append",
        default=[],
        help="Restrict download to specific dataset id(s) (repeatable)",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=180,
        help="Per-download timeout seconds",
    )
    parser.add_argument(
        "--user-agent",
        type=str,
        default="tabulensis-real-world-downloader/1.0",
        help="User-Agent header for HTTP requests",
    )
    parser.add_argument(
        "--soft-max-bytes",
        type=int,
        default=200 * 1024 * 1024,
        help="Soft per-dataset size cap (bytes); emits a warning when exceeded",
    )
    parser.add_argument(
        "--hard-max-bytes",
        type=int,
        default=1024 * 1024 * 1024,
        help="Hard per-dataset size cap (bytes); download aborts when exceeded",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Re-download even if the sha256 blob already exists in the corpus",
    )
    args = parser.parse_args()

    reg = load_registry(args.registry)
    datasets = reg.get("datasets", [])
    if not datasets:
        print(f"No datasets defined in {args.registry}. Nothing to download.")
        return 0

    corpus_dir: Path = args.corpus_dir
    index_path: Path = args.index or (corpus_dir / "index.json")
    tmp_dir: Path = args.tmp_dir
    tmp_dir.mkdir(parents=True, exist_ok=True)

    selected_ids = {d.strip() for d in args.dataset_id if d.strip()}

    index = load_corpus_index(index_path)

    downloaded = 0
    skipped = 0
    for ds in datasets:
        if not isinstance(ds, dict):
            continue
        ds_id = str(ds.get("id") or "").strip()
        if not ds_id:
            continue
        if selected_ids and ds_id not in selected_ids:
            continue

        kind = str(ds.get("kind") or "").strip()
        source_url = str(ds.get("source_url") or "").strip()
        expected_sha256 = str(ds.get("sha256") or "").strip().lower()
        expected_bytes = ds.get("bytes")

        if not kind or not source_url or not expected_sha256:
            raise ValueError(f"Dataset {ds_id} missing required fields (kind/source_url/sha256)")

        ext = kind_to_ext(kind)
        blob_name = f"sha256_{expected_sha256}.{ext.lstrip('.')}"
        blob_path = corpus_dir / blob_name

        if blob_path.exists() and not args.force:
            # Ensure index contains dataset_id mapping.
            entry = find_corpus_entry_by_sha256(index, expected_sha256)
            if entry and not entry.get("dataset_id"):
                entry["dataset_id"] = ds_id
            if not entry:
                index.setdefault("files", []).append(
                    {
                        "sha256": expected_sha256,
                        "size_bytes": int(blob_path.stat().st_size),
                        "extension": ext,
                        "filename": blob_name,
                        "dataset_id": ds_id,
                        "ingested_at": "unknown",
                    }
                )
            from real_world_lib import save_corpus_index

            save_corpus_index(index_path, index)
            skipped += 1
            continue

        tmp_path = tmp_dir / f"{ds_id}{ext}"
        result = download_url_to_path(
            source_url,
            tmp_path,
            timeout_seconds=args.timeout_seconds,
            max_bytes=args.hard_max_bytes,
            expected_sha256=expected_sha256,
            user_agent=args.user_agent,
        )

        if result.bytes > args.soft_max_bytes:
            print(
                f"WARNING: Dataset {ds_id} is {result.bytes} bytes (soft cap {args.soft_max_bytes})"
            )

        if expected_bytes is not None:
            try:
                expected_bytes_i = int(expected_bytes)
            except Exception:
                expected_bytes_i = None
            if expected_bytes_i is not None and expected_bytes_i != result.bytes:
                raise ValueError(
                    f"Size mismatch for {ds_id}: registry bytes={expected_bytes_i}, downloaded={result.bytes}"
                )

        ingest_file_to_corpus(
            result.path,
            corpus_dir=corpus_dir,
            index_path=index_path,
            dataset_id=ds_id,
            source_tag=str(ds.get("source_homepage") or "").strip() or None,
        )
        downloaded += 1

    print(f"Downloaded {downloaded} dataset(s), skipped {skipped}.")
    print(f"Corpus: {corpus_dir}")
    print(f"Index: {index_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

