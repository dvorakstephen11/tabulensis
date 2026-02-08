#!/usr/bin/env python3
"""
Shared helpers for the real-world dataset program.

This module is intentionally dependency-light (stdlib + PyYAML which is already used by repo scripts).
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

import yaml


DEFAULT_EXTS = {".xlsx", ".xlsm", ".xltx", ".xltm", ".pbix", ".pbit"}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_path(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def sha256_stream(handle, *, max_bytes: int | None = None) -> tuple[str, int]:
    hasher = hashlib.sha256()
    total = 0
    while True:
        chunk = handle.read(1024 * 1024)
        if not chunk:
            break
        total += len(chunk)
        if max_bytes is not None and total > max_bytes:
            raise ValueError(f"Stream exceeded max_bytes={max_bytes}")
        hasher.update(chunk)
    return hasher.hexdigest(), total


def load_yaml(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(str(path))
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if data is None:
        return {}
    if not isinstance(data, dict):
        raise ValueError(f"Expected YAML mapping at {path}, got {type(data)}")
    return data


def save_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(str(path))
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"Expected JSON object at {path}, got {type(data)}")
    return data


def normalize_kind(kind: str) -> str:
    kind = kind.strip().lower().lstrip(".")
    if kind not in {"xlsx", "xlsm", "xltx", "xltm", "pbix", "pbit"}:
        raise ValueError(f"Unsupported kind: {kind}")
    return kind


def kind_to_ext(kind: str) -> str:
    return "." + normalize_kind(kind)


def load_registry(path: Path) -> dict[str, Any]:
    data = load_yaml(path)
    version = int(data.get("version", 0) or 0)
    if version != 1:
        raise ValueError(f"Unsupported registry version {version} in {path}")
    datasets = data.get("datasets", [])
    if datasets is None:
        datasets = []
    if not isinstance(datasets, list):
        raise ValueError(f"registry.datasets must be a list in {path}")
    return {"version": 1, "datasets": datasets}


def load_cases(path: Path) -> dict[str, Any]:
    data = load_yaml(path)
    version = int(data.get("version", 0) or 0)
    if version != 1:
        raise ValueError(f"Unsupported cases version {version} in {path}")
    cases = data.get("cases", [])
    if cases is None:
        cases = []
    if not isinstance(cases, list):
        raise ValueError(f"cases.cases must be a list in {path}")
    return {"version": 1, "cases": cases}


def iter_input_files(root: Path, recursive: bool, exts: set[str]) -> list[Path]:
    pattern = "**/*" if recursive else "*"
    files: list[Path] = []
    for path in root.glob(pattern):
        if path.is_file() and path.suffix.lower() in exts:
            files.append(path)
    return files


def load_corpus_index(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "files": []}
    data = load_json(path)
    if int(data.get("version", 0) or 0) != 1:
        raise ValueError(f"Unsupported corpus index version in {path}")
    files = data.get("files", [])
    if files is None:
        files = []
    if not isinstance(files, list):
        raise ValueError(f"corpus index files must be a list in {path}")
    data.setdefault("files", files)
    return data


def save_corpus_index(path: Path, index: dict[str, Any]) -> None:
    # Keep stable ordering for diffs.
    files = index.get("files", [])
    if isinstance(files, list):
        def sort_key(entry: Any) -> tuple:
            if not isinstance(entry, dict):
                return ("", "")
            return (str(entry.get("dataset_id") or ""), str(entry.get("sha256") or ""))

        files_sorted = sorted(files, key=sort_key)
        index = dict(index)
        index["files"] = files_sorted
    save_json(path, index)


def corpus_blob_filename(sha256: str, ext: str) -> str:
    ext = ext.lower().lstrip(".")
    return f"sha256_{sha256}.{ext}"


def ingest_file_to_corpus(
    path: Path,
    *,
    corpus_dir: Path,
    index_path: Path,
    dataset_id: str | None = None,
    source_tag: str | None = None,
) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(str(path))

    ext = path.suffix.lower()
    if ext not in DEFAULT_EXTS:
        raise ValueError(f"Unsupported extension for corpus ingest: {ext} ({path})")

    digest = sha256_path(path)
    dest_name = corpus_blob_filename(digest, ext)
    dest_path = corpus_dir / dest_name

    corpus_dir.mkdir(parents=True, exist_ok=True)
    index = load_corpus_index(index_path)

    existing = None
    for entry in index.get("files", []):
        if isinstance(entry, dict) and entry.get("sha256") == digest:
            existing = entry
            break

    if not dest_path.exists():
        shutil.copy2(path, dest_path)

    if existing is None:
        entry: dict[str, Any] = {
            "sha256": digest,
            "size_bytes": int(path.stat().st_size),
            "extension": ext,
            "filename": dest_name,
            "ingested_at": utc_now_iso(),
        }
        if dataset_id:
            entry["dataset_id"] = dataset_id
        if source_tag:
            entry["source_tag"] = source_tag
        index.setdefault("files", []).append(entry)
    else:
        # Best-effort annotate existing entries.
        if dataset_id and not existing.get("dataset_id"):
            existing["dataset_id"] = dataset_id
        if source_tag and not existing.get("source_tag"):
            existing["source_tag"] = source_tag
        if not existing.get("filename"):
            existing["filename"] = dest_name
        if not existing.get("extension"):
            existing["extension"] = ext
        if not existing.get("size_bytes"):
            existing["size_bytes"] = int(dest_path.stat().st_size)

    save_corpus_index(index_path, index)

    return {
        "sha256": digest,
        "size_bytes": int(dest_path.stat().st_size),
        "extension": ext,
        "filename": dest_name,
        "path": str(dest_path),
    }


def find_corpus_entry_by_dataset_id(index: dict[str, Any], dataset_id: str) -> dict[str, Any] | None:
    for entry in index.get("files", []):
        if not isinstance(entry, dict):
            continue
        if entry.get("dataset_id") == dataset_id:
            return entry
    return None


def find_corpus_entry_by_sha256(index: dict[str, Any], sha256: str) -> dict[str, Any] | None:
    for entry in index.get("files", []):
        if not isinstance(entry, dict):
            continue
        if entry.get("sha256") == sha256:
            return entry
    return None


def resolve_dataset_path(
    *,
    dataset_id: str,
    corpus_dir: Path,
    index_path: Path,
) -> Path:
    index = load_corpus_index(index_path)
    entry = find_corpus_entry_by_dataset_id(index, dataset_id)
    if not entry:
        raise FileNotFoundError(f"Dataset id not found in corpus index: {dataset_id}")
    filename = entry.get("filename")
    if not filename:
        sha = entry.get("sha256")
        ext = entry.get("extension")
        if not sha or not ext:
            raise FileNotFoundError(f"Corpus entry for {dataset_id} missing filename/sha/ext")
        filename = corpus_blob_filename(str(sha), str(ext))
    path = corpus_dir / str(filename)
    if not path.exists():
        raise FileNotFoundError(f"Corpus blob missing on disk: {path}")
    return path


@dataclass
class DownloadResult:
    sha256: str
    bytes: int
    path: Path


def download_url_to_path(
    url: str,
    dest_path: Path,
    *,
    timeout_seconds: int = 120,
    max_bytes: int | None = None,
    expected_sha256: str | None = None,
    user_agent: str = "tabulensis-real-world-downloader/1.0",
    retries: int = 2,
    retry_sleep_seconds: float = 2.0,
) -> DownloadResult:
    dest_path.parent.mkdir(parents=True, exist_ok=True)

    last_err: Exception | None = None
    for attempt in range(1, retries + 2):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": user_agent})
            with urllib.request.urlopen(req, timeout=timeout_seconds) as resp:
                hasher = hashlib.sha256()
                total = 0
                with dest_path.open("wb") as out:
                    while True:
                        chunk = resp.read(1024 * 1024)
                        if not chunk:
                            break
                        total += len(chunk)
                        if max_bytes is not None and total > max_bytes:
                            raise ValueError(f"Download exceeded max_bytes={max_bytes} ({total} bytes)")
                        hasher.update(chunk)
                        out.write(chunk)
            digest = hasher.hexdigest()
            if expected_sha256 and digest.lower() != expected_sha256.lower():
                raise ValueError(f"sha256 mismatch for {url}: expected {expected_sha256}, got {digest}")
            return DownloadResult(sha256=digest, bytes=total, path=dest_path)
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, ValueError) as exc:
            last_err = exc
            try:
                if dest_path.exists():
                    dest_path.unlink()
            except Exception:
                pass
            if attempt <= retries + 1:
                time.sleep(retry_sleep_seconds)
                continue
            break
    assert last_err is not None
    raise last_err

