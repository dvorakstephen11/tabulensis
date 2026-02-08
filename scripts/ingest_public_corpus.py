#!/usr/bin/env python3
"""
Ingest public (pinned) real-world corpus files into hashed storage under corpus_public/.

This mirrors scripts/ingest_private_corpus.py but supports attaching dataset ids.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from real_world_lib import (
    DEFAULT_EXTS,
    ingest_file_to_corpus,
    iter_input_files,
    load_registry,
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Ingest public corpus files into hashed storage (corpus_public/)."
    )
    parser.add_argument("--input-dir", type=Path, required=True, help="Directory to scan")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("corpus_public"),
        help="Destination directory for hashed files",
    )
    parser.add_argument(
        "--index",
        type=Path,
        default=None,
        help="Optional corpus index path (defaults to <output-dir>/index.json)",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=None,
        help="Optional dataset registry yaml (used to infer dataset_id by sha256 pin)",
    )
    parser.add_argument(
        "--dataset-id",
        type=str,
        default="",
        help="Optional dataset id to attach to all ingested files (overrides registry inference)",
    )
    parser.add_argument(
        "--source-tag",
        type=str,
        default="",
        help="Optional source label (avoid customer identifiers)",
    )
    parser.add_argument(
        "--no-recursive",
        action="store_true",
        help="Disable recursive scanning",
    )
    parser.add_argument(
        "--extensions",
        type=str,
        default="",
        help="Comma-separated extension list to include (e.g. .xlsx,.pbix)",
    )
    args = parser.parse_args()

    input_dir = args.input_dir
    if not input_dir.exists():
        raise FileNotFoundError(f"Input dir not found: {input_dir}")

    exts = set(DEFAULT_EXTS)
    if args.extensions:
        exts = {ext.strip().lower() for ext in args.extensions.split(",") if ext.strip()}

    output_dir = args.output_dir
    index_path = args.index or (output_dir / "index.json")

    registry_sha_to_id: dict[str, str] = {}
    if args.registry:
        reg = load_registry(args.registry)
        for ds in reg.get("datasets", []):
            if not isinstance(ds, dict):
                continue
            sha = str(ds.get("sha256") or "").strip().lower()
            ds_id = str(ds.get("id") or "").strip()
            if sha and ds_id:
                registry_sha_to_id[sha] = ds_id

    dataset_id_override = args.dataset_id.strip() or None
    source_tag = args.source_tag.strip() or None

    files = iter_input_files(input_dir, not args.no_recursive, exts)
    if not files:
        print("No files found to ingest.")
        return 0

    ingested = 0
    for path in files:
        dsid = dataset_id_override
        # Registry inference only if no override.
        if not dsid and registry_sha_to_id:
            from real_world_lib import sha256_path

            digest = sha256_path(path)
            dsid = registry_sha_to_id.get(digest.lower())
        ingest_file_to_corpus(
            path,
            corpus_dir=output_dir,
            index_path=index_path,
            dataset_id=dsid,
            source_tag=source_tag,
        )
        ingested += 1

    print(f"Ingested {ingested} file(s).")
    print(f"Output: {output_dir}")
    print(f"Index: {index_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

