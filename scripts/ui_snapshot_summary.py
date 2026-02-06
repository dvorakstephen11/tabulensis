#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path


def load_json(path: Path):
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def parse_layout_sizes(log_text: str):
    m = re.search(r"Layout sizes: (.+)$", log_text, flags=re.MULTILINE)
    return m.group(1).strip() if m else None


def image_stats(path: Path):
    try:
        from PIL import Image
        import numpy as np
    except Exception:
        return None

    img = Image.open(path).convert("RGB")
    a = np.asarray(img)
    mean = a.mean(axis=(0, 1))
    std = a.std(axis=(0, 1))
    return {
        "size": f"{img.width}x{img.height}",
        "mean_rgb": [float(x) for x in mean],
        "std_rgb": [float(x) for x in std],
        "blank": bool(float(std.max()) < 1.0),
    }


def main() -> int:
    strict = False
    args = sys.argv[1:]
    if "--strict" in args:
        strict = True
        args = [a for a in args if a != "--strict"]

    if len(args) != 1 or args[0] in ("-h", "--help"):
        print("Usage: scripts/ui_snapshot_summary.py [--strict] <path/to/run.json>", file=sys.stderr)
        return 2

    meta_path = Path(args[0])
    if not meta_path.is_file():
        print(f"Not found: {meta_path}", file=sys.stderr)
        return 2

    meta = load_json(meta_path)
    tag = meta.get("tag")
    scenario = meta.get("scenario")
    print(f"scenario={scenario} tag={tag}")

    ready_path = meta.get("readyCopy") or meta.get("readyFile")
    if ready_path:
        ready_path = Path(ready_path)
    ready = load_json(ready_path) if ready_path and ready_path.is_file() else None
    if ready:
        print(
            "ready:"
            f" status={ready.get('status')}"
            f" reason={ready.get('reason')}"
            f" expected_mode={ready.get('expected_mode')}"
            f" actual_mode={ready.get('actual_mode')}"
            f" diff_id={ready.get('diff_id')}"
        )
        print(
            "ui:"
            f" root_tab={ready.get('root_tab')}"
            f" result_tab={ready.get('result_tab')}"
            f" selected_sheet={ready.get('selected_sheet')}"
            f" sheet_count={ready.get('sheet_count')}"
        )
        status_text = ready.get("status_text")
        if status_text:
            print(f"status_text={status_text}")
    else:
        print("ready: missing")

    log_path = meta.get("logFile")
    if log_path and Path(log_path).is_file():
        log_text = read_text(Path(log_path))
        layout = parse_layout_sizes(log_text)
        if layout:
            print(f"layout={layout}")
    else:
        print("log: missing")

    shot_path = meta_path.with_suffix(".png")
    if shot_path.is_file():
        stats = image_stats(shot_path)
        if stats:
            print(
                "image:"
                f" size={stats['size']}"
                f" blank={stats['blank']}"
                f" std_rgb={[round(x, 2) for x in stats['std_rgb']]}"
            )
    else:
        print("image: missing")

    if not strict:
        return 0

    problems = []
    if not ready:
        problems.append("ready missing")
    else:
        if ready.get("status") != "ok":
            problems.append(f"ready status={ready.get('status')}")
        if not ready.get("reason"):
            problems.append("ready reason missing")
    if not shot_path.is_file():
        problems.append("image missing")
    else:
        stats = image_stats(shot_path)
        if stats and stats.get("blank"):
            problems.append("image appears blank")

    if problems:
        print("strict_fail=" + "; ".join(problems), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
