import argparse
import hashlib
import json
import shutil
import sys
import zipfile
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple
from xml.etree import ElementTree as ET

import yaml

# Import generators
try:
    from .generators.corrupt import ContainerCorruptGenerator
    from .generators.database import KeyedTableGenerator
    from .generators.grid import (
        AddressSanityGenerator,
        BasicGridGenerator,
        ColumnAlignmentG9Generator,
        ColumnMoveG12Generator,
        EdgeCaseGenerator,
        GridTailDiffGenerator,
        MultiCellDiffGenerator,
        Pg6SheetScenarioGenerator,
        RectBlockMoveG12Generator,
        RowAlignmentG10Generator,
        RowAlignmentG8Generator,
        RowBlockMoveG11Generator,
        RowFuzzyMoveG13Generator,
        SheetCaseRenameGenerator,
        SingleCellDiffGenerator,
        SparseGridGenerator,
        ValueFormulaGenerator,
    )
    from .generators.mashup import (
        MashupCorruptGenerator,
        MashupDuplicateGenerator,
        MashupEncodeGenerator,
        MashupInjectGenerator,
        MashupMultiEmbeddedGenerator,
        MashupOneQueryGenerator,
        MashupPermissionsMetadataGenerator,
    )
    from .generators.pbix import PbixGenerator
    from .generators.objects import ChartsGenerator, CopyTemplateGenerator, NamedRangesGenerator
    from .generators.perf import LargeGridGenerator
except ImportError:
    from generators.corrupt import ContainerCorruptGenerator
    from generators.database import KeyedTableGenerator
    from generators.grid import (
        AddressSanityGenerator,
        BasicGridGenerator,
        ColumnAlignmentG9Generator,
        ColumnMoveG12Generator,
        EdgeCaseGenerator,
        GridTailDiffGenerator,
        MultiCellDiffGenerator,
        Pg6SheetScenarioGenerator,
        RectBlockMoveG12Generator,
        RowAlignmentG10Generator,
        RowAlignmentG8Generator,
        RowBlockMoveG11Generator,
        RowFuzzyMoveG13Generator,
        SheetCaseRenameGenerator,
        SingleCellDiffGenerator,
        SparseGridGenerator,
        ValueFormulaGenerator,
    )
    from generators.mashup import (
        MashupCorruptGenerator,
        MashupDuplicateGenerator,
        MashupEncodeGenerator,
        MashupInjectGenerator,
        MashupMultiEmbeddedGenerator,
        MashupOneQueryGenerator,
        MashupPermissionsMetadataGenerator,
    )
    from generators.pbix import PbixGenerator
    from generators.objects import ChartsGenerator, CopyTemplateGenerator, NamedRangesGenerator
    from generators.perf import LargeGridGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "multi_cell_diff": MultiCellDiffGenerator,
    "grid_tail_diff": GridTailDiffGenerator,
    "row_alignment_g8": RowAlignmentG8Generator,
    "row_alignment_g10": RowAlignmentG10Generator,
    "row_block_move_g11": RowBlockMoveG11Generator,
    "row_fuzzy_move_g13": RowFuzzyMoveG13Generator,
    "column_move_g12": ColumnMoveG12Generator,
    "rect_block_move_g12": RectBlockMoveG12Generator,
    "column_alignment_g9": ColumnAlignmentG9Generator,
    "sheet_case_rename": SheetCaseRenameGenerator,
    "pg6_sheet_scenario": Pg6SheetScenarioGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "mashup:one_query": MashupOneQueryGenerator,
    "mashup:multi_query_with_embedded": MashupMultiEmbeddedGenerator,
    "mashup:permissions_metadata": MashupPermissionsMetadataGenerator,
    "pbix": PbixGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
    "named_ranges": NamedRangesGenerator,
    "charts": ChartsGenerator,
    "copy_template": CopyTemplateGenerator,
}

FILE_ARG_KEYS = ("template", "base_file", "model_schema_file")
ZIP_EXTENSIONS = {".xlsx", ".xlsm", ".pbix", ".pbit", ".zip"}


def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}", file=sys.stderr)
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}", file=sys.stderr)
            sys.exit(1)


def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)


def clean_output_dir(output_dir: Path):
    if not output_dir.exists():
        return
    resolved = output_dir.resolve()
    if resolved == Path(resolved.anchor):
        raise RuntimeError(f"Refusing to clean root directory: {resolved}")
    for child in output_dir.iterdir():
        if child.is_dir():
            shutil.rmtree(child)
        else:
            child.unlink()


def list_output_names(outputs: Any) -> List[str]:
    if isinstance(outputs, list):
        return [str(name) for name in outputs]
    if isinstance(outputs, str):
        return [outputs]
    return []


def scenario_label(scenario: Dict[str, Any], idx: int) -> str:
    return scenario.get("id") or f"index {idx}"


def resolve_fixture_path(path_value: str, fixtures_root: Path) -> Optional[Path]:
    candidate = Path(path_value)
    if candidate.exists():
        return candidate
    fallback = fixtures_root / path_value
    if fallback.exists():
        return fallback
    return None


def generated_dependency(path_value: str) -> Optional[Path]:
    parts = Path(path_value).parts
    if "generated" not in parts:
        return None
    idx = parts.index("generated")
    if idx + 1 >= len(parts):
        return None
    return Path(*parts[idx + 1 :])


def preflight_manifest(
    manifest: Dict[str, Any],
    output_dir: Path,
    fixtures_root: Path,
) -> List[str]:
    errors: List[str] = []
    scenarios = manifest.get("scenarios", [])
    output_to_info: Dict[str, str] = {}
    output_to_index: Dict[str, int] = {}

    for idx, scenario in enumerate(scenarios):
        label = scenario_label(scenario, idx)
        generator_name = scenario.get("generator")
        outputs = list_output_names(scenario.get("output"))

        if not scenario.get("id") or not generator_name or not outputs:
            errors.append(f"Scenario {label} is missing id, generator, or output.")
            continue

        if generator_name not in GENERATORS:
            errors.append(f"Scenario {label} uses unknown generator '{generator_name}'.")

        for name in outputs:
            if name in output_to_info:
                prev = output_to_info[name]
                errors.append(
                    f"Output '{name}' is duplicated in scenarios {prev} and {label}."
                )
            else:
                output_to_info[name] = label
                output_to_index[name] = idx

    for idx, scenario in enumerate(scenarios):
        label = scenario_label(scenario, idx)
        args = scenario.get("args", {}) or {}

        for key in FILE_ARG_KEYS:
            value = args.get(key)
            if not value:
                continue
            if not isinstance(value, str):
                errors.append(f"Scenario {label} arg '{key}' must be a string.")
                continue

            if key == "base_file":
                dep = generated_dependency(value)
                if dep is not None:
                    dep_name = dep.as_posix()
                    if dep_name in output_to_index:
                        if output_to_index[dep_name] >= idx:
                            errors.append(
                                f"Scenario {label} depends on generated/{dep_name} "
                                "but it is not produced earlier in the manifest."
                            )
                    elif not (output_dir / dep).exists():
                        errors.append(
                            f"Scenario {label} depends on generated/{dep_name} "
                            "but it is not produced by this manifest or present in the output dir."
                        )
                else:
                    if resolve_fixture_path(value, fixtures_root) is None:
                        errors.append(
                            f"Scenario {label} arg '{key}' file '{value}' not found."
                        )
            else:
                if resolve_fixture_path(value, fixtures_root) is None:
                    errors.append(
                        f"Scenario {label} arg '{key}' file '{value}' not found."
                    )

    return errors


def normalize_core_xml(data: bytes) -> bytes:
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        return data

    for elem in root.iter():
        tag = elem.tag
        if tag.endswith("created") or tag.endswith("modified"):
            elem.text = "1970-01-01T00:00:00Z"

    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def hash_zip_contents(path: Path) -> str:
    hasher = hashlib.sha256()
    with zipfile.ZipFile(path, "r") as zin:
        entries = [info for info in zin.infolist() if not info.is_dir()]
        entries.sort(key=lambda info: info.filename)
        for info in entries:
            data = zin.read(info.filename)
            if info.filename == "docProps/core.xml":
                data = normalize_core_xml(data)
            entry_hash = hashlib.sha256(data).hexdigest()
            hasher.update(info.filename.encode("utf-8"))
            hasher.update(b"\0")
            hasher.update(entry_hash.encode("utf-8"))
            hasher.update(b"\n")
    return hasher.hexdigest()


def compute_checksum(path: Path) -> Tuple[str, str]:
    ext = path.suffix.lower()
    if ext in ZIP_EXTENSIONS:
        digest = hash_zip_contents(path)
        return digest, "zip-entries-v1"
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return digest, "raw"


def collect_outputs_with_meta(
    manifest: Dict[str, Any],
) -> Dict[str, Dict[str, Any]]:
    output_map: Dict[str, Dict[str, Any]] = {}
    for idx, scenario in enumerate(manifest.get("scenarios", [])):
        outputs = list_output_names(scenario.get("output"))
        for name in outputs:
            output_map[name] = {
                "id": scenario_label(scenario, idx),
                "generator": scenario.get("generator"),
                "args": scenario.get("args", {}) or {},
            }
    return output_map


def verify_outputs(
    manifest: Dict[str, Any],
    output_dir: Path,
) -> List[str]:
    errors: List[str] = []
    output_map = collect_outputs_with_meta(manifest)

    for name, meta in output_map.items():
        path = output_dir / name
        if not path.exists():
            errors.append(f"Missing output: {name}")
            continue

        ext = path.suffix.lower()
        if ext in ZIP_EXTENSIONS:
            try:
                with zipfile.ZipFile(path, "r") as zin:
                    if ext in (".xlsx", ".xlsm"):
                        requires_content_types = True
                        if (
                            meta.get("generator") == "corrupt_container"
                            and meta.get("args", {}).get("mode") == "no_content_types"
                        ):
                            requires_content_types = False
                        if requires_content_types and "[Content_Types].xml" not in zin.namelist():
                            errors.append(
                                f"{name} is missing [Content_Types].xml"
                            )
            except zipfile.BadZipFile:
                errors.append(f"{name} is not a valid ZIP container")

    return errors


def write_lock_file(
    manifest_path: Path,
    manifest: Dict[str, Any],
    output_dir: Path,
    lock_path: Path,
) -> List[str]:
    errors: List[str] = []
    output_map = collect_outputs_with_meta(manifest)
    checksums: Dict[str, Dict[str, str]] = {}

    for name in sorted(output_map.keys()):
        path = output_dir / name
        if not path.exists():
            errors.append(f"Missing output: {name}")
            continue
        try:
            digest, mode = compute_checksum(path)
        except zipfile.BadZipFile:
            errors.append(f"{name} is not a valid ZIP container")
            continue
        checksums[name] = {
            "hash": f"sha256:{digest}",
            "mode": mode,
        }

    if errors:
        return errors

    payload = {
        "version": 1,
        "manifest": str(manifest_path).replace("\\", "/"),
        "output_dir": str(output_dir).replace("\\", "/"),
        "algorithm": "sha256",
        "files": checksums,
    }

    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return errors


def verify_lock_file(
    manifest_path: Path,
    manifest: Dict[str, Any],
    output_dir: Path,
    lock_path: Path,
) -> List[str]:
    errors: List[str] = []
    if not lock_path.exists():
        return [f"Lock file not found: {lock_path}"]

    try:
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return [f"Failed to parse lock file {lock_path}: {e}"]

    expected_outputs = set(collect_outputs_with_meta(manifest).keys())
    lock_files = lock.get("files", {})
    if not isinstance(lock_files, dict):
        return [f"Lock file {lock_path} has invalid 'files' section"]

    if lock.get("manifest") and lock.get("manifest") != str(manifest_path).replace("\\", "/"):
        errors.append(
            f"Lock file manifest mismatch: {lock.get('manifest')} != {manifest_path}"
        )

    missing_in_lock = expected_outputs - set(lock_files.keys())
    extra_in_lock = set(lock_files.keys()) - expected_outputs

    if missing_in_lock:
        errors.append(
            "Lock file is missing entries for: " + ", ".join(sorted(missing_in_lock))
        )
    if extra_in_lock:
        errors.append(
            "Lock file has extra entries not in manifest: "
            + ", ".join(sorted(extra_in_lock))
        )

    for name in sorted(expected_outputs):
        path = output_dir / name
        if not path.exists():
            errors.append(f"Missing output: {name}")
            continue
        try:
            digest, mode = compute_checksum(path)
        except zipfile.BadZipFile:
            errors.append(f"{name} is not a valid ZIP container")
            continue

        expected = lock_files.get(name, {})
        expected_hash = expected.get("hash")
        expected_mode = expected.get("mode")
        actual_hash = f"sha256:{digest}"

        if expected_hash != actual_hash:
            errors.append(
                f"Checksum mismatch for {name}: expected {expected_hash}, got {actual_hash}"
            )
        if expected_mode and expected_mode != mode:
            errors.append(
                f"Checksum mode mismatch for {name}: expected {expected_mode}, got {mode}"
            )

    return errors


def generate_fixtures(
    manifest: Dict[str, Any],
    output_dir: Path,
    force: bool,
) -> List[str]:
    errors: List[str] = []
    scenarios = manifest.get("scenarios", [])
    print(f"Found {len(scenarios)} scenarios in manifest.")

    for idx, scenario in enumerate(scenarios):
        label = scenario_label(scenario, idx)
        scenario_id = scenario.get("id")
        generator_name = scenario.get("generator")
        generator_args = scenario.get("args", {})
        outputs = scenario.get("output")
        output_names = list_output_names(outputs)

        if not scenario_id or not generator_name or not output_names:
            errors.append(f"Scenario {label} is missing id, generator, or output.")
            continue

        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")

        if generator_name not in GENERATORS:
            errors.append(f"Scenario {scenario_id}: unknown generator '{generator_name}'.")
            continue

        output_paths = [output_dir / name for name in output_names]
        existing = [path for path in output_paths if path.exists()]
        if existing and not force:
            names = ", ".join(path.name for path in existing)
            errors.append(
                f"Scenario {scenario_id} would overwrite existing outputs: {names}"
            )
            continue
        if force:
            for path in existing:
                if path.is_dir():
                    shutil.rmtree(path)
                else:
                    path.unlink()

        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            errors.append(f"Scenario {scenario_id} failed: {e}")
            import traceback
            traceback.print_exc()

    return errors


def report_errors(errors: List[str]):
    for error in errors:
        print(f"Error: {error}", file=sys.stderr)


def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent

    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=default_manifest,
        help="Path to the manifest YAML file.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=default_output,
        help="Directory to output generated files.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Overwrite existing outputs.",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Delete existing outputs in the output directory before generating.",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify expected outputs exist and are structurally valid.",
    )
    parser.add_argument(
        "--write-lock",
        type=Path,
        help="Write a checksum lock file after generation.",
    )
    parser.add_argument(
        "--verify-lock",
        type=Path,
        help="Verify outputs against a checksum lock file.",
    )
    parser.add_argument(
        "--continue-on-error",
        action="store_true",
        help="Continue and exit 0 even if some scenarios fail.",
    )

    args = parser.parse_args()

    manifest = load_manifest(args.manifest)

    if args.clean:
        clean_output_dir(args.output_dir)

    preflight_errors = preflight_manifest(manifest, args.output_dir, fixtures_root)
    if preflight_errors:
        report_errors(preflight_errors)
        sys.exit(1)

    generate = True
    if args.verify or args.verify_lock:
        generate = False
    if args.write_lock:
        generate = True

    errors: List[str] = []

    if generate:
        ensure_output_dir(args.output_dir)
        errors.extend(generate_fixtures(manifest, args.output_dir, args.force))
        if errors and not args.continue_on_error:
            report_errors(errors)
            sys.exit(1)

    if args.verify:
        errors.extend(verify_outputs(manifest, args.output_dir))

    if args.verify_lock:
        errors.extend(
            verify_lock_file(
                args.manifest, manifest, args.output_dir, args.verify_lock
            )
        )

    if args.write_lock:
        errors.extend(
            write_lock_file(
                args.manifest, manifest, args.output_dir, args.write_lock
            )
        )

    if errors:
        report_errors(errors)
        if not args.continue_on_error:
            sys.exit(1)


if __name__ == "__main__":
    main()
