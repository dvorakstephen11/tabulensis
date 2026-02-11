#!/usr/bin/env python3
"""Overnight Operator Agent (Self-Sustaining Codex Session)

This is a long-running watchdog that starts ONE Codex CLI session with a single directive:
keep doing profitable work until the time budget expires.

Key goals (aligned with `long_running_codex_agent.md`):
- Self-sustaining: no deterministic checklist/task queue.
- Single Codex instance: one `codex exec` session for the whole run; the watchdog only
  resumes/restarts if the Codex process exits early.
- Always use model `gpt-5.3-codex` with reasoning effort `xhigh`.
- Reversible work: operate in a dedicated git worktree and create branches/commits.
- Ongoing summary: append to ops logs during the run.

Docs:
- Runbook: `docs/meta/automation/overnight_agent_runbook.md`
- Architecture: `docs/meta/automation/overnight_operator_agent_plan.md`
- Config: `docs/meta/automation/overnight_agent.yaml`
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import signal
import shutil
import subprocess
import sys
import textwrap
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

try:
    import yaml  # type: ignore
except Exception:  # pragma: no cover
    yaml = None


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CONFIG_PATH = ROOT / "docs" / "meta" / "automation" / "overnight_agent.yaml"

REQUIRED_MODEL = "gpt-5.3-codex"
REQUIRED_MODEL_REASONING_EFFORT = "xhigh"


class ConfigError(RuntimeError):
    pass


class GitError(RuntimeError):
    pass


def utc_now() -> dt.datetime:
    return dt.datetime.now(tz=dt.timezone.utc)


def iso_utc_now() -> str:
    return utc_now().isoformat(timespec="seconds")


def safe_mkdir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def cfg_path(repo_root: Path, value: str) -> Path:
    p = Path(value)
    return (repo_root / p).resolve() if not p.is_absolute() else p.resolve()


def cfg_get_str(cfg: dict[str, Any], key: str, default: str) -> str:
    value = cfg.get(key, default)
    if not isinstance(value, str):
        raise ConfigError(f"Expected {key!r} to be a string")
    return value


def cfg_get_bool(cfg: dict[str, Any], key: str, default: bool) -> bool:
    value = cfg.get(key, default)
    if not isinstance(value, bool):
        raise ConfigError(f"Expected {key!r} to be a bool")
    return value


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


def run_git(
    repo_root: Path,
    args: list[str],
    *,
    capture: bool = True,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
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


def ensure_git_identity(repo_root: Path, *, name: str, email: str) -> None:
    got_name = run_git(repo_root, ["config", "--get", "user.name"], check=False).stdout.strip()
    got_email = run_git(repo_root, ["config", "--get", "user.email"], check=False).stdout.strip()
    if not got_name:
        run_git(repo_root, ["config", "user.name", name], check=True)
    if not got_email:
        run_git(repo_root, ["config", "user.email", email], check=True)


def cfg_get_git_identity(cfg: dict[str, Any]) -> tuple[str, str]:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        return ("Overnight Agent", "overnight-agent@localhost")
    ident = repo_cfg.get("git_identity", {})
    if not isinstance(ident, dict):
        ident = {}
    name = cfg_get_str(ident, "name", "Overnight Agent")
    email = cfg_get_str(ident, "email", "overnight-agent@localhost")
    return (name, email)


def ensure_branch_exists(repo_root: Path, branch: str, base_branch: str) -> None:
    res = run_git(repo_root, ["show-ref", "--verify", "--quiet", f"refs/heads/{branch}"], capture=False, check=False)
    if res.returncode == 0:
        return
    run_git(repo_root, ["branch", branch, base_branch], capture=True, check=True)


def ensure_worktree(repo_root: Path, *, base_branch: str, branch: str, worktree_path: Path) -> None:
    safe_mkdir(worktree_path.parent)
    if worktree_path.exists():
        run_git(worktree_path, ["rev-parse", "--is-inside-work-tree"], capture=True, check=True)
        run_git(worktree_path, ["checkout", branch], capture=True, check=True)
        return

    branch_exists = (
        run_git(repo_root, ["show-ref", "--verify", "--quiet", f"refs/heads/{branch}"], capture=False, check=False).returncode
        == 0
    )
    if branch_exists:
        # Branch already exists: attach it to a new worktree (do NOT pass -b).
        run_git(repo_root, ["worktree", "add", str(worktree_path), branch], capture=True, check=True)
        return

    # Branch does not exist: create it from base_branch as part of adding the worktree.
    run_git(repo_root, ["worktree", "add", "-b", branch, str(worktree_path), base_branch], capture=True, check=True)


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
    f.write(f"pid={os.getpid()} started_utc={iso_utc_now()}\n")
    f.flush()
    return f


def _copy_if_missing(dst: Path, src: Path) -> None:
    if dst.exists() or not src.exists():
        return
    safe_mkdir(dst.parent)
    dst.write_bytes(src.read_bytes())
    try:
        dst.chmod(0o600)
    except Exception:
        pass


def prepare_codex_home(codex_home: Path) -> None:
    """Seed a writable CODEX_HOME (best-effort auth/config copy into tmp/).

    This copies secrets into tmp/ (gitignored). This is intentional to allow non-interactive runs.
    """

    safe_mkdir(codex_home)
    _copy_if_missing(codex_home / "auth.json", Path(os.path.expanduser("~/.codex/auth.json")))
    _copy_if_missing(codex_home / "config.toml", Path(os.path.expanduser("~/.codex/config.toml")))


@dataclass
class SessionState:
    run_id: str
    started_epoch_s: int
    started_utc: str
    deadline_epoch_s: int
    deadline_utc: str
    hours: float

    base_branch: str
    session_branch: str
    session_worktree: str

    ops_journal_branch: str
    ops_journal_worktree: str

    codex_home: str
    codex_bin: str
    codex_model: str
    codex_model_reasoning_effort: str
    codex_full_auto: bool
    codex_sandbox: str

    restarts: int = 0
    last_exit_code: int | None = None
    last_exit_utc: str | None = None
    codex_pid: int | None = None

    def remaining_s(self) -> int:
        return max(0, int(self.deadline_epoch_s - time.time()))


def session_state_path(cfg: dict[str, Any]) -> Path:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    state_s = cfg_get_str(repo_cfg, "session_state", "tmp/overnight_agent/session.json")
    return cfg_path(ROOT, state_s)


def load_session_state(path: Path) -> SessionState | None:
    if not path.exists():
        return None
    try:
        obj = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None
    if not isinstance(obj, dict):
        return None
    # Backwards compat for older session.json:
    # - v1 used `codex_model=gpt-5.3-codex-xhigh` (effort encoded in the model name).
    # - v2 stores base model + explicit `codex_model_reasoning_effort`.
    if "codex_model_reasoning_effort" not in obj:
        model = obj.get("codex_model")
        if isinstance(model, str) and model.endswith("-xhigh"):
            obj["codex_model"] = model[: -len("-xhigh")]
            obj["codex_model_reasoning_effort"] = "xhigh"
        else:
            obj["codex_model_reasoning_effort"] = REQUIRED_MODEL_REASONING_EFFORT
    try:
        return SessionState(**obj)
    except Exception:
        return None


def write_session_state(path: Path, state: SessionState) -> None:
    safe_mkdir(path.parent)
    payload = json.dumps(state.__dict__, indent=2, sort_keys=True)
    path.write_text(payload + "\n", encoding="utf-8", newline="\n")


def _sync_session_state_to_worktrees(cfg: dict[str, Any], *, state: SessionState) -> None:
    """
    The watchdog runs in the primary working tree, but the Codex agent runs in a separate git worktree.

    To make `python3 scripts/overnight_agent.py time-remaining` work *inside the session worktree*,
    mirror the session state file into:
    - <session_worktree>/<repo.session_state>
    - <ops_journal_worktree>/<repo.session_state>  (harmless; keeps tooling consistent)
    """
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        return
    state_s = cfg_get_str(repo_cfg, "session_state", "tmp/overnight_agent/session.json")
    p = Path(state_s)
    if p.is_absolute():
        return

    for root in (Path(state.session_worktree), Path(state.ops_journal_worktree)):
        if not root.exists():
            continue
        dst = (root / p).resolve()
        try:
            write_session_state(dst, state)
        except Exception:
            # Best-effort: mirroring should never crash the watchdog.
            pass


def new_session_state(cfg: dict[str, Any], *, hours: float) -> SessionState:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    base_branch = cfg_get_str(repo_cfg, "base_branch", "main")
    worktree_root_s = cfg_get_str(repo_cfg, "session_worktree_root", "../excel_diff_worktrees/overnight_session")
    ops_branch = cfg_get_str(repo_cfg, "ops_journal_branch", "overnight/ops-journal")
    ops_worktree_s = cfg_get_str(repo_cfg, "ops_journal_worktree", "../excel_diff_worktrees/overnight_ops_journal")

    codex_cfg = cfg.get("codex", {})
    if not isinstance(codex_cfg, dict):
        raise ConfigError("codex must be a mapping")
    codex_bin = cfg_get_str(codex_cfg, "codex_bin", "codex")
    codex_home_s = cfg_get_str(codex_cfg, "codex_home", "tmp/overnight_agent/codex_home")
    codex_model = cfg_get_str(codex_cfg, "model", REQUIRED_MODEL).strip()
    codex_reasoning_effort = cfg_get_str(codex_cfg, "model_reasoning_effort", REQUIRED_MODEL_REASONING_EFFORT).strip()
    codex_full_auto = cfg_get_bool(codex_cfg, "full_auto", True)
    codex_sandbox = cfg_get_str(codex_cfg, "sandbox", "workspace-write")

    if codex_model != REQUIRED_MODEL:
        raise ConfigError(f"codex.model must be {REQUIRED_MODEL!r} (got {codex_model!r})")
    if codex_reasoning_effort != REQUIRED_MODEL_REASONING_EFFORT:
        raise ConfigError(
            f"codex.model_reasoning_effort must be {REQUIRED_MODEL_REASONING_EFFORT!r} (got {codex_reasoning_effort!r})"
        )

    started = int(time.time())
    deadline = int(started + float(hours) * 3600.0)
    run_id = utc_now().strftime("%Y-%m-%d_%H%M%S")

    session_branch = f"overnight/session_{run_id}"
    worktree_root = cfg_path(ROOT, worktree_root_s)
    session_worktree = (worktree_root / run_id).resolve()

    ops_worktree = cfg_path(ROOT, ops_worktree_s)

    return SessionState(
        run_id=run_id,
        started_epoch_s=started,
        started_utc=dt.datetime.fromtimestamp(started, tz=dt.timezone.utc).isoformat(timespec="seconds"),
        deadline_epoch_s=deadline,
        deadline_utc=dt.datetime.fromtimestamp(deadline, tz=dt.timezone.utc).isoformat(timespec="seconds"),
        hours=float(hours),
        base_branch=base_branch,
        session_branch=session_branch,
        session_worktree=str(session_worktree),
        ops_journal_branch=ops_branch,
        ops_journal_worktree=str(ops_worktree),
        codex_home=str(cfg_path(ROOT, codex_home_s)),
        codex_bin=codex_bin,
        codex_model=codex_model,
        codex_model_reasoning_effort=codex_reasoning_effort,
        codex_full_auto=codex_full_auto,
        codex_sandbox=codex_sandbox,
    )


def ensure_session_worktrees(cfg: dict[str, Any], state: SessionState) -> None:
    ensure_worktree(
        ROOT,
        base_branch=state.base_branch,
        branch=state.session_branch,
        worktree_path=Path(state.session_worktree),
    )
    ensure_branch_exists(ROOT, state.ops_journal_branch, state.base_branch)
    ensure_worktree(
        ROOT,
        base_branch=state.base_branch,
        branch=state.ops_journal_branch,
        worktree_path=Path(state.ops_journal_worktree),
    )

    name, email = cfg_get_git_identity(cfg)
    # Best-effort: don't override operator identity; only set if missing.
    try:
        ensure_git_identity(Path(state.session_worktree), name=name, email=email)
    except Exception:
        pass
    try:
        ensure_git_identity(Path(state.ops_journal_worktree), name=name, email=email)
    except Exception:
        pass


def _format_hms(seconds: int) -> str:
    s = max(0, int(seconds))
    h = s // 3600
    m = (s % 3600) // 60
    sec = s % 60
    return f"{h:02d}:{m:02d}:{sec:02d}"


def build_initial_prompt(*, cfg: dict[str, Any], config_path: Path, state: SessionState) -> str:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        repo_cfg = {}
    exec_log = cfg_get_str(repo_cfg, "exec_summary_log", "docs/meta/logs/ops/executive_summary.log")
    reports_dir = cfg_get_str(repo_cfg, "reports_dir", "docs/meta/logs/ops")

    return textwrap.dedent(
        f"""\
        You are the Tabulensis overnight operator agent running non-interactively inside Codex CLI.

        SINGLE DIRECTIVE:
        Keep doing profitable work until the time limit is up (deadline UTC: {state.deadline_utc}).

        Reliable time remaining:
        - Run: `python3 scripts/overnight_agent.py time-remaining`
        - Stop starting new risky work when remaining time < 20 minutes; focus on wrap-up, docs, and logs.

        Hard constraints (do not violate):
        - No deploys. No secret rotation. No destructive git operations (no reset --hard, no force push).
        - Avoid wide-scope churn (especially formatting). Prefer the smallest valid change.
        - Follow perf policy in `AGENTS.md` (full perf cycle only for major perf-risk changes).
        - Do not request or use secrets.

        Git / rollback constraints:
        - You are operating in a dedicated git worktree: `{state.session_worktree}` (do not edit the primary working tree).
        - For each substantial work item, create a new branch under `overnight/` and commit.
          Example:
            - `git fetch origin`
            - `git switch -c overnight/<YYYY-MM-DD_HHMM>_<short_slug> origin/{state.base_branch}`
            - implement + validate + commit
        - Keep each branch reviewable; if you are unsure, prefer docs-only or small fixes.

        Ongoing summary (append throughout the run):
        - Use the ops journal worktree: `{state.ops_journal_worktree}` on branch `{state.ops_journal_branch}`.
        - Append 1 line per completed work item to `{exec_log}`:
          Format: `<ISO_UTC_TIMESTAMP> <branch> <commit_sha_or_n/a> <1-3 sentence summary>`
          Preferred helper (auto-commits on ops journal branch):
            - `python3 scripts/overnight_agent.py ops-log --branch <branch> --commit <sha_or_n/a> --message \"<summary>\"`
        - If you need operator decisions, append questions to:
          `{reports_dir}/<YYYY-MM-DD>_questions_for_operator.md`

        Boot sequence (do this first, in order):
        1) Documentation audit:
           - Scan for waste/contradictions and fix low-risk issues.
           - Run `python3 scripts/docs_integrity.py` (and `--check-links` if it’s fast).
        2) Strategy plan:
           - Read `product_roadmap.md`, `meta_methodology.md`, and `docs/index.md`.
           - Decide the next 1-3 most valuable tasks for this run given remaining time.
           - Write a short plan to `{reports_dir}/{state.run_id}_strategy.md` (commit it on the ops journal branch).
        3) Execution loop:
           - Repeatedly pick the next most valuable task, implement it, validate appropriately, commit, and append an exec summary line.

        Notes:
        - The most valuable work is not always code: fixing contradictions and tightening operating docs is often high ROI.
        - Feature ideas should come from `product_roadmap.md` when appropriate.
        - Keep logs and artifacts readable for a morning review.
        """
    )


def build_resume_prompt(*, config_path: Path, state: SessionState) -> str:
    return textwrap.dedent(
        f"""\
        Resume the overnight run.
        Deadline UTC: {state.deadline_utc}
        Remaining time: run `python3 scripts/overnight_agent.py time-remaining`
        Continue the execution loop: pick the most profitable next task and keep going until the deadline.
        """
    )


def _pid_is_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    # Best-effort sanity check to reduce PID-reuse footguns (Linux/WSL).
    try:
        cmdline = (Path(f"/proc/{pid}/cmdline")).read_bytes()
        if cmdline and b"codex" not in cmdline:
            return False
    except Exception:
        pass
    return True


def _terminate_pid(pid: int, *, grace_s: int) -> None:
    if pid <= 0:
        return
    try:
        os.kill(pid, signal.SIGTERM)
    except Exception:
        return
    t0 = time.time()
    while time.time() - t0 < float(grace_s):
        if not _pid_is_alive(pid):
            return
        time.sleep(1)
    try:
        os.kill(pid, signal.SIGKILL)
    except Exception:
        pass


def run_codex_process(
    *,
    state: SessionState,
    run_dir: Path,
    prompt: str,
    resume: bool,
    kill_at_epoch_s: int,
    kill_grace_s: int,
    on_pid: Callable[[int | None], None] | None = None,
) -> int:
    if shutil.which(state.codex_bin) is None:
        raise ConfigError(f"Codex CLI not found on PATH: {state.codex_bin!r}")

    codex_home = Path(state.codex_home)
    prepare_codex_home(codex_home)

    env = dict(os.environ)
    env["CODEX_HOME"] = str(codex_home)

    safe_mkdir(run_dir)
    log_path = run_dir / ("codex_resume.log" if resume else "codex_start.log")

    effort_cfg = f'model_reasoning_effort="{state.codex_model_reasoning_effort}"'

    if resume:
        cmd = [state.codex_bin, "exec", "resume", "--last", "--model", state.codex_model, "--config", effort_cfg]
        if state.codex_full_auto:
            cmd.append("--full-auto")
        cmd.append("-")
    else:
        cmd = [
            state.codex_bin,
            "exec",
            "--model",
            state.codex_model,
            "--config",
            effort_cfg,
            "--cd",
            state.session_worktree,
            "--add-dir",
            state.ops_journal_worktree,
            "--sandbox",
            state.codex_sandbox,
        ]
        if state.codex_full_auto:
            cmd.append("--full-auto")
        cmd.append("-")

    started = time.time()
    with log_path.open("a", encoding="utf-8", errors="replace") as log_f:
        log_f.write(f"\n# {iso_utc_now()} cmd: {' '.join(cmd)}\n")
        log_f.flush()
        proc = subprocess.Popen(
            cmd,
            cwd=ROOT,
            env=env,
            stdin=subprocess.PIPE,
            stdout=log_f,
            stderr=log_f,
            text=True,
        )
        assert proc.stdin is not None
        proc.stdin.write(prompt)
        proc.stdin.flush()
        proc.stdin.close()
        if on_pid is not None:
            try:
                on_pid(int(proc.pid))
            except Exception:
                pass

        def _terminate_child(reason: str) -> int:
            log_f.write(f"\n# {iso_utc_now()} terminating_codex reason={reason}\n")
            log_f.flush()
            try:
                proc.terminate()
            except Exception:
                pass
            t0 = time.time()
            while time.time() - t0 < float(kill_grace_s):
                rc2 = proc.poll()
                if rc2 is not None:
                    return int(rc2)
                time.sleep(1)
            log_f.write(f"\n# {iso_utc_now()} terminate_grace_expired: sending SIGKILL\n")
            log_f.flush()
            try:
                proc.kill()
            except Exception:
                pass
            try:
                return int(proc.wait(timeout=30))
            except Exception:
                return 137

        try:
            while True:
                rc = proc.poll()
                if rc is not None:
                    dur = time.time() - started
                    log_f.write(f"\n# {iso_utc_now()} exit={rc} duration_s={dur:.1f}\n")
                    log_f.flush()
                    return int(rc)

                if time.time() >= float(kill_at_epoch_s):
                    log_f.write(f"\n# {iso_utc_now()} deadline_reached: sending SIGTERM\n")
                    log_f.flush()
                    return _terminate_child("deadline")

                time.sleep(2)
        except KeyboardInterrupt:
            # Ctrl+C should not leave an orphaned Codex process running.
            _terminate_child("keyboard_interrupt")
            raise
        finally:
            if on_pid is not None:
                try:
                    on_pid(None)
                except Exception:
                    pass


def cmd_doctor(cfg: dict[str, Any], *, config_path: Path) -> int:
    print(f"repo_root: {ROOT}")
    print(f"python: {sys.version.split()[0]}")
    print(f"PyYAML: {'ok' if yaml is not None else 'missing'}")

    codex_cfg = cfg.get("codex", {})
    codex_bin = "codex"
    codex_model = REQUIRED_MODEL
    codex_reasoning_effort = REQUIRED_MODEL_REASONING_EFFORT
    if isinstance(codex_cfg, dict):
        codex_bin = cfg_get_str(codex_cfg, "codex_bin", codex_bin)
        codex_model = cfg_get_str(codex_cfg, "model", codex_model)
        codex_reasoning_effort = cfg_get_str(codex_cfg, "model_reasoning_effort", codex_reasoning_effort)

    print(f"codex.bin: {codex_bin}")
    print(f"codex.model: {codex_model}")
    print(f"codex.model_reasoning_effort: {codex_reasoning_effort}")

    try:
        head = run_git(ROOT, ["rev-parse", "HEAD"]).stdout.strip()
        print(f"git HEAD: {head}")
    except Exception as exc:
        print(f"git: ERROR ({exc})")
        return 2

    if shutil.which(codex_bin) is None:
        print(f"codex: ERROR (not found on PATH: {codex_bin!r})")
        return 2

    try:
        ver = subprocess.run([codex_bin, "--version"], capture_output=True, text=True, check=False).stdout.strip()
        if ver:
            print(f"codex: {ver}")
    except Exception:
        pass

    if codex_model != REQUIRED_MODEL:
        print(f"codex.model: ERROR (must be {REQUIRED_MODEL!r})")
        return 2
    if codex_reasoning_effort != REQUIRED_MODEL_REASONING_EFFORT:
        print(f"codex.model_reasoning_effort: ERROR (must be {REQUIRED_MODEL_REASONING_EFFORT!r})")
        return 2

    st_path = session_state_path(cfg)
    try:
        safe_mkdir(st_path.parent)
        tmp = st_path.parent / ".write_test"
        tmp.write_text("ok\n", encoding="utf-8")
        tmp.unlink(missing_ok=True)
        print(f"state: ok ({st_path})")
    except Exception as exc:
        print(f"state: ERROR ({st_path}): {exc}")
        return 2

    # smoke check prompt building (this is what the agent uses as its directive)
    _ = build_initial_prompt(cfg=cfg, config_path=config_path, state=new_session_state(cfg, hours=0.001))

    return 0


def cmd_time_remaining(cfg: dict[str, Any]) -> int:
    st_path = session_state_path(cfg)
    st = load_session_state(st_path)
    if st is None:
        print("No active session state found.")
        return 2
    rem = st.remaining_s()
    print(
        json.dumps(
            {
                "run_id": st.run_id,
                "now_utc": iso_utc_now(),
                "deadline_utc": st.deadline_utc,
                "remaining_s": rem,
                "remaining_hms": _format_hms(rem),
            },
            sort_keys=True,
        )
    )
    return 0


def cmd_ops_log(cfg: dict[str, Any], *, branch: str, commit: str, message: str) -> int:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    exec_log_rel = cfg_get_str(repo_cfg, "exec_summary_log", "docs/meta/logs/ops/executive_summary.log")

    st_path = session_state_path(cfg)
    st = load_session_state(st_path)
    if st is None:
        print("No active session state found (cannot locate ops journal worktree).", file=sys.stderr)
        return 2

    ops_wt = Path(st.ops_journal_worktree)
    if not ops_wt.exists():
        print(f"Ops journal worktree does not exist: {ops_wt}", file=sys.stderr)
        return 2

    # Ensure we're on the correct branch before modifying/committing.
    run_git(ops_wt, ["checkout", st.ops_journal_branch], capture=True, check=True)

    name, email = cfg_get_git_identity(cfg)
    try:
        ensure_git_identity(ops_wt, name=name, email=email)
    except Exception:
        pass

    msg = " ".join((message or "").replace("\r", " ").splitlines()).strip()
    if not msg:
        print("Empty --message (nothing to log).", file=sys.stderr)
        return 2

    line = f"{iso_utc_now()} {branch or 'n/a'} {commit or 'n/a'} {msg}\n"
    exec_log_path = ops_wt / exec_log_rel
    safe_mkdir(exec_log_path.parent)
    with exec_log_path.open("a", encoding="utf-8") as f:
        f.write(line)

    run_git(ops_wt, ["add", "-A", "--", exec_log_rel], capture=True, check=True)
    if run_git(ops_wt, ["diff", "--cached", "--quiet"], capture=False, check=False).returncode == 0:
        return 0
    run_git(ops_wt, ["commit", "-m", f"ops: exec summary ({st.run_id})"], capture=True, check=True)
    return 0


def supervise(
    cfg: dict[str, Any],
    *,
    config_path: Path,
    hours: float,
    resume: bool,
    restart_backoff_s: int,
) -> int:
    repo_cfg = cfg.get("repo", {})
    if not isinstance(repo_cfg, dict):
        raise ConfigError("repo must be a mapping")
    run_root_s = cfg_get_str(repo_cfg, "run_root", "tmp/overnight_agent")
    run_root = cfg_path(ROOT, run_root_s)
    lock_path = run_root / "lock"

    st_path = session_state_path(cfg)

    lock_f = acquire_lock(lock_path)
    try:
        loaded: SessionState | None = load_session_state(st_path) if resume else None
        existing_session = loaded is not None and int(time.time()) < int(loaded.deadline_epoch_s)
        st: SessionState
        if not existing_session:
            st = new_session_state(cfg, hours=hours)
            write_session_state(st_path, st)
        else:
            st = loaded  # type: ignore[assignment]

        ensure_session_worktrees(cfg, st)
        _sync_session_state_to_worktrees(cfg, state=st)

        # If the watchdog was restarted but a previous Codex process is still running, do not start a second one.
        if st.codex_pid is not None and _pid_is_alive(int(st.codex_pid)):
            while int(time.time()) < int(st.deadline_epoch_s) and _pid_is_alive(int(st.codex_pid)):
                time.sleep(5)
            if int(time.time()) >= int(st.deadline_epoch_s) and _pid_is_alive(int(st.codex_pid)):
                _terminate_pid(int(st.codex_pid), grace_s=120)
            if st.codex_pid is not None and not _pid_is_alive(int(st.codex_pid)):
                st.codex_pid = None
                write_session_state(st_path, st)
                _sync_session_state_to_worktrees(cfg, state=st)

        run_dir = run_root / "runs" / st.run_id
        safe_mkdir(run_dir)

        first = not existing_session

        def _persist_pid(pid: int | None) -> None:
            st.codex_pid = pid
            write_session_state(st_path, st)
            _sync_session_state_to_worktrees(cfg, state=st)

        try:
            while int(time.time()) < int(st.deadline_epoch_s):
                if st.remaining_s() <= 0:
                    break

                want_resume = not first
                prompt = (
                    build_initial_prompt(cfg=cfg, config_path=config_path, state=st)
                    if first
                    else build_resume_prompt(config_path=config_path, state=st)
                )

                # Primary attempt: resume after the first run. If resume fails (e.g., no session found),
                # fall back to starting a new session with the full directive.
                if want_resume:
                    rc = run_codex_process(
                        state=st,
                        run_dir=run_dir,
                        prompt=prompt,
                        resume=True,
                        kill_at_epoch_s=st.deadline_epoch_s,
                        kill_grace_s=120,
                        on_pid=_persist_pid,
                    )
                    if rc != 0:
                        rc = run_codex_process(
                            state=st,
                            run_dir=run_dir,
                            prompt=build_initial_prompt(cfg=cfg, config_path=config_path, state=st),
                            resume=False,
                            kill_at_epoch_s=st.deadline_epoch_s,
                            kill_grace_s=120,
                            on_pid=_persist_pid,
                        )
                else:
                    rc = run_codex_process(
                        state=st,
                        run_dir=run_dir,
                        prompt=prompt,
                        resume=False,
                        kill_at_epoch_s=st.deadline_epoch_s,
                        kill_grace_s=120,
                        on_pid=_persist_pid,
                    )

                first = False

                st.last_exit_code = int(rc)
                st.last_exit_utc = iso_utc_now()

                if int(time.time()) >= int(st.deadline_epoch_s):
                    break

                # Codex exited early: restart loop.
                st.restarts += 1
                write_session_state(st_path, st)
                _sync_session_state_to_worktrees(cfg, state=st)
                time.sleep(max(1, int(restart_backoff_s)))

            write_session_state(st_path, st)
            _sync_session_state_to_worktrees(cfg, state=st)
            return 0
        except KeyboardInterrupt:
            write_session_state(st_path, st)
            _sync_session_state_to_worktrees(cfg, state=st)
            return 130
    finally:
        try:
            lock_f.close()
        except Exception:
            pass


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run the Tabulensis overnight operator agent (self-sustaining).")
    parser.add_argument(
        "--config",
        type=str,
        default=str(DEFAULT_CONFIG_PATH),
        help=f"Path to config YAML (default: {DEFAULT_CONFIG_PATH})",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("doctor", help="Check environment + config wiring")
    sub.add_parser("time-remaining", help="Print remaining time for the active overnight session (JSON)")
    p_ops = sub.add_parser("ops-log", help="Append + commit one executive summary line on the ops journal branch")
    p_ops.add_argument("--branch", type=str, default="n/a")
    p_ops.add_argument("--commit", type=str, default="n/a")
    p_ops.add_argument("--message", type=str, required=True)

    p_sup = sub.add_parser("supervise", help="Start one Codex session and supervise it until the time budget expires")
    p_sup.add_argument("--hours", type=float, default=10.0)
    p_sup.add_argument("--no-resume", action="store_true", help="Ignore any existing session.json; start a new session")
    p_sup.add_argument("--restart-backoff-seconds", type=int, default=10)

    args = parser.parse_args(argv)
    cfg_path0 = cfg_path(ROOT, args.config)
    cfg = load_yaml(cfg_path0)

    if args.cmd == "doctor":
        return cmd_doctor(cfg, config_path=cfg_path0)
    if args.cmd == "time-remaining":
        return cmd_time_remaining(cfg)
    if args.cmd == "ops-log":
        return cmd_ops_log(cfg, branch=str(args.branch), commit=str(args.commit), message=str(args.message))
    if args.cmd == "supervise":
        return supervise(
            cfg,
            config_path=cfg_path0,
            hours=float(args.hours),
            resume=not bool(args.no_resume),
            restart_backoff_s=int(args.restart_backoff_seconds),
        )
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
