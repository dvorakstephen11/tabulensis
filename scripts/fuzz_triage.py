import argparse
import subprocess
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def run(cmd: list[str], cwd: Path) -> None:
    print(" ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Reproduce and minimize fuzz artifacts.")
    parser.add_argument("--target", required=True, help="Fuzz target name")
    parser.add_argument("--artifact", required=True, help="Path to artifact file")
    parser.add_argument(
        "--cmin",
        action="store_true",
        help="Run corpus minimization after tmin",
    )
    args = parser.parse_args()

    fuzz_dir = repo_root() / "core" / "fuzz"
    artifact = Path(args.artifact).resolve()
    if not artifact.exists():
        raise FileNotFoundError(f"Artifact not found: {artifact}")

    run(["cargo", "fuzz", "run", args.target, str(artifact), "-runs=1"], fuzz_dir)
    run(["cargo", "fuzz", "tmin", args.target, str(artifact)], fuzz_dir)

    if args.cmin:
        run(["cargo", "fuzz", "cmin", args.target], fuzz_dir)

    print("Triage complete. See minimized artifact in core/fuzz/artifacts.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
