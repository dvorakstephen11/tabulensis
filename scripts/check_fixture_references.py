import re
import sys
from pathlib import Path

import yaml

RE_FIXTURE_NAME = re.compile(r'"([A-Za-z0-9._-]+\.(?:xlsx|xlsm|pbix|pbit|zip|txt|bin))"')
RE_WORKFLOW_REF = re.compile(r"fixtures/generated/([A-Za-z0-9._-]+\.(?:xlsx|xlsm|pbix|pbit|zip|txt|bin))")

IGNORED_FIXTURE_NAMES = {
    "definitely_missing.xlsx",
    "missing_mashup.xlsx",
    "nonexistent_a.xlsx",
    "nonexistent_b.xlsx",
    "book.xlsx",
    "excel_diff_not_zip.txt",
    "Foo.txt",
    "Bar.txt",
    "Baz.txt",
}


def load_manifest_outputs(path: Path) -> set[str]:
    if not path.exists():
        raise FileNotFoundError(f"Manifest not found: {path}")
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    outputs: set[str] = set()
    for scenario in data.get("scenarios", []):
        out = scenario.get("output")
        if isinstance(out, list):
            outputs.update(str(name) for name in out)
        elif out:
            outputs.add(str(out))
    return outputs


def scan_test_fixtures(paths: list[Path]) -> set[str]:
    fixtures: set[str] = set()
    for path in paths:
        text = path.read_text(encoding="utf-8")
        if '#![cfg(feature = "perf-metrics")]' in text:
            continue
        for name in RE_FIXTURE_NAME.findall(text):
            if name in IGNORED_FIXTURE_NAMES:
                continue
            fixtures.add(name)
    return fixtures


def scan_workflow_fixtures(paths: list[Path]) -> set[str]:
    fixtures: set[str] = set()
    for path in paths:
        text = path.read_text(encoding="utf-8")
        for name in RE_WORKFLOW_REF.findall(text):
            fixtures.add(name)
    return fixtures


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    core_tests = repo_root / "core" / "tests"
    cli_tests = repo_root / "cli" / "tests"
    workflows = repo_root / ".github" / "workflows"

    test_files = list(core_tests.rglob("*.rs")) + list(cli_tests.rglob("*.rs"))
    workflow_files = list(workflows.rglob("*.yml")) + list(workflows.rglob("*.yaml"))

    manifest_tests = repo_root / "fixtures" / "manifest_cli_tests.yaml"
    manifest_release = repo_root / "fixtures" / "manifest_release_smoke.yaml"

    errors: list[str] = []

    try:
        test_manifest_outputs = load_manifest_outputs(manifest_tests)
    except FileNotFoundError as exc:
        errors.append(str(exc))
        test_manifest_outputs = set()

    try:
        release_manifest_outputs = load_manifest_outputs(manifest_release)
    except FileNotFoundError as exc:
        errors.append(str(exc))
        release_manifest_outputs = set()

    test_refs = scan_test_fixtures(test_files)
    missing_tests = sorted(test_refs - test_manifest_outputs)
    if missing_tests:
        errors.append(
            "Tests reference fixtures not present in fixtures/manifest_cli_tests.yaml: "
            + ", ".join(missing_tests)
        )

    workflow_refs = scan_workflow_fixtures(workflow_files)
    missing_workflows = sorted(workflow_refs - release_manifest_outputs)
    if missing_workflows:
        errors.append(
            "Workflows reference fixtures not present in fixtures/manifest_release_smoke.yaml: "
            + ", ".join(missing_workflows)
        )

    if errors:
        for error in errors:
            print(f"Error: {error}", file=sys.stderr)
        return 1

    print("Fixture reference guard passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
