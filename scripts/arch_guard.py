from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]

PARSE_FILES = [
    "core/src/excel_open_xml.rs",
    "core/src/grid_parser.rs",
    "core/src/datamashup_framing.rs",
]

DIFF_FILES = [
    "core/src/diff.rs",
    "core/src/object_diff.rs",
    "core/src/m_diff.rs",
    "core/src/formula_diff.rs",
]
DIFF_GLOBS = [
    "core/src/engine/*.rs",
    "core/src/m_ast_diff/*.rs",
]

PARSE_FORBIDDEN = [
    "crate::diff",
    "crate::engine",
    "crate::package",
]

DIFF_FORBIDDEN = [
    "crate::excel_open_xml",
    "crate::grid_parser",
    "crate::container",
    "crate::datamashup_framing",
]


def expand_globs(globs: list[str]) -> list[str]:
    files: list[str] = []
    for pattern in globs:
        files.extend(str(p.relative_to(ROOT)) for p in ROOT.glob(pattern))
    return sorted(set(files))


def scan_files(files: list[str], forbidden: list[str], label: str) -> list[str]:
    violations: list[str] = []
    for rel in files:
        path = ROOT / rel
        if not path.exists():
            violations.append(f"{label}: missing {rel}")
            continue
        text = path.read_text(encoding="utf-8")
        for token in forbidden:
            if token in text:
                violations.append(f"{label}: {rel} contains {token}")
    return violations


def main() -> int:
    diff_files = DIFF_FILES + expand_globs(DIFF_GLOBS)
    violations = []
    violations.extend(scan_files(PARSE_FILES, PARSE_FORBIDDEN, "parse"))
    violations.extend(scan_files(diff_files, DIFF_FORBIDDEN, "diff"))

    if violations:
        print("Architecture guard violations:")
        for entry in violations:
            print(f"- {entry}")
        return 1

    print("Architecture guard: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
