#!/usr/bin/env python3
"""
Copy the repo's deep research prompt to the clipboard for pasting into ChatGPT.

Usage:
  python3 scripts/deep_research_prompt.py          # copy to clipboard (default)
  python3 scripts/deep_research_prompt.py --print  # print to stdout
  python3 scripts/deep_research_prompt.py --prompt docs/meta/prompts/deep_research_ops_dashboard_apis.md
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from datetime import date, datetime
from pathlib import Path


PROMPT_PATH = Path("docs/meta/prompts/deep_research_market_analysis.md")


def _is_wsl() -> bool:
    # https://learn.microsoft.com/en-us/windows/wsl/
    if os.environ.get("WSL_DISTRO_NAME"):
        return True
    rel = platform.release().lower()
    return "microsoft" in rel or "wsl" in rel


def _run_clipboard_command(cmd: list[str], text: str) -> bool:
    try:
        subprocess.run(cmd, input=text.encode("utf-8"), check=True)
        return True
    except Exception:
        return False


def copy_to_clipboard(text: str) -> bool:
    # Prefer Windows clipboard when running under WSL (ChatGPT is typically in Windows browser).
    if _is_wsl():
        clip = shutil.which("clip.exe") or shutil.which("clip")
        if clip and _run_clipboard_command([clip], text):
            return True
        pwsh = shutil.which("powershell.exe")
        if pwsh:
            return _run_clipboard_command([pwsh, "-NoProfile", "-Command", "Set-Clipboard"], text)

    system = platform.system()

    if system == "Windows":
        clip = shutil.which("clip.exe") or shutil.which("clip")
        if clip:
            return _run_clipboard_command([clip], text)
        pwsh = shutil.which("powershell") or shutil.which("powershell.exe")
        if pwsh:
            return _run_clipboard_command([pwsh, "-NoProfile", "-Command", "Set-Clipboard"], text)

    if system == "Darwin":
        pbcopy = shutil.which("pbcopy")
        if pbcopy:
            return _run_clipboard_command([pbcopy], text)

    # Linux: try Wayland first, then X11.
    wl = shutil.which("wl-copy")
    if wl:
        return _run_clipboard_command([wl], text)
    xclip = shutil.which("xclip")
    if xclip:
        return _run_clipboard_command([xclip, "-selection", "clipboard"], text)
    xsel = shutil.which("xsel")
    if xsel:
        return _run_clipboard_command([xsel, "--clipboard", "--input"], text)

    return False


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Copy the deep research prompt to clipboard.")
    parser.add_argument("--print", action="store_true", help="Print the prompt to stdout instead of copying.")
    parser.add_argument(
        "--prompt",
        type=str,
        default=str(PROMPT_PATH),
        help=(
            "Prompt file path (default: docs/meta/prompts/deep_research_market_analysis.md). "
            "You can pass either a repo-relative path or a filename under docs/meta/prompts/."
        ),
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent
    candidate = Path(args.prompt)
    if not candidate.is_absolute():
        # Accept either a repo-relative path or just a filename living under docs/meta/prompts/.
        prompt_file = (repo_root / candidate) if (repo_root / candidate).exists() else (repo_root / "docs/meta/prompts" / candidate)
    else:
        prompt_file = candidate
    if not prompt_file.exists():
        print(f"Prompt file not found: {prompt_file}", file=sys.stderr)
        return 2

    text = prompt_file.read_text(encoding="utf-8", errors="replace")
    text = text.replace("{{RUN_DATE}}", date.today().isoformat())
    text = text.replace("{{RUN_DATETIME}}", datetime.now().isoformat(timespec="seconds"))

    if args.print:
        sys.stdout.write(text)
        return 0

    ok = copy_to_clipboard(text)
    if ok:
        print(f"Copied deep research prompt to clipboard ({len(text):,} chars).")
        print("Paste into ChatGPT Deep research.")
        return 0

    backup_dir = repo_root / "tmp"
    backup_dir.mkdir(parents=True, exist_ok=True)
    backup_path = backup_dir / "deep_research_prompt_clipboard_backup.txt"
    backup_path.write_text(text, encoding="utf-8", newline="\n")
    print("Clipboard copy failed.", file=sys.stderr)
    print(f"Wrote backup to: {backup_path}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
