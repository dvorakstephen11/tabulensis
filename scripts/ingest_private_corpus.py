import argparse
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path


DEFAULT_EXTS = {".xlsx", ".xlsm", ".pbix", ".pbit"}


def iter_inputs(root: Path, recursive: bool, exts: set[str]) -> list[Path]:
    pattern = "**/*" if recursive else "*"
    files = []
    for path in root.glob(pattern):
        if path.is_file() and path.suffix.lower() in exts:
            files.append(path)
    return files


def sha256_bytes(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_index(path: Path) -> dict:
    if not path.exists():
        return {"version": 1, "files": []}
    return json.loads(path.read_text(encoding="utf-8"))


def save_index(path: Path, index: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Ingest private corpus files into hashed storage.")
    parser.add_argument("--input-dir", type=Path, required=True, help="Directory to scan")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("corpus_private"),
        help="Destination directory for hashed files",
    )
    parser.add_argument(
        "--index",
        type=Path,
        default=None,
        help="Optional metadata index path (defaults to <output-dir>/index.json)",
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

    exts = DEFAULT_EXTS
    if args.extensions:
        exts = {ext.strip().lower() for ext in args.extensions.split(",") if ext.strip()}

    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    index_path = args.index or (output_dir / "index.json")
    index = load_index(index_path)
    existing = {entry["sha256"] for entry in index.get("files", [])}

    ingested = 0
    skipped = 0
    now = datetime.now(timezone.utc).isoformat()

    for path in iter_inputs(input_dir, not args.no_recursive, exts):
        digest = sha256_bytes(path)
        ext = path.suffix.lower()
        dest_name = f"sha256_{digest}.{ext.lstrip('.')}"
        dest_path = output_dir / dest_name

        if digest in existing:
            skipped += 1
            continue

        shutil.copy2(path, dest_path)
        entry = {
            "sha256": digest,
            "size_bytes": path.stat().st_size,
            "extension": ext,
            "ingested_at": now,
        }
        if args.source_tag:
            entry["source_tag"] = args.source_tag
        index.setdefault("files", []).append(entry)
        existing.add(digest)
        ingested += 1

    save_index(index_path, index)

    print(f"Ingested {ingested} file(s), skipped {skipped}.")
    print(f"Output: {output_dir}")
    print(f"Index: {index_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
