#!/usr/bin/env python3
"""
Copy the repo's deep research prompt to the clipboard for pasting into ChatGPT.

Usage:
  python3 scripts/deep_research_prompt.py          # copy to clipboard (default)
  python3 scripts/deep_research_prompt.py --open   # also open ChatGPT in your browser
  python3 scripts/deep_research_prompt.py --list   # list available prompt short names
  python3 scripts/deep_research_prompt.py --new-result  # create a new results file + print the path
  python3 scripts/deep_research_prompt.py --new-a-b  # create A/B/synthesis result files + print paths
  python3 scripts/deep_research_prompt.py --new-result --audio  # also print a placeholder audio output path
  python3 scripts/deep_research_prompt.py --audio  # print a placeholder audio output path and exit
  python3 scripts/deep_research_prompt.py --new-result --copy-result-path  # also copy the path to clipboard
  python3 scripts/deep_research_prompt.py --new-result --open-result  # also open the result file (best-effort)
  python3 scripts/deep_research_prompt.py --print  # print to stdout
  python3 scripts/deep_research_prompt.py --prompt market_analysis
  python3 scripts/deep_research_prompt.py --prompt deep_research_ops_dashboard_apis.md
  python3 scripts/deep_research_prompt.py --prompt docs/meta/prompts/deep_research_ops_dashboard_apis.md
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import webbrowser
from datetime import date, datetime
from pathlib import Path


PROMPTS_DIR = Path("docs/meta/prompts")
PROMPT_PATH = PROMPTS_DIR / "deep_research_market_analysis.md"
RESULTS_DIR = Path("docs/meta/results")
AUDIO_DIR = Path("docs/meta/audio")
CHATGPT_URL = "https://chatgpt.com/"


def _sanitize_slug(s: str) -> str:
    # Keep filenames conservative/portable (ASCII-ish), and avoid surprises.
    s = s.strip().lower()
    s = re.sub(r"[^a-z0-9_-]+", "_", s)
    s = re.sub(r"_+", "_", s).strip("_")
    return s or "result"


def _prompt_to_topic_slug(prompt_file: Path) -> str:
    stem = prompt_file.stem
    if stem.startswith("deep_research_"):
        stem = stem.removeprefix("deep_research_")
    return _sanitize_slug(stem)


def _create_new_result_file(repo_root: Path, prompt_file: Path) -> Path:
    results_dir = repo_root / RESULTS_DIR
    results_dir.mkdir(parents=True, exist_ok=True)

    # Timestamp prefix: stable sort order; unique enough; collision-safe with a suffix.
    ts = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    topic = _prompt_to_topic_slug(prompt_file)

    for i in range(1000):
        suffix = "" if i == 0 else f"_{i + 1}"
        p = results_dir / f"{ts}_{topic}{suffix}.md"
        try:
            with p.open("x", encoding="utf-8", newline="\n"):
                pass
            return p
        except FileExistsError:
            continue

    raise RuntimeError(f"Failed to create a unique result file under: {results_dir}")


def _create_new_a_b_result_files(repo_root: Path, prompt_file: Path) -> tuple[Path, Path, Path]:
    results_dir = repo_root / RESULTS_DIR
    results_dir.mkdir(parents=True, exist_ok=True)

    # Timestamp prefix: stable sort order; unique enough; collision-safe with a suffix.
    ts = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    topic = _prompt_to_topic_slug(prompt_file)

    for i in range(1000):
        suffix = "" if i == 0 else f"_{i + 1}"
        base = f"{ts}_{topic}{suffix}"
        paths = [
            results_dir / f"{base}_a.md",
            results_dir / f"{base}_b.md",
            results_dir / f"{base}_synthesis.md",
        ]

        created: list[Path] = []
        try:
            for p in paths:
                with p.open("x", encoding="utf-8", newline="\n"):
                    pass
                created.append(p)
            return (paths[0], paths[1], paths[2])
        except FileExistsError:
            # Avoid leaving a partially-created set.
            for p in created:
                try:
                    p.unlink()
                except Exception:
                    pass
            continue

    raise RuntimeError(f"Failed to create unique A/B/synthesis result files under: {results_dir}")


def _placeholder_audio_path_for_result(repo_root: Path, result_path: Path) -> Path:
    # Placeholder path for future TTS output. We intentionally do not create the file here.
    return repo_root / AUDIO_DIR / f"{result_path.stem}.wav"


def _placeholder_audio_path_for_prompt(repo_root: Path, prompt_file: Path) -> Path:
    # Placeholder path for future TTS output. We intentionally do not create the file here.
    ts = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    topic = _prompt_to_topic_slug(prompt_file)
    return repo_root / AUDIO_DIR / f"{ts}_{topic}.wav"


def _list_prompt_short_names(repo_root: Path) -> list[str]:
    prompts_dir = repo_root / PROMPTS_DIR
    if not prompts_dir.exists():
        return []

    names: list[str] = []
    for p in prompts_dir.glob("deep_research_*.md"):
        stem = p.stem
        if not stem.startswith("deep_research_"):
            continue
        names.append(stem.removeprefix("deep_research_"))
    return sorted(set(names))


def _resolve_prompt_path(repo_root: Path, prompt_arg: str) -> Path | None:
    raw = Path(prompt_arg).expanduser()
    if raw.is_absolute():
        return raw if raw.exists() else None

    candidates: list[Path] = []
    seen: set[Path] = set()

    def add(p: Path) -> None:
        if p in seen:
            return
        candidates.append(p)
        seen.add(p)

    def add_repo_relative(p: Path) -> None:
        add(repo_root / p)
        if p.parent == Path("."):
            add(repo_root / PROMPTS_DIR / p)

    def add_variants(p: Path) -> None:
        add_repo_relative(p)
        if p.suffix == "":
            add_repo_relative(p.with_suffix(".md"))

    add_variants(raw)

    # Allow passing short names like "market_analysis" -> deep_research_market_analysis.md
    if not raw.name.startswith("deep_research_"):
        prefixed = raw.with_name(f"deep_research_{raw.name}")
        add_variants(prefixed)

    for p in candidates:
        if p.exists():
            return p
    return None


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


def _run_open_command(cmd: list[str]) -> bool:
    try:
        # Some openers (notably xdg-open in headless envs) can hang. Treat open as best-effort and
        # fail fast rather than blocking the caller.
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5)
        return True
    except Exception:
        return False


def open_in_browser_best_effort(url: str) -> bool:
    # Prefer Windows browser when running under WSL.
    if _is_wsl():
        wslview = shutil.which("wslview")
        if wslview and _run_open_command([wslview, url]):
            return True

        cmd = shutil.which("cmd.exe")
        if cmd and _run_open_command([cmd, "/c", "start", "", url]):
            return True

        pwsh = shutil.which("powershell.exe")
        if pwsh:
            # PowerShell single-quoted string escaping: '' inside ''.
            quoted = "'" + url.replace("'", "''") + "'"
            if _run_open_command([pwsh, "-NoProfile", "-Command", f"Start-Process {quoted}"]):
                return True

    if platform.system() == "Windows":
        try:
            os.startfile(url)  # type: ignore[attr-defined]
            return True
        except Exception:
            pass

    try:
        return bool(webbrowser.open(url, new=2, autoraise=True))
    except Exception:
        return False


def open_result_files_best_effort(paths: list[Path]) -> bool:
    # Intended for local/interactive use; must not crash in headless or CI contexts.
    if not paths:
        return False

    ok_all = True
    for p in paths:
        try:
            rp = p.resolve()
        except Exception:
            rp = p

        if not rp.exists():
            ok_all = False
            continue

        # Prefer Windows shell when running under WSL.
        if _is_wsl():
            wslview = shutil.which("wslview")
            if wslview and _run_open_command([wslview, str(rp)]):
                continue

            wslpath = shutil.which("wslpath")
            winpath: str | None = None
            if wslpath:
                try:
                    winpath = (
                        subprocess.check_output([wslpath, "-w", str(rp)], text=True, stderr=subprocess.DEVNULL)
                        .strip()
                        or None
                    )
                except Exception:
                    winpath = None

            cmd = shutil.which("cmd.exe")
            if cmd and winpath and _run_open_command([cmd, "/c", "start", "", winpath]):
                continue

            pwsh = shutil.which("powershell.exe")
            if pwsh and winpath:
                # PowerShell single-quoted string escaping: '' inside ''.
                quoted = "'" + winpath.replace("'", "''") + "'"
                if _run_open_command([pwsh, "-NoProfile", "-Command", f"Start-Process {quoted}"]):
                    continue

        system = platform.system()
        if system == "Windows":
            try:
                os.startfile(str(rp))  # type: ignore[attr-defined]
                continue
            except Exception:
                pass

        if system == "Darwin":
            opener = shutil.which("open")
            if opener and _run_open_command([opener, str(rp)]):
                continue

        xdg_open = shutil.which("xdg-open")
        if xdg_open and _run_open_command([xdg_open, str(rp)]):
            continue

        ok_all = False

    return ok_all


def maybe_open_chatgpt(should_open: bool) -> None:
    if not should_open:
        return
    ok = open_in_browser_best_effort(CHATGPT_URL)
    if ok:
        print("Opened ChatGPT in your browser.", file=sys.stderr)
    else:
        print(f"Could not open ChatGPT automatically. Please open: {CHATGPT_URL}", file=sys.stderr)


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
    parser.add_argument("--list", action="store_true", help="List available prompt short names and exit.")
    parser.add_argument(
        "--new-result",
        action="store_true",
        help="Create a new timestamped file under docs/meta/results/ and print its path.",
    )
    parser.add_argument(
        "--new-a-b",
        action="store_true",
        help=(
            "Create three new timestamped files under docs/meta/results/ (A/B/synthesis) and print their paths. "
            "Filenames end with _a.md, _b.md, and _synthesis.md."
        ),
    )
    parser.add_argument(
        "--audio",
        action="store_true",
        help=(
            "Print a placeholder audio output path under docs/meta/audio/. "
            "If used with --new-result/--new-a-b, prints the audio path(s) corresponding to the created result file(s)."
        ),
    )
    parser.add_argument(
        "--open-result",
        action="store_true",
        help="Open the created result file(s) in the default editor (best-effort). Requires --new-result or --new-a-b.",
    )
    parser.add_argument(
        "--copy-result-path",
        action="store_true",
        help=(
            "Copy the created result file path(s) to clipboard (best-effort; still prints paths to stdout). "
            "Requires --new-result or --new-a-b."
        ),
    )
    parser.add_argument("--open", action="store_true", help="Open ChatGPT in the default browser (best-effort).")
    parser.add_argument("--print", action="store_true", help="Print the prompt to stdout instead of copying.")
    parser.add_argument(
        "--prompt",
        type=str,
        default=str(PROMPT_PATH),
        help=(
            "Prompt file path (default: docs/meta/prompts/deep_research_market_analysis.md). "
            "You can pass a repo-relative path, a filename under docs/meta/prompts/, or a short name "
            "like 'market_analysis' (mapped to deep_research_market_analysis.md)."
        ),
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent

    if args.list:
        names = _list_prompt_short_names(repo_root)
        if not names:
            print(f"No prompts found under: {repo_root / PROMPTS_DIR}", file=sys.stderr)
            return 2
        for name in names:
            print(name)
        maybe_open_chatgpt(args.open)
        return 0

    if args.new_result and args.new_a_b:
        print("--new-result and --new-a-b are mutually exclusive.", file=sys.stderr)
        return 2
    if args.copy_result_path and not (args.new_result or args.new_a_b):
        print("--copy-result-path requires --new-result or --new-a-b.", file=sys.stderr)
        return 2
    if args.open_result and not (args.new_result or args.new_a_b):
        print("--open-result requires --new-result or --new-a-b.", file=sys.stderr)
        return 2

    prompt_file = _resolve_prompt_path(repo_root, args.prompt)
    if not prompt_file:
        print(f"Prompt file not found for --prompt {args.prompt!r}", file=sys.stderr)
        return 2

    if args.new_result or args.new_a_b:
        result_paths: list[Path]
        if args.new_result:
            result_paths = [_create_new_result_file(repo_root, prompt_file)]
        else:
            a_path, b_path, s_path = _create_new_a_b_result_files(repo_root, prompt_file)
            result_paths = [a_path, b_path, s_path]

        outs: list[str] = []
        for rp in result_paths:
            try:
                out = rp.relative_to(repo_root).as_posix()
            except Exception:
                out = str(rp)
            outs.append(out)
            print(out)

        if args.audio:
            for rp in result_paths:
                audio_path = _placeholder_audio_path_for_result(repo_root, rp)
                try:
                    audio_out = audio_path.relative_to(repo_root).as_posix()
                except Exception:
                    audio_out = str(audio_path)
                print(audio_out)

        if args.copy_result_path:
            ok = copy_to_clipboard("\n".join(outs))
            if not ok and sys.stderr.isatty():
                sys.stderr.write(
                    "Warning: clipboard copy failed. Install wl-copy/xclip/xsel (Linux) or use WSL/Windows clipboard.\n"
                )
            elif ok and sys.stderr.isatty():
                sys.stderr.write("Copied result path to clipboard.\n")
        if args.open_result:
            ok = open_result_files_best_effort(result_paths)
            if ok:
                print("Opened result file(s) in your editor.", file=sys.stderr)
            else:
                msg = "\n".join(outs)
                print(f"Could not open result file(s) automatically. Open manually:\n{msg}", file=sys.stderr)
        maybe_open_chatgpt(args.open)
        return 0

    if args.audio:
        audio_path = _placeholder_audio_path_for_prompt(repo_root, prompt_file)
        try:
            audio_out = audio_path.relative_to(repo_root).as_posix()
        except Exception:
            audio_out = str(audio_path)
        print(audio_out)
        maybe_open_chatgpt(args.open)
        return 0

    text = prompt_file.read_text(encoding="utf-8", errors="replace")
    text = text.replace("{{RUN_DATE}}", date.today().isoformat())
    text = text.replace("{{RUN_DATETIME}}", datetime.now().isoformat(timespec="seconds"))

    if args.print:
        sys.stdout.write(text)
        maybe_open_chatgpt(args.open)
        return 0

    ok = copy_to_clipboard(text)
    if ok:
        print(f"Copied deep research prompt to clipboard ({len(text):,} chars).")
        print("Paste into ChatGPT Deep research.")
        maybe_open_chatgpt(args.open)
        return 0

    backup_dir = repo_root / "tmp"
    backup_dir.mkdir(parents=True, exist_ok=True)
    backup_path = backup_dir / "deep_research_prompt_clipboard_backup.txt"
    backup_path.write_text(text, encoding="utf-8", newline="\n")
    print("Clipboard copy failed.", file=sys.stderr)
    print(f"Wrote backup to: {backup_path}", file=sys.stderr)
    maybe_open_chatgpt(args.open)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
