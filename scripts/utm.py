#!/usr/bin/env python3
"""
Generate a URL with UTM parameters for marketing attribution.

Example:
  python3 scripts/utm.py --base-url https://tabulensis.com/download \\
    --source x --medium social --campaign demo_short --content clip_01 --copy
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
from urllib.parse import parse_qsl, urlencode, urlparse, urlunparse


_UTM_VALUE_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$")


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


def _parse_utm_value(value: str) -> str:
    v = (value or "").strip()
    if not v:
        raise argparse.ArgumentTypeError("must be non-empty")
    if not _UTM_VALUE_RE.fullmatch(v):
        raise argparse.ArgumentTypeError("must match [A-Za-z0-9][A-Za-z0-9_-]{0,63} (no spaces)")
    return v


def _parse_utm_value_optional(value: str) -> str:
    v = (value or "").strip()
    if not v:
        return ""
    return _parse_utm_value(v)


def with_utms(
    *,
    base_url: str,
    source: str,
    medium: str,
    campaign: str,
    content: str,
) -> str:
    parsed = urlparse(base_url)
    q = dict(parse_qsl(parsed.query, keep_blank_values=True))
    q["utm_source"] = source
    q["utm_medium"] = medium
    q["utm_campaign"] = campaign
    if content:
        q["utm_content"] = content
    else:
        q.pop("utm_content", None)
    new_query = urlencode(q, doseq=True)
    return urlunparse(parsed._replace(query=new_query))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Generate a URL with UTM parameters.")
    parser.add_argument(
        "--base-url",
        default="https://tabulensis.com/download",
        help="Base URL to append UTMs to (default: https://tabulensis.com/download).",
    )
    parser.add_argument("--source", required=True, type=_parse_utm_value, help="utm_source (e.g., x, linkedin, reddit).")
    parser.add_argument(
        "--medium",
        required=True,
        type=_parse_utm_value,
        help="utm_medium (e.g., social, community, email, ad).",
    )
    parser.add_argument(
        "--campaign",
        required=True,
        type=_parse_utm_value,
        help="utm_campaign (e.g., demo_short, before_after, user_story).",
    )
    parser.add_argument(
        "--content",
        default="",
        type=str,
        help="utm_content (optional; variant id).",
    )
    parser.add_argument("--copy", action="store_true", help="Copy the generated URL to clipboard (best-effort).")
    args = parser.parse_args(argv)

    try:
        content = _parse_utm_value_optional(str(args.content))
        url = with_utms(
            base_url=str(args.base_url).strip(),
            source=str(args.source).strip(),
            medium=str(args.medium).strip(),
            campaign=str(args.campaign).strip(),
            content=content,
        )
    except Exception as e:
        print(f"error: failed to build URL: {e}", file=sys.stderr)
        return 2

    print(url)
    if args.copy:
        ok = copy_to_clipboard(url)
        if not ok and sys.stderr.isatty():
            print(
                "Warning: clipboard copy failed. Install wl-copy/xclip/xsel (Linux) or use WSL/Windows clipboard.",
                file=sys.stderr,
            )
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
