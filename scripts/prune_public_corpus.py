#!/usr/bin/env python3
"""
Prune corpus_public/ to keep only blobs referenced by the real-world registry (and optionally derived outputs).

Default is dry-run. Use --apply to delete files and rewrite the corpus index.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

from real_world_lib import load_corpus_index, load_registry, save_corpus_index, load_json


RE_BLOB = re.compile(r"^sha256_([0-9a-f]{64})\.[A-Za-z0-9]+$")


def main() -> int:
    parser = argparse.ArgumentParser(description="Prune corpus_public to referenced sha256 blobs.")
    parser.add_argument(
        "--registry",
        type=Path,
        default=Path("datasets/real_world/registry.yaml"),
        help="Dataset registry yaml",
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
        "--derived-index",
        type=Path,
        default=None,
        help="Optional derived index JSON path (defaults to <corpus-dir>/derived_index.json)",
    )
    parser.add_argument(
        "--keep-derived",
        action="store_true",
        help="Keep sha256 blobs referenced by derived_index.json as well as the registry pins",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Actually delete files and rewrite index.json (default is dry-run)",
    )
    args = parser.parse_args()

    corpus_dir: Path = args.corpus_dir
    index_path: Path = args.index or (corpus_dir / "index.json")
    derived_index_path: Path = args.derived_index or (corpus_dir / "derived_index.json")

    keep: set[str] = set()
    reg = load_registry(args.registry)
    for ds in reg.get("datasets", []):
        if not isinstance(ds, dict):
            continue
        sha = str(ds.get("sha256") or "").strip().lower()
        if sha:
            keep.add(sha)

    if args.keep_derived and derived_index_path.exists():
        try:
            derived = load_json(derived_index_path)
            for entry in derived.get("derived", []) or []:
                if not isinstance(entry, dict):
                    continue
                sha = str(entry.get("sha256") or "").strip().lower()
                if sha:
                    keep.add(sha)
        except Exception as exc:
            raise RuntimeError(f"Failed to read derived index {derived_index_path}: {exc}") from exc

    if not corpus_dir.exists():
        print(f"Corpus dir does not exist: {corpus_dir}")
        return 0

    blobs: list[Path] = []
    for path in corpus_dir.iterdir():
        if not path.is_file():
            continue
        m = RE_BLOB.match(path.name)
        if m:
            blobs.append(path)

    to_delete: list[Path] = []
    for blob in blobs:
        m = RE_BLOB.match(blob.name)
        assert m is not None
        sha = m.group(1).lower()
        if sha not in keep:
            to_delete.append(blob)

    if not to_delete:
        print("Nothing to prune.")
        return 0

    print(f"Would delete {len(to_delete)} blob(s):")
    for path in sorted(to_delete):
        print(f"  - {path}")

    if not args.apply:
        print("Dry-run only. Re-run with --apply to delete.")
        return 0

    # Delete files.
    for path in to_delete:
        try:
            path.unlink()
        except Exception as exc:
            raise RuntimeError(f"Failed to delete {path}: {exc}") from exc

    # Rewrite index.json to drop deleted entries.
    index = load_corpus_index(index_path)
    kept_files = []
    for entry in index.get("files", []) or []:
        if not isinstance(entry, dict):
            continue
        sha = str(entry.get("sha256") or "").strip().lower()
        if sha and sha in keep:
            kept_files.append(entry)
    index["files"] = kept_files
    save_corpus_index(index_path, index)

    print(f"Pruned {len(to_delete)} blob(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
