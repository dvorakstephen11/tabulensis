#!/usr/bin/env python3
"""
Refresh the auto-generated checklist index in docs/index.md.

This scans the repo's Markdown files (git-tracked plus untracked-but-not-ignored)
for checkbox items like:
  - [ ] ...
  - [x] ...
  - [] ...

Then it updates the section between:
  <!-- BEGIN CHECKLIST INDEX -->
  <!-- END CHECKLIST INDEX -->

Run from repo root:
  python3 scripts/update_docs_index_checklists.py

Rationale:
- The checklist index is committed. Scanning git-tracked files (plus untracked
  files that are not ignored) keeps the output deterministic in CI and avoids
  local-only scratch files accidentally polluting docs/index.md (as long as
  scratch files are properly gitignored).
"""

from __future__ import annotations

import argparse
import os
import re
import sys
import subprocess
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote


RE_CHECKBOX = re.compile(r"^\s*[-*]\s+\[(?P<mark>[ xX]?)\](?:\s+.*)?$")

EXCLUDED_DIR_NAMES = {
    ".codex_skills",
    ".git",
    "__pycache__",
    "node_modules",
    "target",
    "tmp",
    "vendor",
}

BEGIN_MARKER = "<!-- BEGIN CHECKLIST INDEX -->"
END_MARKER = "<!-- END CHECKLIST INDEX -->"


@dataclass(frozen=True)
class ChecklistStats:
    path: Path
    unchecked: int
    checked: int

    @property
    def total(self) -> int:
        return self.unchecked + self.checked


def _is_excluded(path: Path) -> bool:
    return any(part in EXCLUDED_DIR_NAMES for part in path.parts)


def iter_markdown_files(repo_root: Path) -> list[Path]:
    """
    Return absolute Paths for markdown files to scan.

    Default: scan git-tracked markdown files plus untracked-but-not-ignored markdown files.
    This respects `.gitignore` (so local-only ignored files do not pollute the index) while
    still picking up newly created checklists before the first commit.
    Fallback: filesystem scan when git is unavailable.
    """
    try:
        tracked = subprocess.check_output(
            ["git", "ls-files"],
            cwd=str(repo_root),
            stderr=subprocess.DEVNULL,
            text=True,
        )
        untracked = subprocess.check_output(
            ["git", "ls-files", "--others", "--exclude-standard"],
            cwd=str(repo_root),
            stderr=subprocess.DEVNULL,
            text=True,
        )

        paths: list[Path] = []
        seen: set[str] = set()
        for out_raw in (tracked, untracked):
            for line in out_raw.splitlines():
                line = line.strip()
                if not line:
                    continue
                if not line.lower().endswith(".md"):
                    continue
                if line in seen:
                    continue
                seen.add(line)
                p = (repo_root / line).resolve()
                if not p.exists():
                    continue
                if _is_excluded(p):
                    continue
                paths.append(p)
        return paths
    except Exception:
        out: list[Path] = []
        for p in repo_root.rglob("*.md"):
            if _is_excluded(p):
                continue
            out.append(p)
        return out


def count_checkboxes(md_path: Path) -> ChecklistStats:
    unchecked = 0
    checked = 0
    try:
        text = md_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ChecklistStats(md_path, unchecked=0, checked=0)

    for line in text.splitlines():
        m = RE_CHECKBOX.match(line)
        if not m:
            continue
        mark = (m.group("mark") or "").strip().lower()
        if mark == "x":
            checked += 1
        else:
            unchecked += 1

    return ChecklistStats(md_path, unchecked=unchecked, checked=checked)


def checklist_entries(repo_root: Path) -> list[ChecklistStats]:
    entries: list[ChecklistStats] = []
    for md in iter_markdown_files(repo_root):
        stats = count_checkboxes(md)
        if stats.total == 0:
            continue
        entries.append(stats)
    entries.sort(key=lambda e: e.path.as_posix())
    return entries


def _rel_href(from_dir: Path, to_path: Path) -> str:
    rel = Path(os.path.relpath(to_path, start=from_dir))
    return quote(rel.as_posix(), safe="/-_.~")


def render_markdown(entries: list[ChecklistStats], *, index_dir: Path, repo_root: Path) -> list[str]:
    lines: list[str] = []
    for e in entries:
        repo_rel = e.path.relative_to(repo_root).as_posix()
        href = _rel_href(index_dir, e.path)
        lines.append(f"- [{repo_rel}]({href}) (open: {e.unchecked}, done: {e.checked})\n")
    return lines


def update_index(index_path: Path, entries: list[ChecklistStats], *, repo_root: Path, dry_run: bool) -> bool:
    text = index_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines(keepends=True)

    begin_idx = next((i for i, line in enumerate(lines) if line.strip() == BEGIN_MARKER), None)
    end_idx = next((i for i, line in enumerate(lines) if line.strip() == END_MARKER), None)
    if begin_idx is None or end_idx is None or end_idx <= begin_idx:
        raise RuntimeError(f"Could not find checklist markers in {index_path}")

    index_dir = index_path.parent
    rendered = render_markdown(entries, index_dir=index_dir, repo_root=repo_root)

    new_lines = lines[: begin_idx + 1]
    if new_lines and not new_lines[-1].endswith("\n"):
        new_lines[-1] = new_lines[-1] + "\n"
    new_lines.append("\n")
    new_lines.extend(rendered)
    new_lines.append("\n")
    new_lines.extend(lines[end_idx:])

    new_text = "".join(new_lines)
    changed = new_text != text
    if changed and not dry_run:
        index_path.write_text(new_text, encoding="utf-8", newline="\n")
    return changed


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true", help="Do not write files, only report changes.")
    parser.add_argument(
        "--print",
        action="store_true",
        help="Print the generated checklist block to stdout (no file updates).",
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent
    index_path = repo_root / "docs" / "index.md"
    entries = checklist_entries(repo_root)

    if args.print:
        index_dir = index_path.parent
        sys.stdout.writelines(render_markdown(entries, index_dir=index_dir, repo_root=repo_root))
        return 0

    changed = update_index(index_path, entries, repo_root=repo_root, dry_run=args.dry_run)
    if args.dry_run:
        print("docs/index.md would change" if changed else "docs/index.md is up to date")
    else:
        print("Updated docs/index.md" if changed else "docs/index.md already up to date")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
