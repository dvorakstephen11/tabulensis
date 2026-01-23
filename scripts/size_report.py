#!/usr/bin/env python3
"""
Generate a size report for a single artifact.

Usage:
  python scripts/size_report.py --label cli --path target/release/tabulensis --zip --out target/size_reports/cli.json
"""

import argparse
import json
import pathlib
import sys
import zipfile


def file_size(path: pathlib.Path) -> int:
    return path.stat().st_size


def zip_size(input_path: pathlib.Path, out_zip: pathlib.Path) -> int:
    if out_zip.exists():
        out_zip.unlink()
    with zipfile.ZipFile(
        out_zip, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as zf:
        zf.write(input_path, arcname=input_path.name)
    return out_zip.stat().st_size


def main() -> int:
    ap = argparse.ArgumentParser(description="Measure artifact size(s) and emit JSON")
    ap.add_argument("--label", required=True, help="Logical label for the artifact")
    ap.add_argument("--path", required=True, help="Path to the artifact on disk")
    ap.add_argument("--zip", action="store_true", help="Also report a zip-compressed size")
    ap.add_argument("--out", default=None, help="Optional output JSON path")
    args = ap.parse_args()

    p = pathlib.Path(args.path)
    if not p.exists():
        print(f"missing: {p}", file=sys.stderr)
        return 2

    raw = file_size(p)
    result = {"label": args.label, "path": str(p), "raw_bytes": raw}

    if args.zip:
        out_zip = pathlib.Path("target") / "size_artifacts" / f"{args.label}.zip"
        out_zip.parent.mkdir(parents=True, exist_ok=True)
        result["zip_bytes"] = zip_size(p, out_zip)
        result["zip_path"] = str(out_zip)

    payload = json.dumps(result, indent=2)
    print(payload)

    if args.out:
        outp = pathlib.Path(args.out)
        outp.parent.mkdir(parents=True, exist_ok=True)
        outp.write_text(payload + "\n", encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
