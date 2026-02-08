#!/usr/bin/env python3
"""
Lightweight PII-ish string scan for candidate datasets.

This is intentionally heuristic and conservative:
- It is NOT a guarantee of no sensitive data.
- It is designed to catch obvious patterns (emails, SSNs, phone numbers).

Policy (recommended):
- RW1 (committed) datasets must pass (or be rejected).
- RW2 (cached public) datasets should pass; otherwise treat like RW3 (private).
"""

from __future__ import annotations

import argparse
import json
import re
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from real_world_lib import resolve_dataset_path, sha256_path


EMAIL_RE = re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
SSN_RE = re.compile(r"\b\d{3}-\d{2}-\d{4}\b")
PHONE_RE = re.compile(
    r"\b(?:\+?1[-.\s]?)?(?:\(\d{3}\)|\d{3})[-.\s]?\d{3}[-.\s]?\d{4}\b"
)


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def redact(s: str, keep_start: int = 3, keep_end: int = 3) -> str:
    if len(s) <= keep_start + keep_end:
        return "*" * len(s)
    return s[:keep_start] + "*" * (len(s) - keep_start - keep_end) + s[-keep_end:]


def email_domain(s: str) -> str | None:
    parts = s.rsplit("@", 1)
    if len(parts) != 2:
        return None
    domain = parts[1].strip().lower()
    # Strip trailing punctuation sometimes present in XML/text.
    domain = domain.rstrip(").,;:<>\"'")
    return domain or None


def scan_text(
    text: str,
    *,
    max_matches: int,
    allow_email_domains: set[str],
) -> list[dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    for kind, rx in [("email", EMAIL_RE), ("ssn", SSN_RE), ("phone", PHONE_RE)]:
        for m in rx.finditer(text):
            raw = m.group(0)
            allowed = False
            if kind == "email" and allow_email_domains:
                dom = email_domain(raw)
                if dom and dom in allow_email_domains:
                    allowed = True
            findings.append({"kind": kind, "match": redact(raw), "allowed": allowed})
            if len(findings) >= max_matches:
                return findings
    return findings


def scan_bytes_stream(
    handle,
    *,
    max_bytes: int,
    max_matches: int,
    allow_email_domains: set[str],
) -> tuple[int, list[dict[str, Any]]]:
    # Decode as utf-8 with errors ignored; scan incrementally.
    scanned = 0
    findings: list[dict[str, Any]] = []
    tail = ""

    while True:
        chunk = handle.read(1024 * 1024)
        if not chunk:
            break
        scanned += len(chunk)
        if scanned > max_bytes:
            break
        text = chunk.decode("utf-8", errors="ignore")
        text = tail + text
        tail = text[-512:]  # keep a small overlap for boundary matches
        new_findings = scan_text(
            text,
            max_matches=max_matches - len(findings),
            allow_email_domains=allow_email_domains,
        )
        findings.extend(new_findings)
        if len(findings) >= max_matches:
            break

    return scanned, findings


def scan_zip(
    path: Path,
    *,
    max_bytes_per_entry: int,
    max_matches: int,
    allow_email_domains: set[str],
) -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    total_scanned = 0

    with zipfile.ZipFile(path, "r") as zf:
        names = zf.namelist()

        # Prefer likely-text entries first.
        priority = []
        for n in names:
            nl = n.lower()
            if nl.endswith("xl/sharedstrings.xml"):
                priority.append(n)
            elif nl.startswith("xl/worksheets/") and nl.endswith(".xml"):
                priority.append(n)
            elif nl.endswith(".xml") or nl.endswith(".json") or nl.endswith(".txt") or nl.endswith(".m"):
                priority.append(n)

        # De-dup while keeping order.
        seen = set()
        ordered = []
        for n in priority + names:
            if n in seen:
                continue
            seen.add(n)
            ordered.append(n)

        for name in ordered:
            if len(results) >= max_matches:
                break
            try:
                info = zf.getinfo(name)
            except KeyError:
                continue
            # Skip extremely large entries unless explicitly allowed (we cap scan).
            if info.file_size <= 0:
                continue
            with zf.open(name, "r") as handle:
                scanned, findings = scan_bytes_stream(
                    handle,
                    max_bytes=max_bytes_per_entry,
                    max_matches=max_matches - len(results),
                    allow_email_domains=allow_email_domains,
                )
            total_scanned += scanned
            for f in findings:
                f = dict(f)
                f["entry"] = name
                results.append(f)
                if len(results) >= max_matches:
                    break

    counts: dict[str, int] = {}
    counts_disallowed: dict[str, int] = {}
    for f in results:
        counts[f["kind"]] = counts.get(f["kind"], 0) + 1
        if not f.get("allowed", False):
            counts_disallowed[f["kind"]] = counts_disallowed.get(f["kind"], 0) + 1

    return {
        "total_scanned_bytes": total_scanned,
        "counts": counts,
        "counts_disallowed": counts_disallowed,
        "findings": results,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Heuristic PII scan for real-world datasets.")
    parser.add_argument(
        "--corpus-dir",
        type=Path,
        default=Path("corpus_public"),
        help="Corpus directory",
    )
    parser.add_argument(
        "--index",
        type=Path,
        default=None,
        help="Optional corpus index path (defaults to <corpus-dir>/index.json)",
    )
    parser.add_argument("--dataset-id", type=str, default="", help="Dataset id to scan (from corpus index)")
    parser.add_argument("--path", type=Path, default=None, help="Scan a direct path (bypasses dataset-id)")
    parser.add_argument(
        "--max-bytes-per-entry",
        type=int,
        default=64 * 1024 * 1024,
        help="Max bytes to scan per zip entry",
    )
    parser.add_argument(
        "--max-matches",
        type=int,
        default=25,
        help="Stop after collecting this many findings total",
    )
    parser.add_argument(
        "--allow-email-domain",
        action="append",
        default=[],
        help="Email domain(s) to treat as non-sensitive (repeatable), e.g. ons.gov.uk",
    )
    parser.add_argument(
        "--fail-on-findings",
        action="store_true",
        help="Exit non-zero if any findings are detected",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write JSON report to this path",
    )
    args = parser.parse_args()

    corpus_dir: Path = args.corpus_dir
    index_path: Path = args.index or (corpus_dir / "index.json")

    dataset_id = args.dataset_id.strip() or None
    path = args.path
    if dataset_id:
        path = resolve_dataset_path(dataset_id=dataset_id, corpus_dir=corpus_dir, index_path=index_path)

    if not path:
        raise SystemExit("ERROR: Provide --dataset-id or --path")
    if not path.exists():
        raise FileNotFoundError(str(path))

    allow_email_domains = {d.strip().lower() for d in args.allow_email_domain if d.strip()}

    payload: dict[str, Any] = {
        "timestamp": utc_now_iso(),
        "path": str(path),
        "sha256": sha256_path(path),
        "bytes": int(path.stat().st_size),
    }
    if dataset_id:
        payload["dataset_id"] = dataset_id

    ext = path.suffix.lower()
    if ext in {".xlsx", ".xlsm", ".xltx", ".xltm", ".pbix", ".pbit"}:
        try:
            payload["scan"] = scan_zip(
                path,
                max_bytes_per_entry=int(args.max_bytes_per_entry),
                max_matches=int(args.max_matches),
                allow_email_domains=allow_email_domains,
            )
        except zipfile.BadZipFile:
            payload["scan"] = {"error": "BadZipFile"}
    else:
        # Plain text fallback.
        text = path.read_text(encoding="utf-8", errors="ignore")
        findings = scan_text(
            text,
            max_matches=int(args.max_matches),
            allow_email_domains=allow_email_domains,
        )
        counts: dict[str, int] = {}
        counts_disallowed: dict[str, int] = {}
        for f in findings:
            counts[f["kind"]] = counts.get(f["kind"], 0) + 1
            if not f.get("allowed", False):
                counts_disallowed[f["kind"]] = counts_disallowed.get(f["kind"], 0) + 1
        payload["scan"] = {"counts": counts, "counts_disallowed": counts_disallowed, "findings": findings}

    out_path = args.out
    if out_path:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"Wrote: {out_path}")
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))

    has_disallowed = bool(payload.get("scan", {}).get("counts_disallowed"))
    if args.fail_on_findings and has_disallowed:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
