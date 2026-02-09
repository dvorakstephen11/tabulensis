#!/usr/bin/env python3
"""
Create today's daily log from a template (file-based when present, otherwise built-in).

Creates:
  docs/meta/logs/daily/YYYY-MM-DD.md

Rules:
  - Refuses to overwrite an existing file.
  - Prints the created path to stdout (repo-relative when possible).
  - Optionally copies the created path to clipboard for quick paste.
  - Optionally opens the created file in your editor (best-effort).
"""

from __future__ import annotations

import argparse
import os
import platform
import shlex
import shutil
import subprocess
import sys
from datetime import date
from pathlib import Path


DEFAULT_TEMPLATE = """# Daily Log

## Date

## Top goals (today)

## Outputs produced (files/links)

## Decisions made

## Risks noticed

## Next actions
"""


def _format_path(repo_root: Path, path: Path) -> str:
    try:
        rel = path.resolve().relative_to(repo_root.resolve())
        return rel.as_posix()
    except Exception:
        return str(path)


def _print_path(repo_root: Path, path: Path) -> str:
    s = _format_path(repo_root, path)
    sys.stdout.write(s + "\n")
    return s


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


def _run_detached(cmd: list[str]) -> bool:
    try:
        subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True
    except Exception:
        return False


def open_in_editor(path: Path) -> bool:
    # Prefer explicit editor configuration.
    editor = os.environ.get("VISUAL") or os.environ.get("EDITOR")
    if editor:
        try:
            cmd = shlex.split(editor, posix=(os.name != "nt"))
        except Exception:
            cmd = [editor]
        if cmd:
            try:
                subprocess.run(cmd + [str(path)], check=False)
                return True
            except FileNotFoundError:
                pass
            except Exception:
                pass

    # Try Windows desktop integration from WSL when available.
    if _is_wsl():
        wslview = shutil.which("wslview")
        if wslview and _run_detached([wslview, str(path)]):
            return True
        explorer = shutil.which("explorer.exe")
        wslpath = shutil.which("wslpath")
        if explorer and wslpath:
            try:
                win_path = subprocess.check_output([wslpath, "-w", str(path)], text=True).strip()
            except Exception:
                win_path = ""
            if win_path and _run_detached([explorer, win_path]):
                return True

    system = platform.system()

    if system == "Windows":
        try:
            os.startfile(str(path))  # type: ignore[attr-defined]
            return True
        except Exception:
            pass
        cmd = shutil.which("cmd.exe") or shutil.which("cmd")
        if cmd and _run_detached([cmd, "/c", "start", "", str(path)]):
            return True
        return False

    if system == "Darwin":
        opener = shutil.which("open")
        if opener:
            return _run_detached([opener, str(path)])
        return False

    opener = shutil.which("xdg-open")
    if opener:
        return _run_detached([opener, str(path)])

    return False


def copy_to_clipboard(text: str) -> bool:
    # Prefer Windows clipboard when running under WSL (often pasting into Windows apps).
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
    parser = argparse.ArgumentParser(description="Create today's daily log from template.")
    parser.add_argument(
        "--copy-path",
        action="store_true",
        help="Copy the created path to clipboard (best-effort; still prints path to stdout).",
    )
    parser.add_argument(
        "--open",
        action="store_true",
        help="Open the created file in your editor (best-effort; prefers $VISUAL/$EDITOR).",
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent
    template_path = repo_root / "docs/meta/logs/daily/_TEMPLATE.md"
    template_text = DEFAULT_TEMPLATE
    if template_path.exists():
        template_text = template_path.read_text(encoding="utf-8", errors="replace")
        template_text = template_text.replace("\r\n", "\n").replace("\r", "\n")

    out_dir = repo_root / "docs/meta/logs/daily"
    out_dir.mkdir(parents=True, exist_ok=True)

    yyyy_mm_dd = date.today().strftime("%Y-%m-%d")
    out_path = out_dir / f"{yyyy_mm_dd}.md"

    try:
        # Use 'x' to make overwrite refusal atomic.
        with out_path.open("x", encoding="utf-8", newline="\n") as f:
            f.write(template_text)
    except FileExistsError:
        sys.stderr.write(f"Refusing to overwrite existing file: {out_path}\n")
        return 1

    created_path_str = _print_path(repo_root, out_path)
    if args.copy_path:
        ok = copy_to_clipboard(created_path_str)
        if not ok and sys.stderr.isatty():
            sys.stderr.write(
                "Warning: clipboard copy failed. Install wl-copy/xclip/xsel (Linux) or use WSL/Windows clipboard.\n"
            )
        elif ok and sys.stderr.isatty():
            sys.stderr.write("Copied path to clipboard.\n")
    if args.open:
        ok = open_in_editor(out_path.resolve())
        if not ok and sys.stderr.isatty():
            sys.stderr.write("Warning: could not open file. Set $VISUAL/$EDITOR or install xdg-open/open.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
