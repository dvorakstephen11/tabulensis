import argparse
import hashlib
import re
import shutil
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def slugify(text: str) -> str:
    text = text.strip().lower()
    text = re.sub(r"[^a-z0-9]+", "_", text)
    return text.strip("_") or "regression"


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def append_block(path: Path, block: str) -> None:
    content = ""
    if path.exists():
        content = path.read_text(encoding="utf-8")
    if content and not content.endswith("\n"):
        content += "\n"
    content += block
    if not content.endswith("\n"):
        content += "\n"
    path.write_text(content, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Add a regression fixture from a fuzz artifact.")
    parser.add_argument("--artifact", required=True, help="Path to minimized artifact")
    parser.add_argument(
        "--type",
        required=True,
        choices=["xlsx", "xlsm", "pbix", "pbit", "dm_bytes"],
        help="Fixture classification",
    )
    parser.add_argument("--area", required=True, help="Area name (e.g. workbook, pbix, datamashup)")
    parser.add_argument("--description", required=True, help="Short description")
    parser.add_argument(
        "--expectation",
        choices=["ok", "error"],
        help="Optional expectation to append to robustness_regressions.yaml",
    )
    parser.add_argument(
        "--error-code",
        help="Required when --expectation=error (e.g. EXDIFF_DM_003)",
    )
    args = parser.parse_args()

    root = repo_root()
    artifact = Path(args.artifact).resolve()
    if not artifact.exists():
        raise FileNotFoundError(f"Artifact not found: {artifact}")

    digest = sha256_file(artifact)[:8]
    slug = slugify(args.description)
    ext = args.type if args.type != "dm_bytes" else "bin"

    filename = f"reg_{args.area}_{slug}_{digest}.{ext}"
    template_dir = root / "fixtures" / "templates" / "regressions" / args.area
    template_dir.mkdir(parents=True, exist_ok=True)
    template_path = template_dir / filename
    shutil.copy2(artifact, template_path)

    manifest_path = root / "fixtures" / "manifest_cli_tests.yaml"
    scenario_id = f"regression_{args.area}_{slug}_{digest}"
    manifest_block = (
        f'  - id: "{scenario_id}"\n'
        f'    generator: "copy_template"\n'
        f'    args:\n'
        f'      template: "templates/regressions/{args.area}/{filename}"\n'
        f'    output: "{filename}"\n'
    )
    append_block(manifest_path, manifest_block)

    if args.expectation:
        if args.expectation == "error" and not args.error_code:
            raise ValueError("--error-code is required when --expectation=error")
        expectations_path = root / "core" / "tests" / "robustness_regressions.yaml"
        if not expectations_path.exists():
            expectations_path.write_text("fixtures:\n", encoding="utf-8")
        error_line = ""
        if args.expectation == "error":
            error_line = f'      error_code: "{args.error_code}"\n'
        expectations_block = (
            f'  - file: "{filename}"\n'
            f'    type: "{args.type}"\n'
            f'    expectation:\n'
            f'      result: "{args.expectation}"\n'
            f"{error_line}"
            f'    invariants:\n'
            f'      self_diff_empty: {str(args.expectation == "ok").lower()}\n'
            f'      deterministic_open: {str(args.expectation == "ok").lower()}\n'
        )
        append_block(expectations_path, expectations_block)

    print(f"Added template: {template_path}")
    print(f"Appended manifest scenario: {scenario_id}")
    print("Remember to regenerate fixtures and update the lock file.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
