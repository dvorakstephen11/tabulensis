# Audio (TTS)

This directory is for generating Text-to-Speech (TTS) audio from operator docs
(for example deep research syntheses) so you can listen while away from the
screen.

## Generate Audio

From repo root:

```bash
python3 scripts/tts_generate.py docs/meta/results/YYYY-MM-DD_<topic>_synthesis.md
```

Notes:
- The script prints the output path to stdout.
- If you omit `--out`, audio is written to `docs/meta/audio/` with a timestamped
  name (directory created if missing).

Optional output formats/paths:

```bash
# Explicit output path
python3 scripts/tts_generate.py docs/meta/results/YYYY-MM-DD_<topic>_synthesis.md --out /tmp/<topic>.wav

# MP3 output (requires ffmpeg)
python3 scripts/tts_generate.py docs/meta/results/YYYY-MM-DD_<topic>_synthesis.md --format mp3
```

## Recommended Inputs

Prefer a synthesis/summary file, not raw output.

Good inputs:
- `docs/meta/results/YYYY-MM-DD_<topic>_synthesis.md`
- Short, curated markdown notes that are intended to be read aloud

Avoid:
- `..._a.md` / `..._b.md` raw deep-research outputs (usually too long/noisy)
- Long logs, code-heavy docs, or unedited transcripts

Tip: keep inputs short. By default, `scripts/tts_generate.py` truncates to
`--max-chars=20000` to prevent giant audio files.

## Output Location

Default output directory: `docs/meta/audio/`

Default file naming: `YYYY-MM-DD_HHMMSS_<slug>.wav` (or `.mp3` if `--format mp3`)

You can override the destination with `--out` (for example write to `/tmp/`).

Note:
- Binary audio outputs (`*.wav`, `*.mp3`) are gitignored by default; only this README is intended to be committed.
