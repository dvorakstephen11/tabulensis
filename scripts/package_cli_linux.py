#!/usr/bin/env python3
"""
Package the Linux CLI into a tar.gz for download.

Example:
  python scripts/package_cli_linux.py --target x86_64-unknown-linux-gnu
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
        "x86_64-unknown-linux-gnu": "x86_64",
        "aarch64-unknown-linux-gnu": "arm64",
        "x86_64-unknown-linux-musl": "x86_64",
        "aarch64-unknown-linux-musl": "arm64",
    }
    return mapping.get(target, target)


def main() -> int:
    parser = argparse.ArgumentParser(description="Package the Linux CLI tar.gz")
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument("--profile", default="release-cli", help="Cargo profile name")
    parser.add_argument("--out-dir", default="target/dist", help="Output directory")
    parser.add_argument("--bin-name", default="tabulensis", help="Binary name")
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

    bin_path = (
        repo_root
        / "target"
        / args.target
        / args.profile
        / args.bin_name
    )
    if not bin_path.exists():
        raise FileNotFoundError(f"Binary not found at {bin_path}")

    out_dir = repo_root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    artifact_name = f"tabulensis-v{version}-linux-{arch}"
    stage_dir = out_dir / artifact_name

    if stage_dir.exists():
        shutil.rmtree(stage_dir)
    stage_dir.mkdir(parents=True)

    shutil.copy2(bin_path, stage_dir / args.bin_name)
    shutil.copy2(repo_root / "README.md", stage_dir / "README.md")

    tar_path = out_dir / f"{artifact_name}.tar.gz"
    if tar_path.exists():
        tar_path.unlink()

    import tarfile

    with tarfile.open(tar_path, "w:gz") as tar:
        tar.add(stage_dir, arcname=artifact_name)

    print(f"Created {tar_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
