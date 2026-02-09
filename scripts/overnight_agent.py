#!/usr/bin/env python3
"""Overnight Operator Agent

Implementation of `docs/meta/automation/overnight_operator_agent_plan.md`.

Entry points:
- `supervise`: time-budgeted supervisor loop with a single-instance lock.
- `run-once`: one restart-safe iteration:
  task -> plan (LLM) -> isolated worktree/branch -> implement (patch) -> validate -> commit -> report.

Key invariants:
- Never mutate the primary working tree (all code changes happen in a per-iteration worktree).
- No deploys, no secret rotation, no destructive git operations.
- Follow repo guardrails in `AGENTS.md` (perf policy, formatting scope, change-scope guard).

Runtime state (local-only; gitignored via `tmp/` and `*.sqlite`):
- `tmp/overnight_agent/state.sqlite3`
- `tmp/overnight_agent/state.json`
- `tmp/overnight_agent/runs/<run_id>/**` (raw prompts/responses/logs, stdout/stderr, timing)

Ops journal is written on a dedicated branch/worktree to avoid merge conflicts across task branches:
- `docs/meta/logs/ops/executive_summary.log` (append-only)
- `docs/meta/logs/ops/<run_id>_report.md`
- `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import fnmatch
import hashlib
import json
import os
import re
import shutil
import shlex
import sqlite3
import subprocess
import sys
import tempfile
import textwrap
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import yaml  # type: ignore
except Exception:  # pragma: no cover
    yaml = None

try:
    import requests  # type: ignore
except Exception:  # pragma: no cover
    requests = None


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CONFIG_PATH = ROOT / "docs" / "meta" / "automation" / "overnight_agent.yaml"


# -----------------------------
# Small utilities
# -----------------------------


class ConfigError(RuntimeError):
    pass


class GitError(RuntimeError):
    pass


class LlmError(RuntimeError):
    pass


def utc_now() -> dt.datetime:
    return dt.datetime.now(tz=dt.timezone.utc)


def iso_utc_now() -> str:
    return utc_now().isoformat(timespec="seconds")


def day_stamp_utc() -> str:
    return utc_now().strftime("%Y-%m-%d")


def safe_mkdir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def slugify(text: str, *, max_len: int = 50) -> str:
    s = re.sub(r"[^a-zA-Z0-9]+", "_", text.strip().lower()).strip("_")
    if not s:
        s = "task"
    return s[:max_len]


def sha1_text(text: str) -> str:
    return hashlib.sha1(text.encode("utf-8", errors="replace")).hexdigest()


def json_dumps(payload: Any) -> str:
    return json.dumps(payload, indent=2, sort_keys=True)


def relpath_posix(path: Path, start: Path) -> str:
    return Path(os.path.relpath(path, start=start)).as_posix()


def cfg_path(repo_root: Path, value: str) -> Path:
    p = Path(value)
    return (repo_root / p).resolve() if not p.is_absolute() else p.resolve()


def cfg_get_str(cfg: dict[str, Any], key: str, default: str) -> str:
    value = cfg.get(key, default)
    if not isinstance(value, str):
        raise ConfigError(f"Expected {key!r} to be a string")
    return value


def cfg_get_int(cfg: dict[str, Any], key: str, default: int) -> int:
    value = cfg.get(key, default)
    if not isinstance(value, int):
        raise ConfigError(f"Expected {key!r} to be an int")
    return value


def cfg_get_bool(cfg: dict[str, Any], key: str, default: bool) -> bool:
    value = cfg.get(key, default)
    if not isinstance(value, bool):
        raise ConfigError(f"Expected {key!r} to be a bool")
    return value


def expand_templates(obj: Any, vars: dict[str, str]) -> Any:
    if isinstance(obj, str):
        out = obj
        for k, v in vars.items():
            out = out.replace("{{" + k + "}}", v)
        return out
    if isinstance(obj, list):
        return [expand_templates(v, vars) for v in obj]
    if isinstance(obj, dict):
        return {k: expand_templates(v, vars) for k, v in obj.items()}
    return obj


def load_yaml(path: Path) -> dict[str, Any]:
    if yaml is None:
        raise ConfigError("PyYAML is not installed.")
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ConfigError(f"Failed to read config: {path} ({exc})") from exc
    if not isinstance(data, dict):
        raise ConfigError(f"Config root must be a mapping: {path}")
    return data


# -----------------------------
# Durable state (SQLite + JSON mirror)
# -----------------------------


SCHEMA_VERSION = 1


def open_state_db(state_path: Path) -> sqlite3.Connection:
    safe_mkdir(state_path.parent)
    conn = sqlite3.connect(state_path)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL;")
    conn.execute("PRAGMA synchronous=NORMAL;")
    _init_db(conn)
    return conn


def _init_db(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        """
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS tasks (
          task_key TEXT PRIMARY KEY,
          source_kind TEXT NOT NULL,
          source_path TEXT NOT NULL,
          line_number INTEGER NOT NULL,
          raw_text TEXT NOT NULL,
          priority INTEGER NOT NULL,
          status TEXT NOT NULL,
          attempt_count INTEGER NOT NULL,
          last_attempted_at TEXT,
          last_result TEXT,
          blocked_reason TEXT
        );
        """
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS iterations (
          run_id TEXT PRIMARY KEY,
          task_key TEXT,
          phase TEXT NOT NULL,
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          finished_at TEXT,
          branch TEXT,
          worktree_path TEXT,
          base_branch TEXT,
          base_commit TEXT,
          cycle_id TEXT,
          plan_json_path TEXT,
          last_error TEXT
        );
        """
    )
    cur = conn.execute("SELECT value FROM meta WHERE key='schema_version'")
    row = cur.fetchone()
    if row is None:
        conn.execute("INSERT INTO meta(key,value) VALUES('schema_version',?)", (str(SCHEMA_VERSION),))
    else:
        try:
            ver = int(row["value"])
        except Exception:
            ver = 0
        if ver != SCHEMA_VERSION:
            raise RuntimeError(
                f"Unsupported state DB schema_version={ver}; expected {SCHEMA_VERSION}. Delete it to recreate."
            )
    conn.commit()


def write_state_mirror(
    state_json_path: Path,
    *,
    run_id: str,
    phase: str,
    status: str,
    extra: dict[str, Any],
) -> None:
    safe_mkdir(state_json_path.parent)
    payload = {"timestamp_utc": iso_utc_now(), "run_id": run_id, "phase": phase, "status": status, **extra}
    state_json_path.write_text(json_dumps(payload) + "\n", encoding="utf-8")


def db_task_row(conn: sqlite3.Connection, task_key: str) -> sqlite3.Row | None:
    return conn.execute("SELECT * FROM tasks WHERE task_key=?", (task_key,)).fetchone()


def db_set_task(conn: sqlite3.Connection, task: "Task") -> None:
    conn.execute(
        """
        INSERT INTO tasks(task_key, source_kind, source_path, line_number, raw_text, priority,
                          status, attempt_count, last_attempted_at, last_result, blocked_reason)
        VALUES(?,?,?,?,?,?, 'pending', 0, NULL, NULL, NULL)
        ON CONFLICT(task_key) DO UPDATE SET
          source_kind=excluded.source_kind,
          source_path=excluded.source_path,
          line_number=excluded.line_number,
          raw_text=excluded.raw_text,
          priority=excluded.priority
        """,
        (task.key, task.source_kind, task.source_path, task.line_number, task.text, task.priority),
    )
    conn.commit()


def db_mark_task_attempt(conn: sqlite3.Connection, task_key: str) -> None:
    conn.execute(
        """
        UPDATE tasks SET
          attempt_count = attempt_count + 1,
          last_attempted_at = ?
        WHERE task_key=?
        """,
        (iso_utc_now(), task_key),
    )
    conn.commit()


def db_mark_task_done(conn: sqlite3.Connection, task_key: str, result: str) -> None:
    conn.execute(
        """
        UPDATE tasks SET
          status='done',
          last_result=?,
          blocked_reason=NULL
        WHERE task_key=?
        """,
        (result, task_key),
    )
    conn.commit()


def db_mark_task_failed(conn: sqlite3.Connection, task_key: str, result: str) -> None:
    conn.execute(
        """
        UPDATE tasks SET
          status='failed',
          last_result=?
        WHERE task_key=?
        """,
        (result, task_key),
    )
    conn.commit()


def db_mark_task_blocked(conn: sqlite3.Connection, task_key: str, reason: str) -> None:
    conn.execute(
        """
        UPDATE tasks SET
          status='blocked',
          blocked_reason=?
        WHERE task_key=?
        """,
        (reason, task_key),
    )
    conn.commit()


def db_active_iteration(conn: sqlite3.Connection) -> sqlite3.Row | None:
    return conn.execute(
        "SELECT * FROM iterations WHERE status='in_progress' ORDER BY started_at DESC LIMIT 1"
    ).fetchone()


def db_get_iteration(conn: sqlite3.Connection, run_id: str) -> sqlite3.Row | None:
    return conn.execute("SELECT * FROM iterations WHERE run_id=?", (run_id,)).fetchone()


def db_create_iteration(conn: sqlite3.Connection, run_id: str) -> None:
    now = iso_utc_now()
    conn.execute(
        """
        INSERT INTO iterations(run_id, task_key, phase, status, started_at, updated_at)
        VALUES(?, NULL, 'ACQUIRE_TASK', 'in_progress', ?, ?)
        """,
        (run_id, now, now),
    )
    conn.commit()


def db_update_iteration(
    conn: sqlite3.Connection,
    run_id: str,
    *,
    phase: str | None = None,
    status: str | None = None,
    finished: bool = False,
    task_key: str | None = None,
    branch: str | None = None,
    worktree_path: str | None = None,
    base_branch: str | None = None,
    base_commit: str | None = None,
    cycle_id: str | None = None,
    plan_json_path: str | None = None,
    last_error: str | None = None,
) -> None:
    fields: list[str] = []
    values: list[Any] = []
    now = iso_utc_now()
    fields.append("updated_at=?")
    values.append(now)
    if phase is not None:
        fields.append("phase=?")
        values.append(phase)
    if status is not None:
        fields.append("status=?")
        values.append(status)
    if finished:
        fields.append("finished_at=?")
        values.append(now)
    if task_key is not None:
        fields.append("task_key=?")
        values.append(task_key)
    if branch is not None:
        fields.append("branch=?")
        values.append(branch)
    if worktree_path is not None:
        fields.append("worktree_path=?")
        values.append(worktree_path)
    if base_branch is not None:
        fields.append("base_branch=?")
        values.append(base_branch)
    if base_commit is not None:
        fields.append("base_commit=?")
        values.append(base_commit)
    if cycle_id is not None:
        fields.append("cycle_id=?")
        values.append(cycle_id)
    if plan_json_path is not None:
        fields.append("plan_json_path=?")
        values.append(plan_json_path)
    if last_error is not None:
        fields.append("last_error=?")
        values.append(last_error)

    values.append(run_id)
    conn.execute(f"UPDATE iterations SET {', '.join(fields)} WHERE run_id=?", tuple(values))
    conn.commit()


# -----------------------------
# Task ingestion
# -----------------------------


RE_CHECKBOX_UNCHECKED = re.compile("^\\s*[-*]\\s+\\[\\s*\\]\\s+(?P<text>.+?)\\s*$")


@dataclass(frozen=True)
class Task:
    key: str
    source_kind: str
    source_path: str  # repo-relative
    line_number: int
    text: str
    priority: int


def parse_unchecked_tasks(md_path: Path, *, repo_root: Path) -> list[Task]:
    try:
        text = md_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return []

    repo_rel = md_path.relative_to(repo_root).as_posix()
    out: list[Task] = []
    for idx, line in enumerate(text.splitlines(), start=1):
        m = RE_CHECKBOX_UNCHECKED.match(line)
        if not m:
            continue
        raw = m.group("text").strip()
        norm = re.sub("\\s+", " ", raw)
        key = sha1_text(f"{repo_rel}:{norm}")
        out.append(
            Task(
                key=key,
                source_kind="markdown_checklist",
                source_path=repo_rel,
                line_number=idx,
                text=norm,
                priority=0,
            )
        )
    return out


RE_INDEX_ENTRY = re.compile(
    r"^- \[(?P<path>[^\]]+)\]\([^)]+\) \(open: (?P<open>\d+), done: (?P<done>\d+)\)\s*$"
)


def parse_docs_index_checklist_paths(index_path: Path, *, repo_root: Path) -> list[Path]:
    """Absolute Paths (within repo) for checklists listed in docs/index.md auto-index block."""
    try:
        text = index_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return []

    begin = "<!-- BEGIN CHECKLIST INDEX -->"
    end = "<!-- END CHECKLIST INDEX -->"
    in_block = False
    paths: list[Path] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == begin:
            in_block = True
            continue
        if stripped == end:
            in_block = False
            continue
        if not in_block:
            continue

        m = RE_INDEX_ENTRY.match(stripped)
        if not m:
            continue
        if int(m.group("open")) <= 0:
            continue

        repo_rel = m.group("path").strip()
        p = (repo_root / repo_rel).resolve()
        if not p.exists():
            continue
        try:
            _ = p.relative_to(repo_root)
        except ValueError:
            continue
        paths.append(p)

    seen: set[str] = set()
    unique: list[Path] = []
    for p in paths:
        rel = p.relative_to(repo_root).as_posix()
        if rel in seen:
            continue
        seen.add(rel)
        unique.append(p)
    unique.sort(key=lambda p: p.relative_to(repo_root).as_posix())
    return unique


def discover_tasks(cfg: dict[str, Any], *, repo_root: Path) -> list[Task]:
    sources = cfg.get("tasks", {}).get("sources", [])
    if not isinstance(sources, list):
        raise ConfigError("tasks.sources must be a list")

    tasks: list[Task] = []
    for src in sources:
        if not isinstance(src, dict):
            continue
        kind = cfg_get_str(src, "kind", "")
        enabled = cfg_get_bool(src, "enabled", True)
        if not enabled:
            continue
        priority = cfg_get_int(src, "priority", 0)
        path_s = cfg_get_str(src, "path", "")
        if not path_s:
            continue

        p = (repo_root / path_s).resolve()
        if kind == "markdown_checklist":
            for t in parse_unchecked_tasks(p, repo_root=repo_root):
                tasks.append(dataclasses.replace(t, priority=priority))
        elif kind == "docs_index_checklists":
            for md in parse_docs_index_checklist_paths(p, repo_root=repo_root):
                for t in parse_unchecked_tasks(md, repo_root=repo_root):
                    tasks.append(dataclasses.replace(t, priority=priority))
        else:
            raise ConfigError(f"Unknown task source kind: {kind!r}")

    tasks.sort(key=lambda t: (-t.priority, t.source_path, t.line_number, t.key))
    return tasks


def try_checkoff_task(repo_root: Path, task: Task) -> bool:
    """Best-effort: mark the first matching `- [ ] <task>` line as done (`- [x] ...`)."""
    src = repo_root / task.source_path
    if not src.exists() or not src.is_file():
        return False
    try:
        text = src.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        m = RE_CHECKBOX_UNCHECKED.match(line)
        if not m:
            continue
        if (m.group("text") or "").strip() != task.text.strip():
            continue
        # Preserve indentation/bullet; replace the bracket content only.
        lines[i] = re.sub("\\[\\s*\\]", "[x]", line, count=1)
        new_text = "".join(lines)
        if new_text != text:
            src.write_text(new_text, encoding="utf-8", newline="\\n")
            return True
        return False
    return False


def select_next_task(conn: sqlite3.Connection, cfg: dict[str, Any], candidates: list[Task]) -> Task | None:
    sel_cfg = cfg.get("tasks", {}).get("selection", {})
    if not isinstance(sel_cfg, dict):
        sel_cfg = {}

    max_attempts = cfg_get_int(sel_cfg, "max_attempts", 3)
    avoid_recent_minutes = cfg_get_int(sel_cfg, "avoid_recent_minutes", 180)

    skip_regexes = sel_cfg.get("skip_regex", [])
    if not isinstance(skip_regexes, list):
        skip_regexes = []
    compiled_skips: list[re.Pattern[str]] = []
    for s in skip_regexes:
        if not isinstance(s, str) or not s.strip():
            continue
        compiled_skips.append(re.compile(s))

    # Defer decision-heavy tasks (do not block them; just prefer other tasks first).
    defer_regexes = sel_cfg.get(
        "defer_regex",
        [
            r"^Decide\b",
            r"^Choose\b",
            r"^If local-only:",
            r"^If committed:",
        ],
    )
    if not isinstance(defer_regexes, list):
        defer_regexes = [r"^Decide\b", r"^Choose\b", r"^If local-only:", r"^If committed:"]
    compiled_defers: list[re.Pattern[str]] = []
    for s in defer_regexes:
        if not isinstance(s, str) or not s.strip():
            continue
        compiled_defers.append(re.compile(s))

    def _is_deferred(text: str) -> bool:
        return any(r.search(text) for r in compiled_defers)

    now = utc_now()
    for allow_deferred in (False, True):
        for t in candidates:
            if (not allow_deferred) and _is_deferred(t.text):
                continue

            db_set_task(conn, t)
            row = db_task_row(conn, t.key)
            if row is None:
                continue

            status = str(row["status"])
            attempts = int(row["attempt_count"])
            last_attempted_at = row["last_attempted_at"]

            if status == "done":
                continue
            if status == "blocked":
                continue
            if status == "failed" and attempts >= max_attempts:
                continue

            if any(r.search(t.text) for r in compiled_skips):
                db_mark_task_blocked(conn, t.key, "Skipped by tasks.selection.skip_regex")
                continue

            if last_attempted_at:
                try:
                    last = dt.datetime.fromisoformat(str(last_attempted_at))
                    if last.tzinfo is None:
                        last = last.replace(tzinfo=dt.timezone.utc)
                    if (now - last) < dt.timedelta(minutes=avoid_recent_minutes):
                        continue
                except Exception:
                    pass

            return t

    return None


# -----------------------------
# Git helpers (worktrees, branches)
# -----------------------------


def run_git(repo_root: Path, args: list[str], *, capture: bool = True, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        capture_output=capture,
        text=True,
        check=False,
    )
    if check and result.returncode != 0:
        raise GitError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result


def git_current_commit(repo_root: Path, ref: str = "HEAD") -> str:
    return run_git(repo_root, ["rev-parse", ref]).stdout.strip()


def git_changed_paths(repo_root: Path) -> list[str]:
    out = run_git(repo_root, ["status", "--porcelain"]).stdout
    paths: set[str] = set()
    for line in out.splitlines():
        if not line:
            continue
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        if path:
            paths.add(path)
    return sorted(paths)


def git_staged_paths(repo_root: Path) -> list[str]:
    out = run_git(repo_root, ["diff", "--cached", "--name-only", "--diff-filter=ACMR"]).stdout
    return sorted({line.strip() for line in out.splitlines() if line.strip()})


def git_has_staged_changes(repo_root: Path) -> bool:
    return subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=repo_root).returncode != 0


def git_has_worktree_changes(repo_root: Path) -> bool:
    if subprocess.run(["git", "diff", "--quiet"], cwd=repo_root).returncode != 0:
        return True
    if subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=repo_root).returncode != 0:
        return True
    return bool(run_git(repo_root, ["status", "--porcelain"]).stdout.strip())


def git_status_entries(repo_root: Path) -> list[tuple[str, str]]:
    """Return (xy, path) from `git status --porcelain` (best-effort, minimal parsing)."""
    out = run_git(repo_root, ["status", "--porcelain"]).stdout
    entries: list[tuple[str, str]] = []
    for line in out.splitlines():
        if not line:
            continue
        xy = line[:2]
        path = line[3:]
        if "->" in path:
            path = path.split("->", 1)[1].strip()
        entries.append((xy, path))
    return entries


def enforce_new_doc_indexing(cfg: dict[str, Any], *, repo_root: Path) -> None:
    policy = cfg.get("policy", {})
    if not isinstance(policy, dict):
        policy = {}
    require_index = bool(policy.get("new_docs_require_index", True))
    if not require_index:
        return

    exempt = policy.get("new_docs_index_exempt_prefixes", ["docs/meta/logs/", "docs/meta/results/"])
    if not isinstance(exempt, list):
        exempt = ["docs/meta/logs/", "docs/meta/results/"]
    exempt_prefixes = [str(x) for x in exempt if isinstance(x, str)]

    docs_cfg = cfg.get("docs", {})
    index_path = repo_root / "docs" / "index.md"
    if isinstance(docs_cfg, dict):
        p = docs_cfg.get("index_file")
        if isinstance(p, str) and p.strip():
            index_path = cfg_path(repo_root, p)

    if not index_path.exists():
        return

    index_text = index_path.read_text(encoding="utf-8", errors="replace")
    index_dir = index_path.parent

    new_md: list[str] = []
    for xy, path in git_status_entries(repo_root):
        if not path.endswith(".md"):
            continue
        if xy == "??" or "A" in xy:
            new_md.append(path)

    if not new_md:
        return

    missing: list[str] = []
    for rel in sorted(set(new_md)):
        if any(rel.startswith(prefix) for prefix in exempt_prefixes):
            continue
        abs_p = (repo_root / rel).resolve()
        rel_link = Path(os.path.relpath(abs_p, start=index_dir)).as_posix()
        if rel_link in index_text:
            continue
        # Fallback: allow basename match (sometimes links use different relative prefix).
        if abs_p.name in index_text:
            continue
        missing.append(rel)

    if missing:
        raise RuntimeError(
            "New Markdown files must be linked from docs/index.md (doc sprawl guard). Missing:\n"
            + "\n".join(f"- {p}" for p in missing)
        )


def ensure_branch_exists(repo_root: Path, branch: str, base_branch: str) -> None:
    res = run_git(repo_root, ["show-ref", "--verify", "--quiet", f"refs/heads/{branch}"], capture=False, check=False)
    if res.returncode == 0:
        return
    run_git(repo_root, ["branch", branch, base_branch], capture=True, check=True)


def ensure_worktree(
    repo_root: Path,
    *,
    base_branch: str,
    branch: str,
    worktree_path: Path,
) -> tuple[str, str]:
    """Return (base_commit, worktree_path_str). Idempotent if worktree already exists."""
    safe_mkdir(worktree_path.parent)
    if worktree_path.exists():
        run_git(worktree_path, ["rev-parse", "--is-inside-work-tree"], capture=True, check=True)
        base_commit = git_current_commit(repo_root, base_branch)
        return base_commit, str(worktree_path)
    base_commit = git_current_commit(repo_root, base_branch)
    run_git(repo_root, ["worktree", "add", "-b", branch, str(worktree_path), base_branch], capture=True, check=True)
    return base_commit, str(worktree_path)


# -----------------------------
# Command runner (policy + logging)
# -----------------------------


@dataclass(frozen=True)
class CmdResult:
    cmd: list[str]
    cwd: str
    exit_code: int
    duration_s: float
    stdout_path: str
    stderr_path: str


def _cmd_to_str(cmd: list[str]) -> str:
    return " ".join(shlex.quote(c) for c in cmd)


def compile_forbidden_cmds(cfg: dict[str, Any]) -> list[re.Pattern[str]]:
    patterns = cfg.get("policy", {}).get("forbid_commands_regex", [])
    if not isinstance(patterns, list):
        patterns = []
    out: list[re.Pattern[str]] = []
    for p in patterns:
        if not isinstance(p, str) or not p.strip():
            continue
        out.append(re.compile(p))
    return out


def enforce_forbidden_cmds(cmd: list[str], forbidden: list[re.Pattern[str]]) -> None:
    joined = _cmd_to_str(cmd)
    for pat in forbidden:
        if pat.search(joined):
            raise RuntimeError(f"Refusing to run forbidden command (matched {pat.pattern!r}): {joined}")


def run_cmd(
    *,
    cmd: list[str],
    cwd: Path,
    env: dict[str, str] | None,
    forbidden: list[re.Pattern[str]],
    out_dir: Path,
    label: str,
    timeout_s: int | None = None,
) -> CmdResult:
    enforce_forbidden_cmds(cmd, forbidden)
    safe_mkdir(out_dir)
    stdout_path = out_dir / f"{label}.stdout.txt"
    stderr_path = out_dir / f"{label}.stderr.txt"
    started = time.time()
    with stdout_path.open("w", encoding="utf-8", errors="replace") as out_f, stderr_path.open(
        "w", encoding="utf-8", errors="replace"
    ) as err_f:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            env=env,
            stdout=out_f,
            stderr=err_f,
            text=True,
            timeout=timeout_s,
            check=False,
        )
    duration_s = time.time() - started
    return CmdResult(
        cmd=cmd,
        cwd=str(cwd),
        exit_code=int(proc.returncode),
        duration_s=float(duration_s),
        stdout_path=str(stdout_path),
        stderr_path=str(stderr_path),
    )


# -----------------------------
# LLM provider (OpenAI via HTTP; no openai-python dependency)
# -----------------------------


@dataclass(frozen=True)
class LlmPlan:
    goal: str
    proposed_changes: list[dict[str, Any]]
    predicted_touched_paths: list[str]
    risk_class: str
    validation_plan: dict[str, Any]
    stop_conditions: list[str]


class LlmClient:
    def plan(self, *, system: str, user: str) -> str:
        raise NotImplementedError

    def patch(self, *, system: str, user: str) -> str:
        raise NotImplementedError


class OpenAIChatCompletionsClient(LlmClient):
    def __init__(
        self,
        *,
        base_url: str,
        api_key: str,
        model: str,
        temperature: float,
        timeout_s: int,
    ) -> None:
        if requests is None:
            raise ConfigError("requests is not installed (needed for OpenAI HTTP calls).")
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._model = model
        self._temperature = float(temperature)
        self._timeout_s = int(timeout_s)

    def _call(self, *, system: str, user: str) -> str:
        url = f"{self._base_url}/chat/completions"
        headers = {"Authorization": f"Bearer {self._api_key}", "Content-Type": "application/json"}
        payload = {
            "model": self._model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "temperature": self._temperature,
        }
        try:
            resp = requests.post(url, headers=headers, json=payload, timeout=self._timeout_s)
        except Exception as exc:
            raise LlmError(f"OpenAI request failed: {exc}") from exc
        if resp.status_code >= 300:
            raise LlmError(f"OpenAI error {resp.status_code}: {resp.text[:500]}")
        data = resp.json()
        try:
            return str(data["choices"][0]["message"]["content"])
        except Exception as exc:
            raise LlmError(f"Unexpected OpenAI response shape: {data}") from exc

    def plan(self, *, system: str, user: str) -> str:
        return self._call(system=system, user=user)

    def patch(self, *, system: str, user: str) -> str:
        return self._call(system=system, user=user)


class MockClient(LlmClient):
    def __init__(self, *, plan_path: Path | None, patch_path: Path | None) -> None:
        self._plan_path = plan_path
        self._patch_path = patch_path

    def plan(self, *, system: str, user: str) -> str:
        if self._plan_path is None:
            raise LlmError("Mock plan_path not provided.")
        return self._plan_path.read_text(encoding="utf-8", errors="replace")

    def patch(self, *, system: str, user: str) -> str:
        if self._patch_path is None:
            raise LlmError("Mock patch_path not provided.")
        return self._patch_path.read_text(encoding="utf-8", errors="replace")


class CodexExecClient(LlmClient):
    """LLM provider via Codex CLI non-interactive sessions (`codex exec`)."""

    def __init__(
        self,
        *,
        codex_bin: str,
        model: str | None,
        extra_args: list[str],
        timeout_s: int,
    ) -> None:
        self._codex_bin = codex_bin
        self._model = model.strip() if isinstance(model, str) else ""
        self._extra_args = list(extra_args)
        self._timeout_s = int(timeout_s)

        if shutil.which(self._codex_bin) is None:
            raise ConfigError(
                f"Codex CLI not found on PATH: {self._codex_bin!r}. "
                "Install Codex CLI or set llm.codex_bin to the correct executable."
            )

    def _call(self, *, system: str, user: str) -> str:
        # Keep the prompt simple and explicit; Codex CLI takes a single prompt string.
        prompt = f"SYSTEM:\\n{system}\\n\\nUSER:\\n{user}\\n"

        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False, encoding="utf-8") as tmp:
            out_path = tmp.name

        cmd: list[str] = [self._codex_bin, "exec"]
        if self._model:
            # Per docs/real_world_datasets_perf_plan.md: `--model, -m <model>`
            cmd += ["--model", self._model]
        cmd += self._extra_args
        # Per docs: `--output-last-message, -o <path>`
        cmd += ["-o", out_path, prompt]

        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=self._timeout_s,
            check=False,
        )
        if proc.returncode != 0:
            raise LlmError(
                "codex exec failed:\n"
                + f"cmd: {_cmd_to_str(cmd)}\n"
                + f"exit: {proc.returncode}\n"
                + (f"stdout:\n{proc.stdout}\n" if proc.stdout else "")
                + (f"stderr:\n{proc.stderr}\n" if proc.stderr else "")
            )

        try:
            content = Path(out_path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            content = proc.stdout or ""
        finally:
            try:
                Path(out_path).unlink(missing_ok=True)  # py3.8+: missing_ok
            except Exception:
                pass

        content = content.strip()
        if content:
            return content
        # Fallback: last resort.
        return (proc.stdout or "").strip()

    def plan(self, *, system: str, user: str) -> str:
        return self._call(system=system, user=user)

    def patch(self, *, system: str, user: str) -> str:
        return self._call(system=system, user=user)


def plan_from_obj(obj: dict[str, Any]) -> LlmPlan:
    def _as_str(v: Any, name: str) -> str:
        if not isinstance(v, str):
            raise LlmError(f"Plan field {name!r} must be a string.")
        return v

    def _as_list(v: Any, name: str) -> list[Any]:
        if not isinstance(v, list):
            raise LlmError(f"Plan field {name!r} must be a list.")
        return v

    def _as_dict(v: Any, name: str) -> dict[str, Any]:
        if not isinstance(v, dict):
            raise LlmError(f"Plan field {name!r} must be an object.")
        return v

    return LlmPlan(
        goal=_as_str(obj.get("goal"), "goal"),
        proposed_changes=[x for x in _as_list(obj.get("proposed_changes"), "proposed_changes") if isinstance(x, dict)],
        predicted_touched_paths=[str(x) for x in _as_list(obj.get("predicted_touched_paths"), "predicted_touched_paths")],
        risk_class=_as_str(obj.get("risk_class"), "risk_class"),
        validation_plan=_as_dict(obj.get("validation_plan"), "validation_plan"),
        stop_conditions=[str(x) for x in _as_list(obj.get("stop_conditions"), "stop_conditions")],
    )


def _extract_first_json_obj(text: str) -> dict[str, Any]:
    start = text.find("{")
    if start < 0:
        raise LlmError("No JSON object found in LLM output.")
    for end in range(len(text), start + 1, -1):
        chunk = text[start:end].strip()
        if not chunk.endswith("}"):
            continue
        try:
            obj = json.loads(chunk)
        except Exception:
            continue
        if isinstance(obj, dict):
            return obj
    raise LlmError("Failed to parse JSON object from LLM output.")


def _extract_unified_diff(text: str) -> str:
    fence = re.search("```diff\\s*(.*?)```", text, flags=re.DOTALL | re.IGNORECASE)
    if fence:
        return fence.group(1).strip() + "\n"
    m = re.search(r"(^diff --git .*?$)", text, flags=re.MULTILINE)
    if m:
        return text[m.start() :].strip() + "\n"
    m2 = re.search(r"(^--- a/.*?$)", text, flags=re.MULTILINE)
    if m2:
        return text[m2.start() :].strip() + "\n"
    raise LlmError("No unified diff found in LLM output.")


def make_llm_client(cfg: dict[str, Any]) -> LlmClient:
    llm = cfg.get("llm", {})
    if not isinstance(llm, dict):
        raise ConfigError("llm must be a mapping")
    provider = cfg_get_str(llm, "provider", "openai_chat_completions")

    if provider == "mock":
        plan_path_s = llm.get("mock_plan_path")
        patch_path_s = llm.get("mock_patch_path")
        plan_path = cfg_path(ROOT, str(plan_path_s)) if isinstance(plan_path_s, str) and plan_path_s else None
        patch_path = cfg_path(ROOT, str(patch_path_s)) if isinstance(patch_path_s, str) and patch_path_s else None
        return MockClient(plan_path=plan_path, patch_path=patch_path)

    if provider == "codex_exec":
        codex_bin = cfg_get_str(llm, "codex_bin", "codex")
        model = cfg_get_str(llm, "model", "").strip() or None
        extra_args0 = llm.get("codex_exec_args", [])
        if not isinstance(extra_args0, list) or not all(isinstance(x, str) for x in extra_args0):
            raise ConfigError("llm.codex_exec_args must be a list of strings")
        timeout_s = int(llm.get("timeout_s", 600))
        return CodexExecClient(
            codex_bin=codex_bin,
            model=model,
            extra_args=[str(x) for x in extra_args0],
            timeout_s=timeout_s,
        )

    if provider != "openai_chat_completions":
        raise ConfigError(f"Unsupported llm.provider: {provider!r}")

    base_url = cfg_get_str(llm, "base_url", "https://api.openai.com/v1")
    model = cfg_get_str(llm, "model", "")
    if not model:
        raise ConfigError("llm.model must be set")
    api_key_env = cfg_get_str(llm, "api_key_env", "OPENAI_API_KEY")
    api_key = os.environ.get(api_key_env, "").strip()
    if not api_key:
        raise ConfigError(f"Missing API key: set {api_key_env}=... in the environment")

    temperature = float(llm.get("temperature", 0.2))
    timeout_s = int(llm.get("timeout_s", 60))
    return OpenAIChatCompletionsClient(
        base_url=base_url,
        api_key=api_key,
        model=model,
        temperature=temperature,
        timeout_s=timeout_s,
    )


# -----------------------------
# Validation: suites + triggers + pipeline
# -----------------------------


def match_trigger(changed_paths: list[str], trig: dict[str, Any]) -> bool:
    any_paths = trig.get("any_paths", [])
    if not isinstance(any_paths, list):
        any_paths = []
    patterns = [p for p in any_paths if isinstance(p, str) and p.strip()]
    if not patterns:
        return False
    for path in changed_paths:
        for pat in patterns:
            if fnmatch.fnmatch(path, pat):
                return True
    return False


def iter_suite_steps(cfg: dict[str, Any], suite_name: str, *, phase: str | None = None) -> list[dict[str, Any]]:
    suites = cfg.get("suites", {})
    if not isinstance(suites, dict) or suite_name not in suites:
        raise ConfigError(f"Unknown suite: {suite_name}")
    suite = suites[suite_name]
    if isinstance(suite, list):
        if phase is not None:
            raise ConfigError(f"Suite {suite_name} does not support phase {phase!r}")
        return [s for s in suite if isinstance(s, dict)]
    if isinstance(suite, dict):
        if phase is None:
            raise ConfigError(f"Suite {suite_name} requires phase ('pre' or 'post')")
        steps = suite.get(phase, [])
        if not isinstance(steps, list):
            raise ConfigError(f"Suite {suite_name}.{phase} must be a list")
        return [s for s in steps if isinstance(s, dict)]
    raise ConfigError(f"Suite {suite_name} must be a list or mapping")


def should_run_step(step: dict[str, Any], context: dict[str, bool]) -> bool:
    when = step.get("when")
    if when is None:
        return True
    if not isinstance(when, str) or not when.strip():
        return True
    return bool(context.get(when.strip(), False))


def run_step(
    *,
    step: dict[str, Any],
    repo_root: Path,
    forbidden: list[re.Pattern[str]],
    out_dir: Path,
    label: str,
    vars: dict[str, str],
    context: dict[str, bool],
) -> CmdResult:
    if not should_run_step(step, context):
        return CmdResult(cmd=["true"], cwd=str(repo_root), exit_code=0, duration_s=0.0, stdout_path="", stderr_path="")

    recipe = cfg_get_str(step, "recipe", "shell")
    cwd_s = cfg_get_str(step, "cwd", ".")
    cmd0 = step.get("cmd")
    if not isinstance(cmd0, list) or not all(isinstance(x, str) for x in cmd0):
        raise ConfigError(f"Invalid step.cmd (must be string list): {step}")
    cmd = [str(x) for x in expand_templates(cmd0, vars)]
    cwd = cfg_path(repo_root, cwd_s)
    if recipe not in {"shell", "python", "cargo", "npm"}:
        raise ConfigError(f"Unknown recipe: {recipe!r}")
    return run_cmd(cmd=cmd, cwd=cwd, env=None, forbidden=forbidden, out_dir=out_dir, label=label)


def run_suite(
    *,
    cfg: dict[str, Any],
    suite_name: str,
    repo_root: Path,
    forbidden: list[re.Pattern[str]],
    out_dir: Path,
    vars: dict[str, str],
    context: dict[str, bool],
    phase: str | None = None,
) -> tuple[bool, list[CmdResult]]:
    ok = True
    results: list[CmdResult] = []
    steps = iter_suite_steps(cfg, suite_name, phase=phase)
    for idx, step in enumerate(steps, start=1):
        res = run_step(
            step=step,
            repo_root=repo_root,
            forbidden=forbidden,
            out_dir=out_dir,
            label=f"{suite_name}.{phase or 'run'}.{idx:02d}",
            vars=vars,
            context=context,
        )
        results.append(res)
        if res.exit_code != 0:
            ok = False
            break
    return ok, results


# -----------------------------
# Ops journal (dedicated branch/worktree)
# -----------------------------


def ensure_git_identity(repo_root: Path, *, name: str, email: str) -> None:
    got_name = run_git(repo_root, ["config", "--get", "user.name"], check=False).stdout.strip()
    got_email = run_git(repo_root, ["config", "--get", "user.email"], check=False).stdout.strip()
    if not got_name:
        run_git(repo_root, ["config", "user.name", name], check=True)
    if not got_email:
        run_git(repo_root, ["config", "user.email", email], check=True)


def ensure_ops_journal_worktree(cfg: dict[str, Any], *, forbidden: list[re.Pattern[str]]) -> Path:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    journal_branch = cfg_get_str(repo_cfg, "ops_journal_branch", "overnight/ops-journal")
    journal_worktree = cfg_get_str(repo_cfg, "ops_journal_worktree", "../excel_diff_worktrees/overnight_ops_journal")
    base_branch = cfg_get_str(repo_cfg, "base_branch", "main")

    worktree_path = cfg_path(ROOT, journal_worktree)
    safe_mkdir(worktree_path.parent)

    ensure_branch_exists(ROOT, journal_branch, base_branch)

    if worktree_path.exists():
        run_git(worktree_path, ["rev-parse", "--is-inside-work-tree"], capture=True, check=True)
        run_git(worktree_path, ["checkout", journal_branch], capture=True, check=True)
        return worktree_path

    run_git(ROOT, ["worktree", "add", str(worktree_path), journal_branch], capture=True, check=True)
    return worktree_path


def write_ops_journal(
    cfg: dict[str, Any],
    *,
    forbidden: list[re.Pattern[str]],
    run_id: str,
    task: Task,
    branch: str,
    status: str,
    phase: str,
    msg: str,
    report_md: str,
) -> None:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    exec_log_s = cfg_get_str(repo_cfg, "exec_summary_log", "docs/meta/logs/ops/executive_summary.log")
    reports_dir_s = cfg_get_str(repo_cfg, "reports_dir", "docs/meta/logs/ops")
    git_ident = repo_cfg.get("git_identity", {})
    if not isinstance(git_ident, dict):
        git_ident = {}
    git_name = cfg_get_str(git_ident, "name", "Overnight Agent")
    git_email = cfg_get_str(git_ident, "email", "overnight-agent@localhost")

    journal_wt = ensure_ops_journal_worktree(cfg, forbidden=forbidden)
    ensure_git_identity(journal_wt, name=git_name, email=git_email)

    exec_log = cfg_path(journal_wt, exec_log_s)
    reports_dir = cfg_path(journal_wt, reports_dir_s)
    safe_mkdir(exec_log.parent)
    safe_mkdir(reports_dir)

    ts = iso_utc_now()
    task_slug = slugify(task.text, max_len=40)
    line = f"{ts} {run_id} {branch} {task_slug} phase={phase} result={status} msg={json.dumps(msg)}\\n"

    existing = ""
    if exec_log.exists():
        try:
            existing = exec_log.read_text(encoding="utf-8", errors="replace")
        except OSError:
            existing = ""
    if run_id not in existing:
        with exec_log.open("a", encoding="utf-8") as f:
            f.write(line)

    report_path = reports_dir / f"{run_id}_report.md"
    report_path.write_text(report_md, encoding="utf-8", newline="\\n")

    run_git(
        journal_wt,
        ["add", "-A", "--", relpath_posix(exec_log, journal_wt), relpath_posix(report_path, journal_wt)],
        capture=True,
        check=True,
    )
    if git_has_staged_changes(journal_wt):
        run_git(journal_wt, ["commit", "-m", f"ops: overnight run {run_id}"], capture=True, check=True)


def append_questions_for_operator(
    cfg: dict[str, Any],
    *,
    forbidden: list[re.Pattern[str]],
    run_id: str,
    task: Task,
    branch: str,
    questions: list[str],
) -> None:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    reports_dir_s = cfg_get_str(repo_cfg, "reports_dir", "docs/meta/logs/ops")
    git_ident = repo_cfg.get("git_identity", {})
    if not isinstance(git_ident, dict):
        git_ident = {}
    git_name = cfg_get_str(git_ident, "name", "Overnight Agent")
    git_email = cfg_get_str(git_ident, "email", "overnight-agent@localhost")

    journal_wt = ensure_ops_journal_worktree(cfg, forbidden=forbidden)
    ensure_git_identity(journal_wt, name=git_name, email=git_email)

    reports_dir = cfg_path(journal_wt, reports_dir_s)
    safe_mkdir(reports_dir)
    q_path = reports_dir / f"{day_stamp_utc()}_questions_for_operator.md"
    if not q_path.exists():
        q_path.write_text(f"# Questions For Operator ({day_stamp_utc()})\\n\\n", encoding="utf-8", newline="\\n")

    current = q_path.read_text(encoding="utf-8", errors="replace")
    if run_id not in current:
        section = [
            f"## {run_id}: {task.text}\\n\\n",
            f"- Task source: `{task.source_path}:{task.line_number}`\\n",
            f"- Branch: `{branch}`\\n\\n",
            "Questions:\\n",
        ]
        for q in questions:
            section.append(f"- {q}\\n")
        section.append("\\n")
        with q_path.open("a", encoding="utf-8") as f:
            f.writelines(section)

    run_git(journal_wt, ["add", "-A", "--", relpath_posix(q_path, journal_wt)], capture=True, check=True)
    if git_has_staged_changes(journal_wt):
        run_git(journal_wt, ["commit", "-m", f"ops: questions {day_stamp_utc()} ({run_id})"], capture=True, check=True)


def build_report_md(
    *,
    run_id: str,
    task: Task,
    branch: str,
    worktree_path: str | None,
    base_branch: str,
    base_commit: str | None,
    plan: LlmPlan | None,
    cmd_results: list[CmdResult],
    commits: dict[str, str],
    status: str,
    error: str | None,
) -> str:
    lines: list[str] = []
    lines.append(f"# Overnight Operator Report: {run_id}\\n\\n")
    lines.append(f"- Timestamp (UTC): {iso_utc_now()}\\n")
    lines.append(f"- Status: **{status}**\\n")
    if error:
        lines.append(f"- Error: `{error}`\\n")
    lines.append(f"- Branch: `{branch}`\\n")
    lines.append(f"- Worktree: `{worktree_path or 'n/a'}`\\n")
    lines.append(f"- Base branch: `{base_branch}`\\n")
    lines.append(f"- Base commit: `{base_commit or 'n/a'}`\\n")

    lines.append("\\n## Task\\n\\n")
    lines.append(f"- Source: `{task.source_path}:{task.line_number}`\\n")
    lines.append(f"- Text: {task.text}\\n")

    if plan is not None:
        lines.append("\\n## Plan (LLM)\\n\\n")
        lines.append("```json\\n")
        lines.append(
            json_dumps(
                {
                    "goal": plan.goal,
                    "proposed_changes": plan.proposed_changes,
                    "predicted_touched_paths": plan.predicted_touched_paths,
                    "risk_class": plan.risk_class,
                    "validation_plan": plan.validation_plan,
                    "stop_conditions": plan.stop_conditions,
                }
            )
        )
        lines.append("\\n```\\n")

    if commits:
        lines.append("\\n## Commits\\n\\n")
        for group, sha in commits.items():
            lines.append(f"- {group}: `{sha}`\\n")

    if cmd_results:
        lines.append("\\n## Commands\\n\\n")
        for res in cmd_results:
            lines.append(f"- `{_cmd_to_str(res.cmd)}` (cwd `{res.cwd}`) -> exit {res.exit_code}, {res.duration_s:.1f}s\\n")
            if res.stdout_path:
                lines.append(f"  - stdout: `{res.stdout_path}`\\n")
            if res.stderr_path:
                lines.append(f"  - stderr: `{res.stderr_path}`\\n")
    return "".join(lines)


# -----------------------------
# Runner state machine
# -----------------------------


PHASES = [
    "ACQUIRE_TASK",
    "PLAN",
    "WORKTREE_CREATE",
    "PRE_VALIDATE",
    "IMPLEMENT",
    "FORMAT",
    "TEST",
    "PERF_POST",
    "DOCS_REFRESH",
    "COMMIT",
    "REPORT",
    "DONE",
]


def next_phase(phase: str) -> str:
    if phase not in PHASES:
        return "DONE"
    idx = PHASES.index(phase)
    return PHASES[min(idx + 1, len(PHASES) - 1)]


def make_run_id() -> str:
    ts = utc_now().strftime("%Y-%m-%d_%H%M%S")
    salt = sha1_text(str(time.time_ns()))[:8]
    return f"{ts}_{salt}"


def read_text_head(path: Path, *, max_chars: int) -> str:
    try:
        data = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""
    if len(data) <= max_chars:
        return data
    return data[:max_chars] + "\n\n[...truncated...]\n"


def build_plan_prompt(*, task: Task, base_branch: str, repo_root: Path) -> tuple[str, str]:
    system = textwrap.dedent(
        """\
        You are an expert software engineer acting as a non-interactive "overnight operator" agent.
        You must be conservative, auditable, and follow repo policies.

        Non-negotiable constraints:
        - No deploys. No secret rotation. No destructive git operations (no reset --hard, no force push).
        - Avoid wide-scope churn (especially formatting). Prefer the smallest valid change.
        - Respect `AGENTS.md` perf policy: full perf cycle only for major perf-risk changes.
        - Do not request or use secrets.

        Output format:
        - Return a single JSON object (no markdown) with EXACT keys:
          goal, proposed_changes, predicted_touched_paths, risk_class, validation_plan, stop_conditions
        - risk_class MUST be one of:
          docs_only, minor, major_perf_risk, wide_scope, security_risk, decision_required
        """
    )
    agents_md = read_text_head(repo_root / "AGENTS.md", max_chars=8000)
    index_md = read_text_head(repo_root / "docs" / "index.md", max_chars=6000)
    plan_doc = read_text_head(
        repo_root / "docs" / "meta" / "automation" / "overnight_operator_agent_plan.md",
        max_chars=6000,
    )
    user = textwrap.dedent(
        f"""\
        Repository: excel_diff (Tabulensis)
        Base branch: {base_branch}

        Task source: {task.source_path}:{task.line_number}
        Task text: {task.text}

        Repo policies / guardrails:
        --- AGENTS.md (excerpt) ---
        {agents_md}

        --- docs/index.md (excerpt) ---
        {index_md}

        --- overnight agent plan (excerpt) ---
        {plan_doc}

        Additional planning rules:
        - If the task needs operator judgment, set risk_class=decision_required and put the exact question(s) in stop_conditions.
        - Keep proposed_changes concrete: list file paths and what to change.
        - predicted_touched_paths should include globs/paths you will likely change.
        """
    )
    return system, user


def build_patch_prompt(
    *,
    task: Task,
    plan: LlmPlan,
    repo_root: Path,
    changed_paths: list[str],
    round_idx: int,
) -> tuple[str, str]:
    system = textwrap.dedent(
        """\
        You are implementing a change in a git worktree.

        Output format:
        - Output ONLY a single unified diff (git apply compatible).
        - Use a fenced code block with ```diff.
        - No prose outside the diff.

        Constraints:
        - Smallest valid change that completes the task.
        - Do not propose forbidden operations (deploy/secret rotation/destructive git).
        - Avoid formatting churn: do NOT propose workspace-wide formatting.
        - If you create a new operating doc (SOP/runbook/checklist), ensure it is linked from docs/index.md.
        - Do not include secrets.
        """
    )

    context_parts: list[str] = []
    for row in plan.proposed_changes[:8]:
        pth = row.get("path")
        if not isinstance(pth, str) or not pth.strip():
            continue
        p = (repo_root / pth).resolve()
        if not p.exists() or not p.is_file():
            continue
        rel = p.relative_to(repo_root).as_posix()
        context_parts.append(f"\n--- FILE: {rel} ---\n")
        context_parts.append(read_text_head(p, max_chars=12000))
    if not context_parts:
        context_parts.append("\n--- FILE: docs/index.md ---\n")
        context_parts.append(read_text_head(repo_root / "docs" / "index.md", max_chars=12000))

    user = textwrap.dedent(
        f"""\
        Patch round: {round_idx}
        Task: {task.text}

        Plan goal: {plan.goal}
        Risk class: {plan.risk_class}
        Predicted touched paths: {json_dumps(plan.predicted_touched_paths)}

        Current changed paths (worktree): {json_dumps(changed_paths)}
        """
    )
    return system, user + "\n\nContext:\n" + "".join(context_parts)


def apply_patch(repo_root: Path, *, patch_text: str, patch_path: Path) -> None:
    safe_mkdir(patch_path.parent)
    patch_path.write_text(patch_text, encoding="utf-8", newline="\n")
    proc = subprocess.run(
        ["git", "apply", "--verbose", str(patch_path)],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError("git apply failed:\n" + (proc.stdout or "") + "\n" + (proc.stderr or ""))


def ensure_change_scope_ok(cfg: dict[str, Any], *, changed_paths: list[str]) -> None:
    max_files = int(cfg.get("policy", {}).get("max_changed_files_hard", 120))
    max_rust = int(cfg.get("policy", {}).get("max_changed_rust_files_hard", 70))
    rust_count = sum(1 for p in changed_paths if p.endswith(".rs"))
    if len(changed_paths) > max_files:
        raise RuntimeError(f"Change scope too wide: {len(changed_paths)} files > {max_files}")
    if rust_count > max_rust:
        raise RuntimeError(f"Rust change scope too wide: {rust_count} .rs files > {max_rust}")


def classify_paths_for_commits(paths: list[str]) -> dict[str, list[str]]:
    code: list[str] = []
    docs: list[str] = []
    perf: list[str] = []
    for p in paths:
        if p.startswith("benchmarks/"):
            perf.append(p)
        elif p.startswith("docs/") or p.endswith(".md") or p in {"AGENTS.md", "README.md"}:
            docs.append(p)
        else:
            code.append(p)
    return {"code": code, "docs": docs, "perf": perf}


def commit_group(
    *,
    cfg: dict[str, Any],
    repo_root: Path,
    forbidden: list[re.Pattern[str]],
    run_dir: Path,
    group_name: str,
    paths: list[str],
    message: str,
) -> str | None:
    if not paths:
        return None

    add_cmd = ["git", "add", "-A", "--", *paths]
    enforce_forbidden_cmds(add_cmd, forbidden)
    if subprocess.run(add_cmd, cwd=repo_root).returncode != 0:
        raise RuntimeError(f"git add failed for group {group_name}")

    max_files = int(cfg.get("policy", {}).get("max_changed_files_hard", 120))
    max_rust = int(cfg.get("policy", {}).get("max_changed_rust_files_hard", 70))

    eol = run_cmd(
        cmd=["python3", "scripts/check_line_endings.py", "--staged"],
        cwd=repo_root,
        env=None,
        forbidden=forbidden,
        out_dir=run_dir,
        label=f"guard.eol.{group_name}",
    )
    if eol.exit_code != 0:
        fix = run_cmd(
            cmd=["python3", "scripts/check_line_endings.py", "--staged", "--fix"],
            cwd=repo_root,
            env=None,
            forbidden=forbidden,
            out_dir=run_dir,
            label=f"guard.eol_fix.{group_name}",
        )
        if fix.exit_code != 0:
            raise RuntimeError(f"EOL guard failed for group {group_name} (see {fix.stderr_path})")
        if subprocess.run(add_cmd, cwd=repo_root).returncode != 0:
            raise RuntimeError(f"git add (restage) failed for group {group_name}")

    scope = run_cmd(
        cmd=[
            "python3",
            "scripts/check_change_scope.py",
            "--staged",
            "--max-files",
            str(max_files),
            "--max-rust-files",
            str(max_rust),
        ],
        cwd=repo_root,
        env=None,
        forbidden=forbidden,
        out_dir=run_dir,
        label=f"guard.scope.{group_name}",
    )
    if scope.exit_code != 0:
        raise RuntimeError(f"Change-scope guard failed for group {group_name} (see {scope.stderr_path})")

    staged = git_staged_paths(repo_root)
    if any(p.startswith("benchmarks/perf_cycles/") for p in staged):
        perf_guard = run_cmd(
            cmd=["python3", "scripts/check_perf_cycle_scope.py", "--staged"],
            cwd=repo_root,
            env=None,
            forbidden=forbidden,
            out_dir=run_dir,
            label=f"guard.perf_cycle.{group_name}",
        )
        if perf_guard.exit_code != 0:
            raise RuntimeError(f"Perf-cycle retention guard failed for group {group_name} (see {perf_guard.stderr_path})")

    if not git_has_staged_changes(repo_root):
        return None

    commit = run_cmd(
        cmd=["git", "commit", "-m", message],
        cwd=repo_root,
        env=None,
        forbidden=forbidden,
        out_dir=run_dir,
        label=f"git.commit.{group_name}",
    )
    if commit.exit_code != 0:
        raise RuntimeError(f"git commit failed for group {group_name} (see {commit.stderr_path})")
    return git_current_commit(repo_root)


def run_iteration(
    cfg: dict[str, Any],
    *,
    config_path: Path,
    run_id: str,
    conn: sqlite3.Connection,
    task_key: str | None,
    resume: bool,
    dry_run: bool,
    plan_only: bool,
) -> int:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    base_branch = cfg_get_str(repo_cfg, "base_branch", "main")
    worktree_root_s = cfg_get_str(repo_cfg, "worktree_root", "../excel_diff_worktrees/overnight")
    run_root_s = cfg_get_str(repo_cfg, "run_root", "tmp/overnight_agent")
    state_json_s = cfg_get_str(repo_cfg, "state_json", "tmp/overnight_agent/state.json")
    git_ident = repo_cfg.get("git_identity", {})
    if not isinstance(git_ident, dict):
        git_ident = {}
    git_name = cfg_get_str(git_ident, "name", "Overnight Agent")
    git_email = cfg_get_str(git_ident, "email", "overnight-agent@localhost")

    forbidden = compile_forbidden_cmds(cfg)
    run_root = cfg_path(ROOT, run_root_s)
    state_json_path = cfg_path(ROOT, state_json_s)

    if resume:
        active = db_active_iteration(conn)
        if active is not None:
            run_id = str(active["run_id"])

    it = db_get_iteration(conn, run_id)
    if it is None:
        db_create_iteration(conn, run_id)
        it = db_get_iteration(conn, run_id)
    assert it is not None

    phase = str(it["phase"])
    status = str(it["status"])

    run_dir = run_root / "runs" / run_id
    safe_mkdir(run_dir)

    task: Task | None = None
    plan: LlmPlan | None = None
    cmd_results: list[CmdResult] = []
    commits: dict[str, str] = {}

    def mirror(extra: dict[str, Any]) -> None:
        write_state_mirror(state_json_path, run_id=run_id, phase=phase, status=status, extra=extra)

    def update(**kwargs: Any) -> None:
        nonlocal phase, status
        db_update_iteration(conn, run_id, **kwargs)
        row2 = db_get_iteration(conn, run_id)
        assert row2 is not None
        phase = str(row2["phase"])
        status = str(row2["status"])

    mirror({"config": str(config_path)})

    try:
        while status == "in_progress" and phase != "DONE":
            mirror({"phase": phase})

            if phase == "ACQUIRE_TASK":
                candidates = discover_tasks(cfg, repo_root=ROOT)
                if task_key:
                    cand = next((t for t in candidates if t.key == task_key), None)
                    if cand is None:
                        raise RuntimeError(f"Task key not found in sources: {task_key}")
                    task = cand
                else:
                    task = select_next_task(conn, cfg, candidates)

                if task is None:
                    update(status="done", phase="DONE", finished=True, last_error=None)
                    return 0

                if not dry_run:
                    db_mark_task_attempt(conn, task.key)
                update(task_key=task.key, phase=next_phase(phase), last_error=None)
                continue

            # Resolve task after selection.
            if task is None:
                row = db_get_iteration(conn, run_id)
                assert row is not None
                tkey = str(row["task_key"])
                trow = db_task_row(conn, tkey)
                if trow is None:
                    candidates = discover_tasks(cfg, repo_root=ROOT)
                    task = next((t for t in candidates if t.key == tkey), None)
                    if task is None:
                        raise RuntimeError(f"Unable to resolve task {tkey} from sources")
                else:
                    task = Task(
                        key=tkey,
                        source_kind=str(trow["source_kind"]),
                        source_path=str(trow["source_path"]),
                        line_number=int(trow["line_number"]),
                        text=str(trow["raw_text"]),
                        priority=int(trow["priority"]),
                    )
            assert task is not None

            if phase == "PLAN":
                if dry_run:
                    plan = LlmPlan(
                        goal=task.text,
                        proposed_changes=[],
                        predicted_touched_paths=[],
                        risk_class="minor",
                        validation_plan={},
                        stop_conditions=[],
                    )
                else:
                    llm = make_llm_client(cfg)
                    sys_p, user_p = build_plan_prompt(task=task, base_branch=base_branch, repo_root=ROOT)
                    raw = llm.plan(system=sys_p, user=user_p)
                    (run_dir / "plan_raw.txt").write_text(raw, encoding="utf-8", newline="\n")
                    obj = _extract_first_json_obj(raw)
                    plan = plan_from_obj(obj)

                plan_path = run_dir / "plan.json"
                plan_path.write_text(
                    json_dumps(
                        {
                            "goal": plan.goal,
                            "proposed_changes": plan.proposed_changes,
                            "predicted_touched_paths": plan.predicted_touched_paths,
                            "risk_class": plan.risk_class,
                            "validation_plan": plan.validation_plan,
                            "stop_conditions": plan.stop_conditions,
                        }
                    )
                    + "\n",
                    encoding="utf-8",
                )
                update(plan_json_path=str(plan_path), phase=next_phase(phase), last_error=None)

                if plan.risk_class == "decision_required":
                    db_mark_task_blocked(conn, task.key, "decision_required (plan)")
                    try:
                        append_questions_for_operator(
                            cfg,
                            forbidden=forbidden,
                            run_id=run_id,
                            task=task,
                            branch="n/a",
                            questions=plan.stop_conditions or ["Decision required."],
                        )
                    except Exception:
                        pass
                    report_md = build_report_md(
                        run_id=run_id,
                        task=task,
                        branch="n/a",
                        worktree_path=None,
                        base_branch=base_branch,
                        base_commit=None,
                        plan=plan,
                        cmd_results=cmd_results,
                        commits=commits,
                        status="blocked",
                        error="decision_required",
                    )
                    try:
                        write_ops_journal(
                            cfg,
                            forbidden=forbidden,
                            run_id=run_id,
                            task=task,
                            branch="n/a",
                            status="blocked",
                            phase=phase,
                            msg="decision_required",
                            report_md=report_md,
                        )
                    except Exception:
                        pass
                    update(status="blocked", phase="DONE", finished=True, last_error="decision_required")
                    break

                if dry_run:
                    # Dry-run is non-destructive: stop after planning and do not create worktrees/branches.
                    update(status="done", phase="DONE", finished=True, last_error="dry_run")
                    break

                if plan_only:
                    # Plan-only: stop after planning (no worktree/branch); write a journal entry for review.
                    report_md = build_report_md(
                        run_id=run_id,
                        task=task,
                        branch="n/a",
                        worktree_path=None,
                        base_branch=base_branch,
                        base_commit=None,
                        plan=plan,
                        cmd_results=cmd_results,
                        commits=commits,
                        status="blocked",
                        error="plan_only",
                    )
                    try:
                        write_ops_journal(
                            cfg,
                            forbidden=forbidden,
                            run_id=run_id,
                            task=task,
                            branch="n/a",
                            status="blocked",
                            phase=phase,
                            msg="plan_only (no changes applied)",
                            report_md=report_md,
                        )
                    except Exception:
                        pass
                    update(status="blocked", phase="DONE", finished=True, last_error="plan_only")
                    break
                continue

            if phase == "WORKTREE_CREATE":
                task_slug = slugify(task.text)
                branch = f"overnight/{utc_now().strftime('%Y-%m-%d_%H%M')}_{task_slug}_{run_id[-8:]}"
                worktree_root = cfg_path(ROOT, worktree_root_s)
                wt_path = worktree_root / f"{run_id}_{task_slug}"
                base_commit, wt_str = ensure_worktree(ROOT, base_branch=base_branch, branch=branch, worktree_path=wt_path)
                update(
                    branch=branch,
                    worktree_path=wt_str,
                    base_branch=base_branch,
                    base_commit=base_commit,
                    phase=next_phase(phase),
                    last_error=None,
                )
                continue

            it2 = db_get_iteration(conn, run_id)
            assert it2 is not None
            wt_path_s = str(it2["worktree_path"] or "")
            branch = str(it2["branch"] or "")
            if not wt_path_s or not branch:
                raise RuntimeError("Missing worktree_path/branch in state")
            wt = Path(wt_path_s)

            if plan is None and it2["plan_json_path"]:
                try:
                    plan_obj = json.loads(Path(str(it2["plan_json_path"])).read_text(encoding="utf-8"))
                    plan = plan_from_obj(plan_obj)
                except Exception:
                    plan = None

            if phase == "PRE_VALIDATE":
                major = False
                triggers = cfg.get("triggers", {})
                if plan is not None:
                    if plan.risk_class == "major_perf_risk":
                        major = True
                    if isinstance(triggers, dict):
                        trig = triggers.get("major_perf_risk")
                        if isinstance(trig, dict) and match_trigger(plan.predicted_touched_paths, trig):
                            major = True
                if major:
                    cycle_id = run_id
                    if (wt / "benchmarks" / "perf_cycles" / cycle_id).exists():
                        raise RuntimeError(f"Perf cycle dir already exists for cycle_id={cycle_id}")
                    ok, results = run_suite(
                        cfg=cfg,
                        suite_name="perf_cycle_full",
                        phase="pre",
                        repo_root=wt,
                        forbidden=forbidden,
                        out_dir=run_dir,
                        vars={"cycle_id": cycle_id},
                        context={"perf_artifacts_staged": False},
                    )
                    cmd_results.extend(results)
                    if not ok:
                        raise RuntimeError("perf_cycle_full pre failed")
                    update(cycle_id=cycle_id, phase=next_phase(phase), last_error=None)
                else:
                    update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "IMPLEMENT":
                if plan_only:
                    update(status="blocked", phase="DONE", finished=True, last_error="plan_only")
                    break
                if dry_run:
                    update(phase=next_phase(phase), last_error=None)
                    continue
                if plan is None:
                    raise RuntimeError("Missing plan; cannot implement")

                max_rounds = int(cfg.get("implementation", {}).get("max_patch_rounds", 3))
                llm = make_llm_client(cfg)
                for i in range(1, max_rounds + 1):
                    changed = git_changed_paths(wt)
                    sys_p, user_p = build_patch_prompt(
                        task=task,
                        plan=plan,
                        repo_root=wt,
                        changed_paths=changed,
                        round_idx=i,
                    )
                    raw = llm.patch(system=sys_p, user=user_p)
                    (run_dir / f"patch_raw_{i:02d}.txt").write_text(raw, encoding="utf-8", newline="\n")
                    try:
                        diff = _extract_unified_diff(raw)
                    except LlmError:
                        if "DONE" in raw.strip().upper():
                            break
                        raise
                    apply_patch(wt, patch_text=diff, patch_path=run_dir / f"patch_{i:02d}.diff")

                    # If we crossed into major-perf-risk territory but didn't run pre: stash -> pre -> pop.
                    it3 = db_get_iteration(conn, run_id)
                    assert it3 is not None
                    have_cycle = bool(str(it3["cycle_id"] or "").strip())
                    triggers = cfg.get("triggers", {})
                    if not have_cycle and isinstance(triggers, dict):
                        trig = triggers.get("major_perf_risk")
                        if isinstance(trig, dict) and match_trigger(git_changed_paths(wt), trig):
                            subprocess.run(["git", "stash", "push", "-u", "-m", f"overnight_agent:{run_id}"], cwd=wt)
                            if git_has_worktree_changes(wt):
                                raise RuntimeError("Failed to stash changes for perf pre baseline")
                            cycle_id = run_id
                            ok2, results2 = run_suite(
                                cfg=cfg,
                                suite_name="perf_cycle_full",
                                phase="pre",
                                repo_root=wt,
                                forbidden=forbidden,
                                out_dir=run_dir,
                                vars={"cycle_id": cycle_id},
                                context={"perf_artifacts_staged": False},
                            )
                            cmd_results.extend(results2)
                            if not ok2:
                                raise RuntimeError("perf_cycle_full pre failed (late)")
                            pop = subprocess.run(["git", "stash", "pop"], cwd=wt)
                            if pop.returncode != 0:
                                raise RuntimeError("git stash pop failed after perf pre baseline")
                            update(cycle_id=cycle_id)

                changed_after = git_changed_paths(wt)
                if not changed_after:
                    raise RuntimeError("Implementation produced no changes")
                ensure_change_scope_ok(cfg, changed_paths=changed_after)
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "FORMAT":
                changed = git_changed_paths(wt)
                if any(p.endswith(".rs") for p in changed):
                    ok, results = run_suite(
                        cfg=cfg,
                        suite_name="fmt_rust",
                        repo_root=wt,
                        forbidden=forbidden,
                        out_dir=run_dir,
                        vars={"cycle_id": str(it2["cycle_id"] or "")},
                        context={"perf_artifacts_staged": False},
                    )
                    cmd_results.extend(results)
                    if not ok:
                        raise RuntimeError("Rust formatting failed")
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "TEST":
                changed = git_changed_paths(wt)
                triggers = cfg.get("triggers", {})
                pipeline = cfg.get("pipeline", {})
                if not isinstance(triggers, dict) or not isinstance(pipeline, dict):
                    triggers = {}
                    pipeline = {}
                for rule in pipeline.get("on_change", []) if isinstance(pipeline.get("on_change", []), list) else []:
                    if not isinstance(rule, dict):
                        continue
                    when = rule.get("when")
                    run_suites = rule.get("run", [])
                    if not isinstance(when, str) or not isinstance(run_suites, list):
                        continue
                    trig = triggers.get(when)
                    if not isinstance(trig, dict):
                        continue
                    if match_trigger(changed, trig):
                        for suite_name in [s for s in run_suites if isinstance(s, str)]:
                            ok, results = run_suite(
                                cfg=cfg,
                                suite_name=suite_name,
                                repo_root=wt,
                                forbidden=forbidden,
                                out_dir=run_dir,
                                vars={"cycle_id": str(it2["cycle_id"] or "")},
                                context={"perf_artifacts_staged": False},
                            )
                            cmd_results.extend(results)
                            if not ok:
                                raise RuntimeError(f"Suite failed: {suite_name}")
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "PERF_POST":
                it4 = db_get_iteration(conn, run_id)
                assert it4 is not None
                cycle_id = str(it4["cycle_id"] or "").strip()
                if cycle_id:
                    ok, results = run_suite(
                        cfg=cfg,
                        suite_name="perf_cycle_full",
                        phase="post",
                        repo_root=wt,
                        forbidden=forbidden,
                        out_dir=run_dir,
                        vars={"cycle_id": cycle_id},
                        context={"perf_artifacts_staged": False},
                    )
                    cmd_results.extend(results)
                    if not ok:
                        raise RuntimeError("perf_cycle_full post failed")
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "DOCS_REFRESH":
                # Best-effort: check off the originating checklist item so it disappears from the open queue.
                try_checkoff_task(wt, task)
                docs_cfg = cfg.get("docs", {})
                if isinstance(docs_cfg, dict) and isinstance(docs_cfg.get("checklist_index_refresh"), dict):
                    step = docs_cfg["checklist_index_refresh"]
                    res = run_step(
                        step=step,
                        repo_root=wt,
                        forbidden=forbidden,
                        out_dir=run_dir,
                        label="docs.checklist_index_refresh",
                        vars={"cycle_id": str(it2["cycle_id"] or "")},
                        context={"perf_artifacts_staged": False},
                    )
                    cmd_results.append(res)
                    if res.exit_code != 0:
                        raise RuntimeError("docs.checklist_index_refresh failed")
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "COMMIT":
                changed = git_changed_paths(wt)
                ensure_change_scope_ok(cfg, changed_paths=changed)
                enforce_new_doc_indexing(cfg, repo_root=wt)
                groups = classify_paths_for_commits(changed)
                ensure_git_identity(wt, name=git_name, email=git_email)

                if groups["code"]:
                    sha = commit_group(
                        cfg=cfg,
                        repo_root=wt,
                        forbidden=forbidden,
                        run_dir=run_dir,
                        group_name="code",
                        paths=groups["code"],
                        message=f"feat: {task.text} ({run_id})",
                    )
                    if sha:
                        commits["code"] = sha
                if groups["perf"]:
                    sha = commit_group(
                        cfg=cfg,
                        repo_root=wt,
                        forbidden=forbidden,
                        run_dir=run_dir,
                        group_name="perf",
                        paths=groups["perf"],
                        message=f"perf: {task.text} ({run_id})",
                    )
                    if sha:
                        commits["perf"] = sha
                if groups["docs"]:
                    sha = commit_group(
                        cfg=cfg,
                        repo_root=wt,
                        forbidden=forbidden,
                        run_dir=run_dir,
                        group_name="docs",
                        paths=groups["docs"],
                        message=f"docs: {task.text} ({run_id})",
                    )
                    if sha:
                        commits["docs"] = sha
                update(phase=next_phase(phase), last_error=None)
                continue

            if phase == "REPORT":
                it5 = db_get_iteration(conn, run_id)
                assert it5 is not None
                report_md = build_report_md(
                    run_id=run_id,
                    task=task,
                    branch=str(it5["branch"] or ""),
                    worktree_path=str(it5["worktree_path"] or ""),
                    base_branch=base_branch,
                    base_commit=str(it5["base_commit"] or ""),
                    plan=plan,
                    cmd_results=cmd_results,
                    commits=commits,
                    status="ok",
                    error=None,
                )
                write_ops_journal(
                    cfg,
                    forbidden=forbidden,
                    run_id=run_id,
                    task=task,
                    branch=str(it5["branch"] or ""),
                    status="ok",
                    phase=phase,
                    msg="completed",
                    report_md=report_md,
                )
                db_mark_task_done(conn, task.key, f"completed in {it5['branch']}")
                update(status="done", phase="DONE", finished=True, last_error=None)
                break

            raise RuntimeError(f"Unknown phase: {phase}")

    except Exception as exc:
        err = str(exc).strip() or exc.__class__.__name__
        db_update_iteration(conn, run_id, last_error=err)
        if task is not None:
            db_mark_task_failed(conn, task.key, err)
        db_update_iteration(conn, run_id, status="failed", phase="DONE", finished=True, last_error=err)
        try:
            itE = db_get_iteration(conn, run_id)
            if task is not None:
                report_md = build_report_md(
                    run_id=run_id,
                    task=task,
                    branch=str(itE["branch"] or "n/a") if itE else "n/a",
                    worktree_path=str(itE["worktree_path"] or "n/a") if itE else "n/a",
                    base_branch=base_branch,
                    base_commit=str(itE["base_commit"] or "n/a") if itE else "n/a",
                    plan=plan,
                    cmd_results=cmd_results,
                    commits=commits,
                    status="failed",
                    error=err,
                )
                write_ops_journal(
                    cfg,
                    forbidden=forbidden,
                    run_id=run_id,
                    task=task,
                    branch=str(itE["branch"] or "n/a") if itE else "n/a",
                    status="failed",
                    phase=phase,
                    msg=err[:180],
                    report_md=report_md,
                )
        except Exception:
            pass
        return 2

    return 0


# -----------------------------
# Supervisor (lock + restarts + time budget)
# -----------------------------


def acquire_lock(lock_path: Path) -> Any:
    import fcntl

    safe_mkdir(lock_path.parent)
    f = lock_path.open("a+", encoding="utf-8")
    try:
        fcntl.flock(f.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as exc:
        raise RuntimeError(f"Another overnight_agent instance is running (lock: {lock_path})") from exc
    f.seek(0)
    f.truncate()
    f.write(f"pid={os.getpid()} started_utc={iso_utc_now()}\\n")
    f.flush()
    return f


def supervise(
    cfg: dict[str, Any],
    *,
    config_path: Path,
    hours: float,
    poll_s: int,
    resume: bool,
    dry_run: bool,
) -> int:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    run_root_s = cfg_get_str(repo_cfg, "run_root", "tmp/overnight_agent")
    run_root = cfg_path(ROOT, run_root_s)
    lock_path = run_root / "lock"
    state_db = run_root / "state.sqlite3"

    started = time.time()
    deadline = started + float(hours) * 3600.0

    lock_f = acquire_lock(lock_path)
    try:
        conn = open_state_db(state_db)
        backoff_s = 5
        while time.time() < deadline:
            run_id = make_run_id()
            try:
                rc = run_iteration(
                    cfg,
                    config_path=config_path,
                    run_id=run_id,
                    conn=conn,
                    task_key=None,
                    resume=resume,
                    dry_run=dry_run,
                    plan_only=False,
                )
                backoff_s = 5
                if rc == 0:
                    time.sleep(2)
                    continue
                time.sleep(backoff_s)
                backoff_s = min(backoff_s * 2, 120)
            except KeyboardInterrupt:
                return 130
            except Exception:
                time.sleep(backoff_s)
                backoff_s = min(backoff_s * 2, 120)
            time.sleep(poll_s)
        return 0
    finally:
        try:
            lock_f.close()
        except Exception:
            pass


# -----------------------------
# CLI
# -----------------------------


def cmd_doctor(cfg: dict[str, Any]) -> int:
    print(f"repo_root: {ROOT}")
    print(f"python: {sys.version.split()[0]}")
    print(f"PyYAML: {'ok' if yaml is not None else 'missing'}")
    print(f"requests: {'ok' if requests is not None else 'missing'}")
    repo_cfg = cfg.get("repo", {})
    if isinstance(repo_cfg, dict):
        print(f"base_branch: {repo_cfg.get('base_branch', 'main')}")
    try:
        head = git_current_commit(ROOT)
        print(f"git HEAD: {head}")
    except Exception as exc:
        print(f"git: ERROR ({exc})")
        return 2
    try:
        _ = make_llm_client(cfg)
        print("llm: ok (configured)")
    except Exception as exc:
        print(f"llm: ERROR ({exc})")
    return 0


def cmd_list_tasks(cfg: dict[str, Any], *, limit: int) -> int:
    repo_cfg = cfg.get("repo", {})
    run_root_s = "tmp/overnight_agent"
    if isinstance(repo_cfg, dict):
        run_root_s = cfg_get_str(repo_cfg, "run_root", run_root_s)
    run_root = cfg_path(ROOT, run_root_s)
    conn = open_state_db(run_root / "state.sqlite3")
    tasks = discover_tasks(cfg, repo_root=ROOT)
    shown = 0
    for t in tasks:
        db_set_task(conn, t)
        row = db_task_row(conn, t.key)
        attempts = int(row["attempt_count"]) if row else 0
        status = str(row["status"]) if row else "pending"
        print(
            f"{t.key[:10]} prio={t.priority} {status} attempts={attempts} {t.source_path}:{t.line_number} {t.text}"
        )
        shown += 1
        if shown >= limit:
            break
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run the Tabulensis overnight operator agent.")
    parser.add_argument(
        "--config",
        type=str,
        default=str(DEFAULT_CONFIG_PATH),
        help=f"Path to config YAML (default: {DEFAULT_CONFIG_PATH})",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("doctor", help="Check environment + config wiring")

    p_list = sub.add_parser("list-tasks", help="List discovered tasks (deterministic order)")
    p_list.add_argument("--limit", type=int, default=50)

    p_once = sub.add_parser("run-once", help="Run one iteration (or resume active)")
    p_once.add_argument("--task-key", type=str, default=None, help="Force a specific task key")
    p_once.add_argument("--no-resume", action="store_true", help="Do not resume an active iteration")
    p_once.add_argument("--dry-run", action="store_true", help="No LLM calls; no changes")
    p_once.add_argument("--plan-only", action="store_true", help="Plan + report only (no implementation)")

    p_sup = sub.add_parser("supervise", help="Run iterations until time budget expires")
    p_sup.add_argument("--hours", type=float, default=10.0)
    p_sup.add_argument("--poll-seconds", type=int, default=10)
    p_sup.add_argument("--no-resume", action="store_true")
    p_sup.add_argument("--dry-run", action="store_true")

    args = parser.parse_args(argv)
    cfg_path0 = cfg_path(ROOT, args.config)
    cfg = load_yaml(cfg_path0)

    if args.cmd == "doctor":
        return cmd_doctor(cfg)
    if args.cmd == "list-tasks":
        return cmd_list_tasks(cfg, limit=int(args.limit))
    if args.cmd == "run-once":
        repo_cfg = cfg.get("repo", {})
        run_root_s = "tmp/overnight_agent"
        if isinstance(repo_cfg, dict):
            run_root_s = cfg_get_str(repo_cfg, "run_root", run_root_s)
        run_root = cfg_path(ROOT, run_root_s)
        conn = open_state_db(run_root / "state.sqlite3")
        run_id = make_run_id()
        resume = not bool(args.no_resume)
        return run_iteration(
            cfg,
            config_path=cfg_path0,
            run_id=run_id,
            conn=conn,
            task_key=str(args.task_key) if args.task_key else None,
            resume=resume,
            dry_run=bool(args.dry_run),
            plan_only=bool(args.plan_only),
        )
    if args.cmd == "supervise":
        resume = not bool(args.no_resume)
        return supervise(
            cfg,
            config_path=cfg_path0,
            hours=float(args.hours),
            poll_s=int(args.poll_seconds),
            resume=resume,
            dry_run=bool(args.dry_run),
        )
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
