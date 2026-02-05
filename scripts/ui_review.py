#!/usr/bin/env python3
import argparse
import json
import os
import subprocess
from datetime import datetime
from pathlib import Path

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def read_png_size(path: Path):
    with path.open("rb") as f:
        header = f.read(24)
    if len(header) < 24 or header[:8] != PNG_SIGNATURE:
        raise ValueError(f"Not a PNG file: {path}")
    width = int.from_bytes(header[16:20], "big")
    height = int.from_bytes(header[20:24], "big")
    return width, height


def resolve_scenario_dir(name: str) -> Path | None:
    candidates = []
    root_env = os.environ.get("EXCEL_DIFF_UI_SCENARIOS_ROOT")
    if root_env:
        candidates.append(Path(root_env))

    cwd = Path.cwd()
    candidates.append(cwd / "desktop" / "ui_scenarios")
    candidates.append(Path(__file__).resolve().parents[1] / "desktop" / "ui_scenarios")

    for root in candidates:
        candidate = root / name
        if (candidate / "scenario.json").exists():
            return candidate
    return None


def load_scenario(name: str):
    scenario_dir = resolve_scenario_dir(name)
    if not scenario_dir:
        return None
    scenario_path = scenario_dir / "scenario.json"
    try:
        return json.loads(scenario_path.read_text())
    except Exception:
        return None


def run_external_review(cmd: str, payload: dict):
    try:
        result = subprocess.run(
            cmd,
            input=json.dumps(payload).encode("utf-8"),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            shell=True,
            check=False,
        )
    except Exception as exc:
        return None, f"Failed to run review command: {exc}"

    if result.returncode != 0:
        return None, result.stderr.decode("utf-8", errors="ignore")
    return result.stdout.decode("utf-8", errors="ignore"), None


def build_default_report(payload: dict) -> str:
    scenario = payload.get("scenario") or {}
    metrics = payload.get("metrics") or {}
    size = payload.get("size") or {}
    status = metrics.get("status", "unknown")

    lines = []
    lines.append(f"# UI Visual Review: {scenario.get('name', 'Scenario')}")
    lines.append("")
    lines.append("**Summary**")
    lines.append(f"- Status: `{status}`")
    lines.append(f"- Mismatch: `{metrics.get('mismatchPercent', 'n/a')}%` (threshold `{metrics.get('thresholdPercent', 'n/a')}%`) ")
    lines.append(f"- Image size: `{size.get('width', '?')}x{size.get('height', '?')}`")
    lines.append("")
    lines.append("**Scenario**")
    lines.append(f"- Name: `{scenario.get('name', 'unknown')}`")
    if scenario.get("description"):
        lines.append(f"- Description: {scenario['description']}")
    if scenario.get("focusPanel"):
        lines.append(f"- Focus panel: `{scenario['focusPanel']}`")
    expected_mode = payload.get("expected_mode") or scenario.get("expectMode")
    if expected_mode:
        lines.append(f"- Expected mode: `{expected_mode}`")
    if payload.get("actual_mode"):
        lines.append(f"- Actual mode: `{payload['actual_mode']}`")
    if payload.get("ready_status"):
        lines.append(f"- Ready status: `{payload['ready_status']}`")
    lines.append("")
    lines.append("**What This Screenshot Likely Shows**")
    lines.append("- Compare screen loaded with the scenario workbook pair.")
    if scenario.get("focusPanel") in ("summary", "details"):
        lines.append(f"- Result tab focused: `{scenario.get('focusPanel')}`.")
    lines.append("- Status bar and summary/detail text reflect the last diff run.")
    lines.append("")
    lines.append("**Quality Impact**")
    if status == "pass":
        lines.append("- No significant visual regression detected at the configured threshold.")
        lines.append("- If changes were intentional, consider updating the baseline to lock them in.")
    elif status == "fail":
        lines.append("- Visual regression likely. Review the diff image and confirm if the change is intended.")
        lines.append("- Check layout/spacing changes, missing text, or truncated controls.")
    else:
        lines.append("- Unable to determine regression status. Inspect metrics and diff output.")
    lines.append("")
    lines.append("**Relevant Code Areas**")
    lines.append("- `desktop/wx/ui/main.xrc` (layout + widget hierarchy)")
    lines.append("- `desktop/wx/src/main.rs` (widget wiring + state updates)")
    lines.append("- `desktop/backend/src/diff_runner.rs` (diff flow + summary data)")
    lines.append("")
    lines.append("**Suggested Follow-ups**")
    lines.append("- Open the diff image to spot the exact change regions.")
    lines.append("- Verify the change aligns with `docs/ui_guidelines.md`.")
    lines.append("- If intentional, update the baseline with `scripts/ui_diff.js --update-baseline`. ")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Generate a UI visual review report.")
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--metrics", default=None)
    parser.add_argument("--image", default=None)
    parser.add_argument("--diff", default=None)
    parser.add_argument("--out", default=None)
    args = parser.parse_args()

    scenario = load_scenario(args.scenario)
    if scenario is None:
        scenario = {"name": args.scenario}

    metrics = {}
    if args.metrics:
        try:
            metrics = json.loads(Path(args.metrics).read_text())
        except Exception:
            metrics = {}

    current_meta = {}
    if args.metrics:
        current_meta_path = Path(args.metrics).parent / "current.json"
        if current_meta_path.exists():
            try:
                current_meta = json.loads(current_meta_path.read_text())
            except Exception:
                current_meta = {}

    image_path = Path(args.image) if args.image else None
    size = {}
    if image_path and image_path.exists():
        try:
            width, height = read_png_size(image_path)
            size = {"width": width, "height": height}
        except Exception:
            size = {}

    payload = {
        "scenario": scenario,
        "metrics": metrics,
        "size": size,
        "image": str(image_path) if image_path else None,
        "diff": args.diff,
        "generatedAt": datetime.utcnow().isoformat() + "Z",
        "actual_mode": metrics.get("actual_mode"),
    }

    guidelines_path = Path.cwd() / "docs" / "ui_guidelines.md"
    if guidelines_path.exists():
        try:
            guidelines_text = guidelines_path.read_text()
            payload["ui_guidelines"] = guidelines_text[:5000]
        except Exception:
            pass

    ready_file = current_meta.get("readyFile")
    if ready_file and Path(ready_file).exists():
        try:
            ready_payload = json.loads(Path(ready_file).read_text())
            payload["actual_mode"] = ready_payload.get("actual_mode")
            payload["expected_mode"] = ready_payload.get("expected_mode")
            payload["ready_status"] = ready_payload.get("status")
        except Exception:
            pass

    review_cmd = os.environ.get("EXCEL_DIFF_UI_REVIEW_CMD")
    report = None
    error = None
    if review_cmd:
        report, error = run_external_review(review_cmd, payload)

    if not report:
        report = build_default_report(payload)
        if review_cmd and error:
            report += "\n\n---\n\n"
            report += f"External review command failed: {error.strip()}\n"
            report += "Using built-in heuristic report instead."

    out_path = args.out
    if not out_path:
        out_path = Path.cwd() / "desktop" / "ui_reports" / args.scenario
        out_path.mkdir(parents=True, exist_ok=True)
        out_path = out_path / f"{metrics.get('tag', 'latest')}.md"
    else:
        out_path = Path(out_path)
        out_path.parent.mkdir(parents=True, exist_ok=True)

    out_path.write_text(report)
    print(f"Wrote UI review report: {out_path}")


if __name__ == "__main__":
    main()
