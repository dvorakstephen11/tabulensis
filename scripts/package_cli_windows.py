#!/usr/bin/env python3
"""
Package the Windows CLI into a zip for download.
"""
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
from pathlib import Path


def run(cmd: list[str]) -> None:
    print("+ " + " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def read_version(cargo_toml: Path) -> str:
    text = cargo_toml.read_text(encoding="utf-8")
    match = re.search(r"^version\s*=\s*\"([^\"]+)\"", text, re.MULTILINE)
    if not match:
        raise ValueError(f"Failed to parse version from {cargo_toml}")
    return match.group(1)


def arch_from_target(target: str) -> str:
    mapping = {
        "x86_64-pc-windows-msvc": "x86_64",
        "aarch64-pc-windows-msvc": "arm64",
    }
    return mapping.get(target, target)


def main() -> int:
    parser = argparse.ArgumentParser(description="Package the Windows CLI zip")
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument("--profile", default="release-cli", help="Cargo profile name")
    parser.add_argument("--out-dir", default="target/dist", help="Output directory")
    parser.add_argument("--bin-name", default="tabulensis.exe", help="Binary name")
    parser.add_argument("--no-build", action="store_true", help="Skip cargo build")
    parser.add_argument("--no-locked", action="store_true", help="Build without --locked")
    parser.add_argument("--version", default=None, help="Override version string")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    version = args.version or read_version(repo_root / "cli" / "Cargo.toml")
    arch = arch_from_target(args.target)

    if not args.no_build:
        cmd = [
            "cargo",
            "build",
            "-p",
            "tabulensis-cli",
            "--profile",
            args.profile,
            "--target",
            args.target,
        ]
        if not args.no_locked:
            cmd.append("--locked")
        run(cmd)

    bin_path = repo_root / "target" / args.target / args.profile / args.bin_name
    if not bin_path.exists():
        raise FileNotFoundError(f"Binary not found at {bin_path}")

    out_dir = repo_root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    artifact_name = f"tabulensis-v{version}-windows-{arch}"
    stage_dir = out_dir / artifact_name

    if stage_dir.exists():
        shutil.rmtree(stage_dir)
    stage_dir.mkdir(parents=True)

    shutil.copy2(bin_path, stage_dir / args.bin_name)
    shutil.copy2(repo_root / "README.md", stage_dir / "README.md")
    shutil.copy2(repo_root / "LICENSE.txt", stage_dir / "LICENSE.txt")
    shutil.copy2(repo_root / "THIRD_PARTY_NOTICES.txt", stage_dir / "THIRD_PARTY_NOTICES.txt")

    zip_path = out_dir / f"{artifact_name}.zip"
    if zip_path.exists():
        zip_path.unlink()

    import zipfile

    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for item in stage_dir.iterdir():
            zf.write(item, arcname=f"{artifact_name}/{item.name}")

    print(f"Created {zip_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
