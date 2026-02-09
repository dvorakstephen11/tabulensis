#!/usr/bin/env python3
"""
Docs integrity checks for Tabulensis.

This is a lightweight, dependency-free helper intended to be run weekly (and
after doc edits) to catch:
- Broken local links in docs/index.md (and optionally the whole primary corpus)
- Checklist-index drift (docs/index.md auto-index block vs current scan)
- Decision-gate hotspots (Decide/Choose/TBD/TODO/If local-only/If committed)
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


RE_MD_LINK = re.compile(r"\[[^\]]+\]\((?P<href>[^)]+)\)")
RE_CHECKLIST_LINE = re.compile(r"^- \[(?P<path>[^]]+)\]\([^)]+\)\s+\(open:\s+\d+,\s+done:\s+\d+\)\s*$")
RE_DECISION = re.compile(r"\b(Decide|Choose|TBD|TODO|If local-only:|If committed:)\b")

BEGIN_MARKER = "<!-- BEGIN CHECKLIST INDEX -->"
END_MARKER = "<!-- END CHECKLIST INDEX -->"


@dataclass(frozen=True)
class LinkIssue:
    doc: str
    href: str
    resolved: str
    kind: str


@dataclass(frozen=True)
class GateHit:
    path: str
    lineno: int
    line: str


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").replace("\r", "\n")


def _is_external_href(href: str) -> bool:
    h = href.strip()
    if not h:
        return True
    if h.startswith("#"):
        return True
    if "://" in h:
        return True
    if h.startswith("mailto:"):
        return True
    return False


def iter_local_hrefs(md_text: str) -> Iterable[str]:
    for m in RE_MD_LINK.finditer(md_text):
        href = (m.group("href") or "").strip()
        if _is_external_href(href):
            continue
        # Drop in-file anchors.
        href = href.split("#", 1)[0].strip()
        if not href:
            continue
        yield href


def check_links_in_file(path: Path, *, base_dir: Path) -> list[LinkIssue]:
    issues: list[LinkIssue] = []
    text = _read_text(path)
    for href in iter_local_hrefs(text):
        resolved = (base_dir / href).resolve()
        # Guardrail: only treat missing as an issue for repo-local links.
        if not resolved.exists():
            issues.append(
                LinkIssue(
                    doc=path.relative_to(repo_root()).as_posix(),
                    href=href,
                    resolved=_safe_rel(resolved),
                    kind="missing",
                )
            )
    return issues


def _safe_rel(path: Path) -> str:
    rr = repo_root()
    try:
        return path.resolve().relative_to(rr.resolve()).as_posix()
    except Exception:
        return str(path)


def docs_index_paths(index_path: Path) -> list[Path]:
    idx_dir = index_path.parent
    text = _read_text(index_path)
    paths: list[Path] = []
    seen: set[str] = set()
    for href in iter_local_hrefs(text):
        # href already had anchors stripped.
        resolved = (idx_dir / href).resolve()
        key = str(resolved)
        if key in seen:
            continue
        seen.add(key)
        paths.append(resolved)
    return paths


def _extract_index_checklist_paths(index_path: Path) -> list[str]:
    text = _read_text(index_path)
    lines = text.splitlines()
    begin = next((i for i, ln in enumerate(lines) if ln.strip() == BEGIN_MARKER), -1)
    end = next((i for i, ln in enumerate(lines) if ln.strip() == END_MARKER), -1)
    if begin < 0 or end < 0 or end <= begin:
        return []
    out: list[str] = []
    for ln in lines[begin + 1 : end]:
        ln = ln.strip()
        if not ln:
            continue
        m = RE_CHECKLIST_LINE.match(ln)
        if not m:
            continue
        out.append(m.group("path"))
    return out


def _scan_checklists_via_tool(repo: Path) -> list[str]:
    cmd = [sys.executable, str(repo / "scripts/update_docs_index_checklists.py"), "--print"]
    out = subprocess.check_output(cmd, cwd=str(repo), text=True)
    paths: list[str] = []
    for ln in out.splitlines():
        ln = ln.strip()
        if not ln:
            continue
        m = RE_CHECKLIST_LINE.match(ln)
        if not m:
            continue
        paths.append(m.group("path"))
    return paths


def _iter_markdown_files_git(repo: Path) -> list[Path]:
    # Match the policy in update_docs_index_checklists.py:
    # - include git-tracked markdown plus untracked-but-not-ignored markdown
    try:
        tracked = subprocess.check_output(["git", "ls-files"], cwd=str(repo), text=True, stderr=subprocess.DEVNULL)
        untracked = subprocess.check_output(
            ["git", "ls-files", "--others", "--exclude-standard"],
            cwd=str(repo),
            text=True,
            stderr=subprocess.DEVNULL,
        )
        paths: list[Path] = []
        seen: set[str] = set()
        for blob in (tracked, untracked):
            for raw in blob.splitlines():
                rel = raw.strip()
                if not rel.lower().endswith(".md"):
                    continue
                if rel in seen:
                    continue
                seen.add(rel)
                p = (repo / rel).resolve()
                if p.exists():
                    paths.append(p)
        return paths
    except Exception:
        return [p for p in repo.rglob("*.md") if p.is_file()]


def decision_gates(repo: Path, *, limit: int) -> list[GateHit]:
    hits: list[GateHit] = []
    for md in _iter_markdown_files_git(repo):
        try:
            rel = md.resolve().relative_to(repo.resolve()).as_posix()
        except Exception:
            rel = str(md)
        text = _read_text(md)
        for i, ln in enumerate(text.splitlines(), start=1):
            if not RE_DECISION.search(ln):
                continue
            hits.append(GateHit(path=rel, lineno=i, line=ln.rstrip()))
            if limit > 0 and len(hits) >= limit:
                return hits
    return hits


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Docs integrity checks.")
    parser.add_argument(
        "--check-links",
        action="store_true",
        help="Also scan docs linked from docs/index.md for broken local links (slower).",
    )
    parser.add_argument(
        "--decision-gates",
        action="store_true",
        help="Print decision-gate hotspots (Decide/Choose/TBD/TODO/If committed/local-only).",
    )
    parser.add_argument(
        "--gate-limit",
        type=int,
        default=200,
        help="Max decision-gate hits to print (default 200; 0 disables the limit).",
    )
    args = parser.parse_args(argv)

    repo = repo_root()
    index_path = repo / "docs/index.md"

    # 1) docs/index.md local link existence.
    issues: list[LinkIssue] = []
    issues.extend(check_links_in_file(index_path, base_dir=index_path.parent))

    # 2) Checklist-index drift check.
    index_checklists = _extract_index_checklist_paths(index_path)
    scanned_checklists = _scan_checklists_via_tool(repo)
    drift_missing = sorted(set(scanned_checklists) - set(index_checklists))
    drift_extra = sorted(set(index_checklists) - set(scanned_checklists))

    # 3) Optional: scan linked docs for broken local links.
    if args.check_links:
        for p in docs_index_paths(index_path):
            if not p.exists() or not p.is_file():
                continue
            issues.extend(check_links_in_file(p, base_dir=p.parent))

    # 4) Optional: decision gates report.
    if args.decision_gates:
        hits = decision_gates(repo, limit=int(args.gate_limit))
        for h in hits:
            print(f"{h.path}:{h.lineno}: {h.line}")
        return 0

    # Default report.
    ok = True
    if issues:
        ok = False
        print("Broken or suspicious local links:")
        for it in issues[:200]:
            print(f"- {it.doc}: [{it.kind}] ({it.href}) -> {it.resolved}")
        if len(issues) > 200:
            print(f"- ... ({len(issues) - 200} more)")

    if drift_missing or drift_extra:
        ok = False
        print("Checklist index drift detected:")
        if drift_missing:
            print("- Present in scan but missing from docs/index.md block:")
            for p in drift_missing:
                print(f"  - {p}")
        if drift_extra:
            print("- Present in docs/index.md block but missing from scan:")
            for p in drift_extra:
                print(f"  - {p}")
        print("Fix: run `python3 scripts/update_docs_index_checklists.py`.")

    if ok:
        print("OK: docs/index links and checklist index look consistent.")
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
