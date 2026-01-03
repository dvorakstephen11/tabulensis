import argparse
import subprocess
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def run_cmin(target: str, fuzz_dir: Path) -> None:
    subprocess.run(
        ["cargo", "fuzz", "cmin", target],
        cwd=fuzz_dir,
        check=True,
    )


def enforce_limits(
    target_dir: Path,
    max_file_bytes: int | None,
    max_files: int | None,
    max_total_bytes: int | None,
    dry_run: bool,
) -> dict:
    files = [p for p in target_dir.iterdir() if p.is_file()]
    files.sort(key=lambda p: (p.stat().st_size, p.name))

    removed = []
    total_bytes = sum(p.stat().st_size for p in files)

    if max_file_bytes is not None:
        for path in list(files):
            if path.stat().st_size > max_file_bytes:
                removed.append(path)
                files.remove(path)
                total_bytes -= path.stat().st_size
                if not dry_run:
                    path.unlink()

    if max_files is not None and len(files) > max_files:
        overflow = files[max_files:]
        for path in overflow:
            removed.append(path)
            total_bytes -= path.stat().st_size
            if not dry_run:
                path.unlink()
        files = files[:max_files]

    if max_total_bytes is not None and total_bytes > max_total_bytes:
        for path in reversed(files):
            if total_bytes <= max_total_bytes:
                break
            removed.append(path)
            total_bytes -= path.stat().st_size
            if not dry_run:
                path.unlink()

    return {
        "kept": len(files),
        "removed": len(removed),
        "total_bytes": total_bytes,
        "removed_files": [str(p) for p in removed],
    }


def main() -> int:
    root = repo_root()
    parser = argparse.ArgumentParser(description="Maintain fuzz corpora.")
    parser.add_argument(
        "--corpus-root",
        type=Path,
        default=root / "core" / "fuzz" / "corpus",
        help="Corpus root directory",
    )
    parser.add_argument(
        "--targets",
        type=str,
        default="",
        help="Comma-separated list of targets to process (default: all)",
    )
    parser.add_argument(
        "--skip-cmin",
        action="store_true",
        help="Skip cargo fuzz cmin",
    )
    parser.add_argument(
        "--max-file-bytes",
        type=int,
        default=2 * 1024 * 1024,
        help="Max size per seed file (default: 2MB)",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=200,
        help="Max number of files per corpus (default: 200)",
    )
    parser.add_argument(
        "--max-total-bytes",
        type=int,
        default=100 * 1024 * 1024,
        help="Max total bytes per corpus (default: 100MB)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Report actions without deleting files",
    )
    args = parser.parse_args()

    corpus_root = args.corpus_root
    if not corpus_root.exists():
        raise FileNotFoundError(f"Corpus root not found: {corpus_root}")

    if args.targets:
        targets = [t.strip() for t in args.targets.split(",") if t.strip()]
    else:
        targets = [p.name for p in corpus_root.iterdir() if p.is_dir()]

    fuzz_dir = root / "core" / "fuzz"
    report = []

    for target in targets:
        target_dir = corpus_root / target
        if not target_dir.exists():
            print(f"Skipping missing corpus target: {target}")
            continue

        if not args.skip_cmin:
            run_cmin(target, fuzz_dir)

        stats = enforce_limits(
            target_dir,
            args.max_file_bytes,
            args.max_files,
            args.max_total_bytes,
            args.dry_run,
        )
        stats["target"] = target
        report.append(stats)

    print("Fuzz corpus maintenance report:")
    for entry in report:
        print(
            f"- {entry['target']}: kept={entry['kept']} removed={entry['removed']} "
            f"total_bytes={entry['total_bytes']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
