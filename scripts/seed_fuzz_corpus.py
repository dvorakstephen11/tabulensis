import argparse
import base64
import hashlib
import json
import shutil
import zipfile
from pathlib import Path
from typing import Iterable, Optional
from xml.etree import ElementTree as ET

import yaml

DATA_MASHUP_NS = {"dm": "http://schemas.microsoft.com/DataMashup"}
DM_MARKERS = (
    b"DataMashup",
    b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p",
)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_config(path: Path) -> list[dict]:
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    fixtures = data.get("fixtures", [])
    if not isinstance(fixtures, list):
        raise ValueError("seed_fixtures.yaml must contain a top-level 'fixtures' list")
    return fixtures


def find_datamashup_element(root: ET.Element) -> Optional[ET.Element]:
    if root.tag.endswith("DataMashup"):
        return root
    return root.find(".//dm:DataMashup", namespaces=DATA_MASHUP_NS)


def extract_datamashup_from_xlsx(path: Path) -> bytes:
    with zipfile.ZipFile(path, "r") as zin:
        for name in zin.namelist():
            if not (name.startswith("customXml/item") and name.endswith(".xml")):
                continue
            buf = zin.read(name)
            if not any(marker in buf for marker in DM_MARKERS):
                continue
            try:
                root = ET.fromstring(buf)
            except ET.ParseError:
                continue
            node = find_datamashup_element(root)
            if node is None or node.text is None:
                continue
            text = "".join(node.text.split())
            if not text:
                continue
            return base64.b64decode(text)
    raise ValueError(f"DataMashup not found in {path}")


def extract_datamashup_from_pbix(path: Path) -> bytes:
    with zipfile.ZipFile(path, "r") as zin:
        try:
            return zin.read("DataMashup")
        except KeyError as exc:
            raise ValueError(f"DataMashup not found in {path}") from exc


def extract_datamashup(path: Path) -> bytes:
    suffix = path.suffix.lower()
    if suffix in {".xlsx", ".xlsm"}:
        return extract_datamashup_from_xlsx(path)
    if suffix in {".pbix", ".pbit"}:
        return extract_datamashup_from_pbix(path)
    raise ValueError(f"Unsupported DataMashup source extension: {path}")


def copy_fixture(src: Path, dest_dir: Path, overwrite: bool) -> Path:
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / src.name
    if dest.exists() and not overwrite:
        return dest
    shutil.copy2(src, dest)
    return dest


def write_datamashup_seed(
    dm_bytes: bytes, dest_dir: Path, stem: str, overwrite: bool
) -> Path:
    dest_dir.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256(dm_bytes).hexdigest()
    dest = dest_dir / f"{stem}_{digest[:8]}.bin"
    if dest.exists() and not overwrite:
        return dest
    dest.write_bytes(dm_bytes)
    return dest


def iter_targets(targets: Iterable[str]) -> list[str]:
    out = []
    for target in targets:
        target = str(target).strip()
        if target and target not in out:
            out.append(target)
    return out


def seed_from_fixtures(
    fixtures: list[dict],
    fixtures_dir: Path,
    corpus_dir: Path,
    overwrite: bool,
    clean: bool,
) -> list[dict]:
    if clean:
        for entry in fixtures:
            for target in iter_targets(entry.get("targets", [])):
                target_dir = corpus_dir / target
                if target_dir.exists():
                    for child in target_dir.iterdir():
                        if child.is_file():
                            child.unlink()

    report = []
    for entry in fixtures:
        filename = entry.get("file")
        if not filename:
            raise ValueError("Each fixture entry must include a 'file'")
        targets = iter_targets(entry.get("targets", []))
        if not targets:
            continue

        src = Path(filename)
        if not src.is_absolute():
            src = fixtures_dir / filename
        if not src.exists():
            raise FileNotFoundError(f"Fixture not found: {src}")

        record = {"file": filename, "targets": targets, "outputs": []}
        for target in targets:
            if target == "fuzz_datamashup_parse":
                try:
                    dm_bytes = extract_datamashup(src)
                except ValueError as exc:
                    record["outputs"].append({"target": target, "skipped": str(exc)})
                    continue
                dest = write_datamashup_seed(
                    dm_bytes, corpus_dir / target, src.stem, overwrite
                )
                record["outputs"].append({"target": target, "path": str(dest)})
            else:
                dest = copy_fixture(src, corpus_dir / target, overwrite)
                record["outputs"].append({"target": target, "path": str(dest)})
        report.append(record)
    return report


def main() -> int:
    root = repo_root()
    parser = argparse.ArgumentParser(description="Seed fuzz corpora from fixtures.")
    parser.add_argument(
        "--config",
        type=Path,
        default=root / "core" / "fuzz" / "seed_fixtures.yaml",
        help="Path to seed_fixtures.yaml",
    )
    parser.add_argument(
        "--fixtures-dir",
        type=Path,
        default=root / "fixtures" / "generated",
        help="Directory containing generated fixtures",
    )
    parser.add_argument(
        "--corpus-dir",
        type=Path,
        default=root / "core" / "fuzz" / "corpus",
        help="Destination corpus root",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Remove existing files in targeted corpus directories before seeding",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing seeds with the same name",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="Optional path to write a JSON report of seeded files",
    )
    args = parser.parse_args()

    fixtures = load_config(args.config)
    report = seed_from_fixtures(
        fixtures, args.fixtures_dir, args.corpus_dir, args.overwrite, args.clean
    )

    if args.report:
        args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")

    print(f"Seeded {len(report)} fixture entries into {args.corpus_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
