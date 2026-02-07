import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


def run(cmd: list[str], env: dict | None = None, cwd: str | Path | None = None) -> None:
    print(f"+ {' '.join(shlex.quote(part) for part in cmd)}", flush=True)
    subprocess.run(cmd, check=True, env=env, cwd=cwd)


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def _ensure_fixture_deps(python: str) -> None:
    try:
        run([python, "-c", "import yaml, openpyxl, lxml, jinja2"])
    except subprocess.CalledProcessError as exc:
        print(
            "\nFixture generator dependencies are missing from this Python environment.\n"
            "Fix (recommended):\n"
            "  cd fixtures && uv sync\n"
            "Or (pip):\n"
            "  python -m pip install -r fixtures/requirements.txt\n",
            file=sys.stderr,
            flush=True,
        )
        raise exc


def run_generate_fixtures(python: str, args_repo_root: list[str], args_fixtures_root: list[str]) -> None:
    fixtures_dir = _repo_root() / "fixtures"
    if shutil.which("generate-fixtures"):
        run(["generate-fixtures", *args_repo_root])
        return

    # Prefer uv when available because it uses the fixture generator's pinned deps.
    if shutil.which("uv") and (fixtures_dir / "pyproject.toml").exists():
        run(["uv", "run", "generate-fixtures", *args_fixtures_root], cwd=fixtures_dir)
        return

    # Last resort: run the generator directly with the current Python.
    _ensure_fixture_deps(python)
    run([python, "fixtures/src/generate.py", *args_repo_root])


def main() -> int:
    os.chdir(_repo_root())
    python = sys.executable
    try:
        run([python, "scripts/check_fixture_references.py"])

        run_generate_fixtures(
            python,
            args_repo_root=["--manifest", "fixtures/manifest_cli_tests.yaml", "--force", "--clean"],
            args_fixtures_root=["--manifest", "manifest_cli_tests.yaml", "--force", "--clean"],
        )
        run_generate_fixtures(
            python,
            args_repo_root=[
                "--manifest",
                "fixtures/manifest_cli_tests.yaml",
                "--verify-lock",
                "fixtures/manifest_cli_tests.lock.json",
            ],
            args_fixtures_root=[
                "--manifest",
                "manifest_cli_tests.yaml",
                "--verify-lock",
                "manifest_cli_tests.lock.json",
            ],
        )

        test_env = dict(**os.environ, TABULENSIS_LICENSE_SKIP="1")
        run(["cargo", "test", "--workspace"], env=test_env)
    except FileNotFoundError as exc:
        print(f"Error: command not found: {exc.filename}", file=sys.stderr, flush=True)
        return 127
    except subprocess.CalledProcessError as exc:
        return exc.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
