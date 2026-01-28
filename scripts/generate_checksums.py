#!/usr/bin/env python3
"""
Generate SHA256SUMS for release artifacts in target/dist.
"""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate SHA256SUMS")
    parser.add_argument("--dir", default="target/dist", help="Artifacts directory")
    parser.add_argument("--out", default="SHA256SUMS", help="Output filename")
    args = parser.parse_args()

    root = Path(args.dir)
    if not root.exists():
        print(f"Directory not found: {root}")
        return 2

    lines = []
    for path in sorted(root.iterdir()):
        if path.is_file():
            if path.name == args.out:
                continue
            digest = sha256_file(path)
            lines.append(f"{digest}  {path.name}")

    if not lines:
        print("No files found to checksum.")
        return 1

    out_path = root / args.out
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Wrote {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
