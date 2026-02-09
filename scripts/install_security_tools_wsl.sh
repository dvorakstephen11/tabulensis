#!/usr/bin/env bash
set -euo pipefail

# Installs the missing tools used by scripts/security_audit.sh:
# - cargo-audit (RustSec)
# - gitleaks
# - semgrep
#
# Intended to be run inside WSL (Ubuntu/Debian-like). Installs user-local binaries
# under ~/.local/bin and cargo installs under ~/.cargo/bin.

is_wsl() {
  [[ -n "${WSL_DISTRO_NAME:-}" ]] && return 0
  [[ -r /proc/version ]] && grep -qiE 'microsoft|wsl' /proc/version && return 0
  return 1
}

log() { printf '%s\n' "$*" >&2; }
die() { log "error: $*"; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

apt_install() {
  local pkgs=("$@")
  if command -v apt-get >/dev/null 2>&1; then
    if command -v sudo >/dev/null 2>&1 && [[ "$(id -u)" -ne 0 ]]; then
      sudo apt-get update
      sudo apt-get install -y "${pkgs[@]}"
    else
      apt-get update
      apt-get install -y "${pkgs[@]}"
    fi
    return 0
  fi
  die "apt-get not found. Install dependencies manually: ${pkgs[*]}"
}

ensure_path_hint() {
  local d="$1"
  if [[ ":$PATH:" != *":$d:"* ]]; then
    log
    log "PATH note: '$d' is not currently in PATH."
    log "Add this to ~/.bashrc (or ~/.zshrc) and restart your shell:"
    log "  export PATH=\"$d:\\$PATH\""
  fi
}

main() {
  if ! is_wsl; then
    log "warning: WSL not detected. Continuing anyway."
  fi

  need_cmd uname
  need_cmd mktemp
  need_cmd tar

  mkdir -p "$HOME/.local/bin"

  log "Installing prerequisites (python, pip, certs, etc.)..."
  apt_install ca-certificates curl python3 python3-pip python3-venv

  need_cmd python3
  need_cmd curl

#
## semgrep (pip user install)
#
  log
  log "Installing semgrep (user-local via pip)..."
  python3 -m pip install --user --upgrade pip >/dev/null
  python3 -m pip install --user --upgrade semgrep
  ensure_path_hint "$HOME/.local/bin"

#
## gitleaks (download latest GitHub release)
#
  log
  if command -v gitleaks >/dev/null 2>&1; then
    log "gitleaks already installed: $(gitleaks version | head -n 1)"
  else
    arch="$(uname -m)"
    case "$arch" in
      x86_64) asset_suffix="linux_x64.tar.gz" ;;
      aarch64|arm64) asset_suffix="linux_arm64.tar.gz" ;;
      *)
        die "unsupported arch for gitleaks installer: $arch"
        ;;
    esac

    log "Installing gitleaks (arch=$arch)..."
    tmpdir="$(mktemp -d -t install_security_tools.XXXXXX)"
    trap 'rm -rf "$tmpdir"' EXIT

    # Use python for robust JSON parsing without jq dependency.
    gitleaks_url="$(python3 - <<PY
import json, sys, urllib.request
api = "https://api.github.com/repos/gitleaks/gitleaks/releases/latest"
suffix = "${asset_suffix}"
req = urllib.request.Request(api, headers={"Accept":"application/vnd.github+json","User-Agent":"tabulensis-security-installer"})
with urllib.request.urlopen(req, timeout=30) as r:
    data = json.load(r)
for a in data.get("assets", []):
    url = a.get("browser_download_url", "")
    if url.endswith(suffix):
        print(url)
        sys.exit(0)
sys.stderr.write(f"no asset ending with {suffix} found in latest release\\n")
sys.exit(2)
PY
)"

    curl -fsSL "$gitleaks_url" -o "$tmpdir/gitleaks.tar.gz"
    tar -xzf "$tmpdir/gitleaks.tar.gz" -C "$tmpdir"
    gitleaks_bin="$(find "$tmpdir" -maxdepth 3 -type f -name gitleaks | head -n 1 || true)"
    [[ -n "${gitleaks_bin:-}" && -f "$gitleaks_bin" ]] || die "gitleaks binary not found after extract"
    install -m 0755 "$gitleaks_bin" "$HOME/.local/bin/gitleaks"
    log "Installed: $HOME/.local/bin/gitleaks"
  fi

#
## cargo-audit (cargo install)
#
  log
  if command -v cargo >/dev/null 2>&1; then
    log "Installing cargo-audit (RustSec) via cargo..."
    cargo install cargo-audit --locked
    ensure_path_hint "$HOME/.cargo/bin"
  else
    log "warning: cargo not found; skipping cargo-audit install."
    log "Install Rust first (rustup), then run:"
    log "  cargo install cargo-audit --locked"
  fi

  log
  log "Verification:"
  if command -v semgrep >/dev/null 2>&1; then
    log "- semgrep: $(semgrep --version | head -n 1)"
  else
    log "- semgrep: NOT FOUND (check ~/.local/bin in PATH)"
  fi
  if command -v gitleaks >/dev/null 2>&1; then
    log "- gitleaks: $(gitleaks version | head -n 1)"
  else
    log "- gitleaks: NOT FOUND (check ~/.local/bin in PATH)"
  fi
  if command -v cargo >/dev/null 2>&1 && command -v cargo-audit >/dev/null 2>&1; then
    log "- cargo-audit: $(cargo audit --version | head -n 1)"
  else
    log "- cargo-audit: NOT FOUND (check ~/.cargo/bin in PATH)"
  fi

  log
  log "Next: re-run the security report:"
  log "  bash scripts/security_audit.sh"
}

main "$@"
