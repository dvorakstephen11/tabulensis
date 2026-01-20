import shlex
import subprocess
import sys


def run(cmd: list[str]) -> None:
    print(f"+ {' '.join(shlex.quote(part) for part in cmd)}", flush=True)
    subprocess.run(cmd, check=True)


def main() -> int:
    python = sys.executable
    try:
        run([python, "scripts/check_fixture_references.py"])
        run(
            [
                "generate-fixtures",
                "--manifest",
                "fixtures/manifest_cli_tests.yaml",
                "--force",
                "--clean",
            ]
        )
        run(
            [
                "generate-fixtures",
                "--manifest",
                "fixtures/manifest_cli_tests.yaml",
                "--verify-lock",
                "fixtures/manifest_cli_tests.lock.json",
            ]
        )
        run(["cargo", "test", "--workspace"])
    except subprocess.CalledProcessError as exc:
        return exc.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
