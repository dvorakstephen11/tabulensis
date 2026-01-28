#!/usr/bin/env python3
"""
Package the wxDragon desktop app into an AppImage.

Requires linuxdeploy + appimagetool available in PATH.
"""

from __future__ import annotations

import argparse
import os
import struct
import zlib
import re
import shutil
import subprocess
from pathlib import Path


def run(cmd: list[str], cwd: Path | None = None, env: dict | None = None) -> None:
    print("+ " + " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True, cwd=cwd, env=env)


def read_version(cargo_toml: Path) -> str:
    text = cargo_toml.read_text(encoding="utf-8")
    match = re.search(r"^version\s*=\s*\"([^\"]+)\"", text, re.MULTILINE)
    if not match:
        raise ValueError(f"Failed to parse version from {cargo_toml}")
    return match.group(1)


def arch_from_target(target: str) -> str:
    mapping = {
        "x86_64-unknown-linux-gnu": "x86_64",
        "aarch64-unknown-linux-gnu": "aarch64",
        "x86_64-unknown-linux-musl": "x86_64",
        "aarch64-unknown-linux-musl": "aarch64",
    }
    return mapping.get(target, target)


def write_placeholder_icon(path: Path, size: int = 256) -> None:
    width = height = size
    rgba = bytes((0x1F, 0x3A, 0x44, 0xFF))
    raw = b"".join(b"\x00" + rgba * width for _ in range(height))
    ihdr = struct.pack("!IIBBBBB", width, height, 8, 6, 0, 0, 0)
    compressed = zlib.compress(raw, 9)

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack("!I", len(data))
            + tag
            + data
            + struct.pack("!I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    payload = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", compressed)
        + chunk(b"IEND", b"")
    )
    path.write_bytes(payload)


def main() -> int:
    parser = argparse.ArgumentParser(description="Package desktop_wx into an AppImage")
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument("--profile", default="release-desktop", help="Cargo profile name")
    parser.add_argument("--out-dir", default="target/dist", help="Output directory")
    parser.add_argument("--bin-name", default="desktop_wx", help="Binary name")
    parser.add_argument("--no-build", action="store_true", help="Skip cargo build")
    parser.add_argument("--no-locked", action="store_true", help="Build without --locked")
    parser.add_argument("--version", default=None, help="Override version string")
    parser.add_argument(
        "--icon",
        default=None,
        help="Optional icon path (PNG). Defaults to a generated placeholder.",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    version = args.version or read_version(repo_root / "desktop" / "wx" / "Cargo.toml")
    arch = arch_from_target(args.target)

    linuxdeploy = shutil.which("linuxdeploy")
    if not linuxdeploy:
        raise RuntimeError(
            "linuxdeploy not found in PATH. Install linuxdeploy + appimagetool before packaging."
        )

    if not args.no_build:
        cmd = [
            "cargo",
            "build",
            "-p",
            "desktop_wx",
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

    artifact_name = f"tabulensis-desktop-v{version}-linux-{arch}"
    appdir_root = repo_root / "target" / "appimage" / f"{artifact_name}.AppDir"

    if appdir_root.exists():
        shutil.rmtree(appdir_root)
    (appdir_root / "usr" / "bin").mkdir(parents=True)

    shutil.copy2(bin_path, appdir_root / "usr" / "bin" / args.bin_name)

    doc_dir = appdir_root / "usr" / "share" / "doc" / "tabulensis"
    doc_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(repo_root / "README.md", doc_dir / "README.md")
    shutil.copy2(repo_root / "LICENSE.txt", doc_dir / "LICENSE.txt")
    shutil.copy2(repo_root / "THIRD_PARTY_NOTICES.txt", doc_dir / "THIRD_PARTY_NOTICES.txt")

    desktop_entry = appdir_root / "tabulensis.desktop"
    desktop_entry.write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Tabulensis\n"
        f"Exec={args.bin_name}\n"
        "Icon=tabulensis\n"
        "Categories=Office;Utility;\n"
        "Terminal=false\n",
        encoding="utf-8",
    )

    icon_path = appdir_root / "tabulensis.png"
    if args.icon:
        icon_src = Path(args.icon)
        if not icon_src.exists():
            raise FileNotFoundError(f"Icon not found at {icon_src}")
        shutil.copy2(icon_src, icon_path)
    else:
        write_placeholder_icon(icon_path, size=256)

    work_dir = repo_root / "target" / "appimage" / "build"
    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True)

    env = os.environ.copy()
    env["ARCH"] = arch
    run(
        [
            linuxdeploy,
            "--appdir",
            str(appdir_root),
            "--desktop-file",
            str(desktop_entry),
            "--icon-file",
            str(appdir_root / "tabulensis.png"),
            "--output",
            "appimage",
        ],
        cwd=work_dir,
        env=env,
    )

    appimages = list(work_dir.glob("*.AppImage"))
    if not appimages:
        raise FileNotFoundError("No AppImage produced by linuxdeploy")

    appimage_path = out_dir / f"{artifact_name}.AppImage"
    shutil.move(str(appimages[0]), appimage_path)

    print(f"Created {appimage_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
