#!/usr/bin/env python3
"""
Generate a TTS audio file from a markdown/text input file.

This is intended for turning operator docs (for example deep research syntheses)
into audio you can listen to while away from the screen.

Usage:
  python3 scripts/tts_generate.py path/to/input.md
  python3 scripts/tts_generate.py path/to/input.md --out /tmp/input.wav
  python3 scripts/tts_generate.py path/to/input.md --format mp3 --voice default --rate 1.1
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path


DEFAULT_MAX_CHARS = 20_000
DEFAULT_OUTPUT_DIR = Path("docs/meta/audio")


def _is_wsl() -> bool:
    # https://learn.microsoft.com/en-us/windows/wsl/
    if os.environ.get("WSL_DISTRO_NAME"):
        return True
    rel = platform.release().lower()
    return "microsoft" in rel or "wsl" in rel


def _sanitize_slug(s: str) -> str:
    s = s.strip().lower()
    s = re.sub(r"[^a-z0-9_-]+", "_", s)
    s = re.sub(r"_+", "_", s).strip("_")
    return s or "audio"


def _guess_ext(fmt: str) -> str:
    if fmt == "mp3":
        return ".mp3"
    return ".wav"


def _detect_default_backend() -> str | None:
    if _is_wsl() and shutil.which("powershell.exe"):
        return "powershell_wsl"
    system = platform.system()
    if system == "Windows" and (shutil.which("powershell") or shutil.which("powershell.exe")):
        return "powershell"
    if system == "Darwin" and shutil.which("say"):
        return "say"
    if shutil.which("espeak-ng") or shutil.which("espeak"):
        return "espeak"
    return None


def _read_input_text(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="replace")
    # Normalize line endings to keep downstream tooling consistent.
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _strip_markdown(text: str) -> str:
    # Keep this intentionally simple: remove the most noisy constructs without
    # pulling in dependencies.
    text = re.sub(r"^---\s*\n.*?\n---\s*\n", "", text, flags=re.DOTALL)  # frontmatter
    text = re.sub(r"```.*?```", "", text, flags=re.DOTALL)  # fenced code blocks
    text = re.sub(r"`([^`]+)`", r"\1", text)  # inline code
    text = re.sub(r"!\[([^\]]*)\]\([^)]*\)", r"\1", text)  # images -> alt text
    text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)  # links -> label
    text = re.sub(r"^\s{0,3}#+\s*", "", text, flags=re.MULTILINE)  # headings
    text = re.sub(r"^\s*>\s?", "", text, flags=re.MULTILINE)  # blockquotes
    # Collapse excessive blank lines.
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip() + "\n"


def _apply_max_chars(text: str, max_chars: int) -> tuple[str, bool]:
    if max_chars <= 0:
        return text, False
    if len(text) <= max_chars:
        return text, False
    return text[:max_chars].rstrip() + "\n", True


def _rate_to_wpm(rate: float, *, base_wpm: int = 175, min_wpm: int = 80, max_wpm: int = 450) -> int:
    if rate <= 0:
        rate = 1.0
    wpm = int(round(base_wpm * rate))
    return max(min_wpm, min(max_wpm, wpm))


def _rate_to_windows_synth_rate(rate: float) -> int:
    # SpeechSynthesizer.Rate is an int in [-10, 10] where 0 is default.
    if rate <= 0:
        rate = 1.0
    v = int(round((rate - 1.0) * 10.0))
    return max(-10, min(10, v))


def _run(cmd: list[str], *, input_bytes: bytes | None = None) -> None:
    subprocess.run(cmd, input=input_bytes, check=True)


def _wsl_to_windows_path(path: Path) -> str:
    out = subprocess.check_output(["wslpath", "-w", str(path)], text=True)
    return out.strip()


def _ensure_parent_dir(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def _espeak_wav(*, text_path: Path, out_wav: Path, voice: str, rate: float) -> None:
    espeak = shutil.which("espeak-ng") or shutil.which("espeak")
    if not espeak:
        raise RuntimeError("espeak/espeak-ng not found")
    wpm = _rate_to_wpm(rate)
    cmd = [espeak, "-f", str(text_path), "-w", str(out_wav), "-s", str(wpm)]
    if voice and voice != "default":
        cmd.extend(["-v", voice])
    _run(cmd)


def _say_wav(*, text_path: Path, out_wav: Path, voice: str, rate: float) -> None:
    say = shutil.which("say")
    if not say:
        raise RuntimeError("say not found")
    wpm = _rate_to_wpm(rate)
    with tempfile.TemporaryDirectory(prefix="tts_generate_") as td:
        tmp_aiff = Path(td) / "out.aiff"
        cmd = [say, "-f", str(text_path), "-o", str(tmp_aiff), "-r", str(wpm)]
        if voice and voice != "default":
            cmd.extend(["-v", voice])
        _run(cmd)

        afconvert = shutil.which("afconvert")
        if afconvert:
            _run([afconvert, "-f", "WAVE", "-d", "LEI16@44100", "-o", str(out_wav), str(tmp_aiff)])
            return

        ffmpeg = shutil.which("ffmpeg")
        if not ffmpeg:
            raise RuntimeError("Need either afconvert (macOS) or ffmpeg to produce .wav from say output")
        _run([ffmpeg, "-y", "-hide_banner", "-loglevel", "error", "-i", str(tmp_aiff), str(out_wav)])


def _powershell_wav(
    *,
    text_path: Path,
    out_wav: Path,
    voice: str,
    rate: float,
    wsl_mode: bool,
) -> None:
    ps = shutil.which("powershell.exe") if wsl_mode else (shutil.which("powershell") or shutil.which("powershell.exe"))
    if not ps:
        raise RuntimeError("powershell not found")

    synth_rate = _rate_to_windows_synth_rate(rate)

    with tempfile.TemporaryDirectory(prefix="tts_generate_ps_") as td:
        ps1 = Path(td) / "tts_generate.ps1"
        ps1.write_text(
            "\n".join(
                [
                    "param(",
                    "  [Parameter(Mandatory=$true)][string]$InPath,",
                    "  [Parameter(Mandatory=$true)][string]$OutPath,",
                    "  [string]$Voice = 'default',",
                    "  [int]$Rate = 0",
                    ")",
                    "Set-StrictMode -Version Latest",
                    "$ErrorActionPreference = 'Stop'",
                    "Add-Type -AssemblyName System.Speech",
                    "$s = New-Object System.Speech.Synthesis.SpeechSynthesizer",
                    "if ($Voice -and $Voice -ne 'default') {",
                    "  try {",
                    "    $s.SelectVoice($Voice)",
                    "  } catch {",
                    "    $voices = $s.GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Name }",
                    "    Write-Error (\"Voice not found: {0}. Available: {1}\" -f $Voice, ($voices -join ', '))",
                    "    exit 2",
                    "  }",
                    "}",
                    "$s.Rate = $Rate",
                    "$text = Get-Content -Raw -Encoding UTF8 $InPath",
                    "$s.SetOutputToWaveFile($OutPath)",
                    "$s.Speak($text)",
                    "$s.Dispose()",
                ]
            )
            + "\n",
            encoding="utf-8",
            newline="\n",
        )

        in_arg = _wsl_to_windows_path(text_path) if wsl_mode else str(text_path)
        out_arg = _wsl_to_windows_path(out_wav) if wsl_mode else str(out_wav)
        ps1_arg = _wsl_to_windows_path(ps1) if wsl_mode else str(ps1)

        cmd = [
            ps,
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            ps1_arg,
            "-InPath",
            in_arg,
            "-OutPath",
            out_arg,
            "-Voice",
            voice or "default",
            "-Rate",
            str(synth_rate),
        ]
        _run(cmd)


def _convert_wav_to_mp3(*, wav_path: Path, mp3_path: Path) -> None:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg not found (required for mp3 output)")
    _run([ffmpeg, "-y", "-hide_banner", "-loglevel", "error", "-i", str(wav_path), str(mp3_path)])


def _print_path(repo_root: Path, path: Path) -> None:
    try:
        rel = path.resolve().relative_to(repo_root.resolve())
        sys.stdout.write(rel.as_posix() + "\n")
    except Exception:
        sys.stdout.write(str(path) + "\n")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Generate a TTS audio file from a markdown/text input file.")
    parser.add_argument("input", type=str, help="Path to input markdown/text file.")
    parser.add_argument(
        "--out",
        type=str,
        default="",
        help=(
            "Output path (.wav or .mp3). If omitted, writes to docs/meta/audio/ with a timestamped name "
            "(directory created if missing)."
        ),
    )
    parser.add_argument(
        "--format",
        choices=["wav", "mp3"],
        default="",
        help="Output format (overrides the --out extension when set). Default: infer from --out or use wav.",
    )
    parser.add_argument("--voice", type=str, default="default", help="Voice name (backend-specific).")
    parser.add_argument("--rate", type=float, default=1.0, help="Speed multiplier (1.0 = default).")
    parser.add_argument(
        "--max-chars",
        type=int,
        default=DEFAULT_MAX_CHARS,
        help=f"Max input characters to synthesize (default {DEFAULT_MAX_CHARS:,}; 0 disables truncation).",
    )
    parser.add_argument(
        "--backend",
        choices=["auto", "powershell_wsl", "powershell", "say", "espeak"],
        default="auto",
        help="TTS backend selection. Default: auto-detect.",
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parent.parent

    in_path = Path(args.input).expanduser()
    if not in_path.is_absolute():
        in_path = (repo_root / in_path).resolve()
    if not in_path.exists() or not in_path.is_file():
        print(f"Input file not found: {in_path}", file=sys.stderr)
        return 2

    raw = _read_input_text(in_path)
    cooked = _strip_markdown(raw) if in_path.suffix.lower() in {".md", ".markdown"} else raw
    orig_len = len(cooked)
    cooked, truncated = _apply_max_chars(cooked, int(args.max_chars))
    if truncated:
        print(
            f"Input truncated to --max-chars={int(args.max_chars):,} (original {orig_len:,} chars).",
            file=sys.stderr,
        )

    fmt = (args.format or "").strip().lower()
    out_path: Path
    if args.out.strip():
        out_path = Path(args.out).expanduser()
        if not out_path.is_absolute():
            out_path = (repo_root / out_path).resolve()
    else:
        out_dir = repo_root / DEFAULT_OUTPUT_DIR
        ts = datetime.now().strftime("%Y-%m-%d_%H%M%S")
        slug = _sanitize_slug(in_path.stem)
        if not fmt:
            fmt = "wav"
        out_path = out_dir / f"{ts}_{slug}{_guess_ext(fmt)}"

    if not fmt:
        ext = out_path.suffix.lower()
        fmt = "mp3" if ext == ".mp3" else "wav"
    if fmt not in {"wav", "mp3"}:
        print(f"Unsupported output format: {fmt!r}", file=sys.stderr)
        return 2

    expected_suffix = ".mp3" if fmt == "mp3" else ".wav"
    if out_path.suffix and out_path.suffix.lower() != expected_suffix:
        print(
            f"Output path extension {out_path.suffix!r} does not match --format {fmt!r}. "
            f"Use an {expected_suffix} filename or omit --format.",
            file=sys.stderr,
        )
        return 2
    if not out_path.suffix:
        out_path = out_path.with_suffix(expected_suffix)

    _ensure_parent_dir(out_path)

    backend = args.backend
    if backend == "auto":
        backend = _detect_default_backend() or ""
    if not backend:
        print("No supported TTS backend found.", file=sys.stderr)
        print("Install one of: espeak-ng (Linux), or run on macOS with say, or on Windows/WSL with PowerShell.", file=sys.stderr)
        return 2

    voice = (args.voice or "default").strip()
    rate = float(args.rate)

    with tempfile.TemporaryDirectory(prefix="tts_generate_") as td:
        td_path = Path(td)
        text_path = td_path / "input.txt"
        text_path.write_text(cooked, encoding="utf-8", newline="\n")

        if fmt == "wav":
            tmp_wav = out_path
        else:
            tmp_wav = td_path / "out.wav"

        try:
            if backend == "espeak":
                _espeak_wav(text_path=text_path, out_wav=tmp_wav, voice=voice, rate=rate)
            elif backend == "say":
                _say_wav(text_path=text_path, out_wav=tmp_wav, voice=voice, rate=rate)
            elif backend == "powershell_wsl":
                _powershell_wav(text_path=text_path, out_wav=tmp_wav, voice=voice, rate=rate, wsl_mode=True)
            elif backend == "powershell":
                _powershell_wav(text_path=text_path, out_wav=tmp_wav, voice=voice, rate=rate, wsl_mode=False)
            else:
                print(f"Unsupported backend: {backend!r}", file=sys.stderr)
                return 2
        except subprocess.CalledProcessError as e:
            print(f"TTS backend failed: {backend} (exit={e.returncode})", file=sys.stderr)
            return 1
        except Exception as e:
            print(f"TTS backend failed: {backend}: {e}", file=sys.stderr)
            return 1

        if fmt == "mp3":
            _convert_wav_to_mp3(wav_path=tmp_wav, mp3_path=out_path)

    _print_path(repo_root, out_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
