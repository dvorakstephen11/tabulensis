#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage:
  scripts/agent_worktree.sh <name> [--base-branch <branch>]
  scripts/agent_worktree.sh --cleanup [--older-than-days <n>] [--dry-run] [--force]

Create/ensure a dedicated git worktree for agentic work:
- Path:   ../excel_diff_worktrees/<name>
- Branch: agent_wt/<name>  (created from --base-branch, default: main)

The command prints the worktree path on success.

Cleanup:
--cleanup removes worktrees whose branch is under refs/heads/agent_wt/* and whose
worktree directory mtime is older than N days (default: 30). Dirty worktrees are
skipped unless --force is set. Use --dry-run to preview.
EOF
}

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${ROOT}" ]]; then
  die "must run inside a git repository"
fi

BASE_BRANCH="main"
MODE="create"
NAME=""
OLDER_THAN_DAYS=30
DRY_RUN=0
FORCE=0

if [[ "${#}" -eq 0 ]]; then
  usage
  exit 2
fi

if [[ "${1}" == "--help" || "${1}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "${1}" == "--cleanup" ]]; then
  MODE="cleanup"
  shift
else
  NAME="${1}"
  shift
fi

while [[ "${#}" -gt 0 ]]; do
  case "${1}" in
    --base-branch)
      shift
      [[ "${#}" -gt 0 ]] || die "--base-branch requires an argument"
      BASE_BRANCH="${1}"
      shift
      ;;
    --older-than-days)
      shift
      [[ "${#}" -gt 0 ]] || die "--older-than-days requires an argument"
      OLDER_THAN_DAYS="${1}"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --force)
      FORCE=1
      shift
      ;;
    *)
      die "unknown arg: ${1} (try --help)"
      ;;
  esac
done

WT_BASE="${ROOT}/../excel_diff_worktrees"

realpath_compat() {
  if command -v realpath >/dev/null 2>&1; then
    realpath -m "${1}"
    return 0
  fi
  python3 - <<'PY' "${1}"
import os, sys
print(os.path.abspath(sys.argv[1]))
PY
}

stat_mtime_epoch() {
  # Linux (GNU stat) / macOS (BSD stat) compatibility.
  if stat -c %Y "${1}" >/dev/null 2>&1; then
    stat -c %Y "${1}"
    return 0
  fi
  stat -f %m "${1}"
}

if [[ "${MODE}" == "create" ]]; then
  [[ -n "${NAME}" ]] || die "name is required"
  if [[ ! "${NAME}" =~ ^[A-Za-z0-9_-]+$ ]]; then
    die "invalid name '${NAME}' (use only A-Z a-z 0-9 _ -)"
  fi

  BRANCH="agent_wt/${NAME}"
  WT_PATH="${WT_BASE}/${NAME}"

  mkdir -p "${WT_BASE}"

  if [[ -e "${WT_PATH}" ]]; then
    git -C "${WT_PATH}" rev-parse --is-inside-work-tree >/dev/null 2>&1 \
      || die "path exists but is not a git worktree: ${WT_PATH}"
    realpath_compat "${WT_PATH}"
    exit 0
  fi

  if git -C "${ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH}"; then
    git -C "${ROOT}" worktree add "${WT_PATH}" "${BRANCH}" >/dev/null
  else
    git -C "${ROOT}" worktree add -b "${BRANCH}" "${WT_PATH}" "${BASE_BRANCH}" >/dev/null
  fi

  realpath_compat "${WT_PATH}"
  exit 0
fi

if [[ "${MODE}" == "cleanup" ]]; then
  now="$(date +%s)"
  cutoff="$(( now - (OLDER_THAN_DAYS * 86400) ))"

  wt_base_abs="$(realpath_compat "${WT_BASE}")"

  cur_path=""
  cur_branch=""
  cur_locked=0

  remove_count=0
  skip_count=0

  flush_entry() {
    [[ -n "${cur_path}" ]] || return 0

    local path="${cur_path}"
    local branch="${cur_branch}"
    local locked="${cur_locked}"

    cur_path=""
    cur_branch=""
    cur_locked=0

    [[ "${branch}" == "refs/heads/agent_wt/"* ]] || return 0

    # Only touch worktrees under our declared worktree base.
    local path_abs
    path_abs="$(realpath_compat "${path}")"
    [[ "${path_abs}" == "${wt_base_abs}/"* ]] || return 0

    if [[ "${locked}" -eq 1 ]]; then
      echo "skip (locked): ${path_abs}" >&2
      skip_count=$((skip_count + 1))
      return 0
    fi

    if [[ ! -d "${path_abs}" ]]; then
      # If the dir vanished, prune metadata and move on.
      git -C "${ROOT}" worktree prune >/dev/null || true
      return 0
    fi

    local mtime
    mtime="$(stat_mtime_epoch "${path_abs}")"
    if [[ "${mtime}" -gt "${cutoff}" ]]; then
      return 0
    fi

    if [[ "${FORCE}" -ne 1 ]]; then
      if [[ -n "$(git -C "${path_abs}" status --porcelain)" ]]; then
        echo "skip (dirty): ${path_abs}" >&2
        skip_count=$((skip_count + 1))
        return 0
      fi
    fi

    if [[ "${DRY_RUN}" -eq 1 ]]; then
      echo "dry-run remove: ${path_abs}" >&2
      remove_count=$((remove_count + 1))
      return 0
    fi

    if [[ "${FORCE}" -eq 1 ]]; then
      git -C "${ROOT}" worktree remove --force "${path_abs}" >/dev/null
    else
      git -C "${ROOT}" worktree remove "${path_abs}" >/dev/null
    fi
    remove_count=$((remove_count + 1))
  }

  while IFS= read -r line; do
    if [[ -z "${line}" ]]; then
      flush_entry
      continue
    fi
    case "${line}" in
      worktree\ *)
        cur_path="${line#worktree }"
        ;;
      branch\ *)
        cur_branch="${line#branch }"
        ;;
      locked*)
        cur_locked=1
        ;;
    esac
  done < <(git -C "${ROOT}" worktree list --porcelain)
  flush_entry

  if [[ "${DRY_RUN}" -ne 1 ]]; then
    git -C "${ROOT}" worktree prune >/dev/null || true
  fi

  echo "cleanup done: removed=${remove_count} skipped=${skip_count} older_than_days=${OLDER_THAN_DAYS} dry_run=${DRY_RUN} force=${FORCE}" >&2
  exit 0
fi

die "internal error: unknown mode '${MODE}'"

