import argparse
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path
from typing import Iterable, Sequence

INCLUDED_EXTENSIONS = {".rs", ".py", ".toml", ".yaml", ".yml", ".gitignore", ".js", ".html"}
EXCLUDED_DIR_NAMES = {
    ".cursor",
    ".git",
    ".idea",
    ".pytest_cache",
    ".venv",
    ".vscode",
    "__pycache__",
    "debug",
    "docs",
    "env",
    "incremental",
    "node_modules",
    "target",
    "terminals",
    "venv",
}
EXCLUDED_PATH_PREFIXES = {
    ("fixtures", "templates"),
    ("fixtures", "generated"),
}
DEFAULT_DOWNLOADS_DIR = Path("/mnt/c/users/dvora/Downloads")

PRIORITY_DOCS = [
    "excel_diff_meta_programming.md",
    "excel_diff_specification.md",
    "excel_diff_testing_plan.md",
    "excel_diff_difficulty_analysis.md",
    "excel_diff_product_differentiation_plan.md",
]
EXTRA_PLANNER_DOCS = ["unified_grid_diff_algorithm_specification.md"]
DESIGN_EVAL_DOCS = PRIORITY_DOCS + EXTRA_PLANNER_DOCS
POST_IMPL_EXCLUDED_DOCS = {"excel_diff_meta_programming.md", "2025-11-30-docs-vs-implementation.md"}
PROJECTIONS_EXCLUDED_DOCS = {"2025-11-30-docs-vs-implementation.md"}

CODEBASE_CONTEXT_FILENAME = "codebase_context.md"

PROMPT_FILES = {
    "planner": "planner_instruction.txt",
    "percent": "percent_completion.md",
    "projections": "revenue_projections.md",
    "post_review": "post_implementation_review_instruction.txt",
    "design_eval": "design_evaluation.md",
    "remediation_template": "remediation_implementer.md",
}

DESIGN_PROMPT_MARKER = (
    "Then examine the implementation in `core/src/` with fresh eyes, informed by but not enslaved to the documentation."
)

DESIGN_TREE_EXCLUDED_DIRS = {"scripts", "logs", "docs", "target"}
DESIGN_TREE_EXCLUDED_FILES = {"README.md"}

DESIGN_CODE_BUNDLES = {
    "codebase_1_core_ir_engine.md": {
        "title": "Core IR and Engine",
        "patterns": [
            "Cargo.toml",
            "core/Cargo.toml",
            "core/src/lib.rs",
            "core/src/workbook.rs",
            "core/src/diff.rs",
            "core/src/engine.rs",
            "core/src/container.rs",
            "core/src/addressing.rs",
            "core/src/hashing.rs",
            "core/src/output/*.rs",
            "core/tests/common/**/*.rs",
            "core/tests/pg1_ir_tests.rs",
            "core/tests/pg3_snapshot_tests.rs",
            "core/tests/pg4_diffop_tests.rs",
            "core/tests/engine_tests.rs",
            "core/tests/output_tests.rs",
            "core/tests/signature_tests.rs",
            "core/tests/addressing_pg2_tests.rs",
            "core/tests/integration_test.rs",
        ],
    },
    "codebase_2_grid_processing.md": {
        "title": "Grid Processing (Parsing, Alignment, Block Moves)",
        "patterns": [
            "core/src/grid_*.rs",
            "core/src/row_alignment.rs",
            "core/src/column_alignment.rs",
            "core/src/database_alignment.rs",
            "core/src/rect_block_move.rs",
            "core/tests/grid_*_tests.rs",
            "core/tests/g*_grid_workbook_tests.rs",
            "core/tests/pg5_grid_diff_tests.rs",
            "core/tests/pg6_object_vs_grid_tests.rs",
            "core/tests/d1_database_mode_tests.rs",
        ],
    },
    "codebase_3_m_language.md": {
        "title": "M Language and DataMashup",
        "patterns": [
            "core/src/datamashup*.rs",
            "core/src/m_*.rs",
            "core/src/excel_open_xml.rs",
            "core/tests/m*_tests.rs",
            "core/tests/data_mashup_tests.rs",
            "core/tests/excel_open_xml_tests.rs",
        ],
    },
    "codebase_4_fixtures.md": {
        "title": "Python Fixtures",
        "patterns": ["fixtures/src/**/*.py"],
    },
}


def estimate_tokens(text: str) -> int:
    return len(text) // 4


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        try:
            return path.read_text(encoding="utf-16")
        except UnicodeDecodeError:
            return path.read_text(encoding="utf-8", errors="replace")


def copy_to_clipboard(text: str, backup_dir: Path | None = None) -> bool:
    attempts: list[str] = []
    try:
        import pyperclip  # type: ignore

        pyperclip.copy(text)
        return True
    except Exception as exc:  # pragma: no cover - dependency optional
        attempts.append(f"pyperclip unavailable or failed: {exc}")

    system = platform.system()
    try:
        if system == "Windows":
            with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False, encoding="utf-8") as tmp:
                tmp.write(text)
                tmp_path = Path(tmp.name)
            try:
                subprocess.run(
                    ["powershell", "-command", f"Get-Content -Raw -Encoding UTF8 -Path '{tmp_path}' | Set-Clipboard"],
                    check=True,
                    capture_output=True,
                )
                return True
            finally:
                tmp_path.unlink(missing_ok=True)
        if system == "Darwin":
            proc = subprocess.Popen(["pbcopy"], stdin=subprocess.PIPE)
            proc.communicate(text.encode("utf-8"))
            return proc.returncode == 0
        proc = subprocess.Popen(["xclip", "-selection", "clipboard"], stdin=subprocess.PIPE)
        proc.communicate(text.encode("utf-8"))
        return proc.returncode == 0
    except Exception as exc:  # pragma: no cover - platform specific
        attempts.append(f"native clipboard command failed: {exc}")

    backup_root = backup_dir or Path.cwd()
    backup_path = backup_root / "_CLIPBOARD_BACKUP.txt"
    backup_path.write_text(text, encoding="utf-8")
    print(f"Clipboard copy failed; saved content to {backup_path}")
    for note in attempts:
        print(f"  - {note}")
    return False


def should_exclude_path(rel_path: Path, excluded_dirs: set[str] = EXCLUDED_DIR_NAMES) -> bool:
    parts = rel_path.parts
    if any(part in excluded_dirs for part in parts):
        return True
    for prefix in EXCLUDED_PATH_PREFIXES:
        if parts[: len(prefix)] == prefix:
            return True
    return False


class ProjectContext:
    def __init__(self, start_dir: Path | None = None, downloads_dir: Path | None = None) -> None:
        self.script_dir = Path(__file__).resolve().parent
        self.root = self._find_repo_root(start_dir or self.script_dir)
        if downloads_dir:
            self.downloads = Path(downloads_dir).expanduser()
        else:
            self.downloads = DEFAULT_DOWNLOADS_DIR
        self.branch = self.current_branch()
        self._git_files = self._load_git_files()

    def _find_repo_root(self, start_dir: Path) -> Path:
        for candidate in [start_dir, *start_dir.parents]:
            if (candidate / ".git").exists():
                return candidate
        return start_dir

    def current_branch(self) -> str | None:
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"],
                capture_output=True,
                text=True,
                check=True,
                cwd=self.root,
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError:
            return None

    def current_commit(self) -> str | None:
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--short", "HEAD"],
                capture_output=True,
                text=True,
                check=True,
                cwd=self.root,
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError:
            return None

    def _load_git_files(self) -> list[Path] | None:
        try:
            result = subprocess.run(
                ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
                capture_output=True,
                text=True,
                check=True,
                cwd=self.root,
            )
            files = [Path(line.strip()) for line in result.stdout.splitlines() if line.strip()]
            return files
        except subprocess.CalledProcessError:
            return None

    def iter_repo_files(self) -> Iterable[Path]:
        if self._git_files is not None:
            for rel_path in sorted(self._git_files):
                path = self.root / rel_path
                if path.is_file():
                    yield path
            return

        for path in sorted(self.root.rglob("*")):
            if path.is_file():
                yield path


class ContextBuilder:
    def __init__(self, output_name: str, context: ProjectContext, downloads_dir: Path | None = None, clean: bool = True):
        self.context = context
        base = Path(downloads_dir) if downloads_dir else context.downloads
        self.out_dir = base / output_name
        if clean and self.out_dir.exists():
            shutil.rmtree(self.out_dir)
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.manifest: list[Path] = []

    def _resolve_source(self, rel_path: str | Path) -> Path:
        path = Path(rel_path)
        if not path.is_absolute():
            path = self.context.root / path
        return path

    def add_file(self, rel_path: str | Path, dest_name: str | None = None) -> Path | None:
        src = self._resolve_source(rel_path)
        if not src.exists():
            return None
        destination = self.out_dir / (dest_name or src.name)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, destination)
        self.manifest.append(destination)
        return destination

    def add_glob(self, pattern: str, dest_subdir: str | None = None) -> list[Path]:
        matched: list[Path] = []
        destination_root = self.out_dir / dest_subdir if dest_subdir else self.out_dir
        destination_root.mkdir(parents=True, exist_ok=True)
        for src in sorted(self.context.root.glob(pattern)):
            if not src.is_file():
                continue
            dest = destination_root / src.name
            shutil.copy2(src, dest)
            self.manifest.append(dest)
            matched.append(dest)
        return matched

    def add_content(self, filename: str, content: str) -> Path:
        destination = self.out_dir / filename
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content, encoding="utf-8")
        self.manifest.append(destination)
        return destination

    def inject_prompt(self, template_name: str, dest_name: str | None = None, copy_prompt: bool = True) -> Path | None:
        template_path = self.context.script_dir / template_name
        if not template_path.exists():
            return None
        content = read_text(template_path)
        out_path = self.add_content(dest_name or template_path.name, content)
        if copy_prompt:
            copy_to_clipboard(content, backup_dir=self.out_dir)
        return out_path

    def add_benchmark_file(self, benchmark_path: Path, dest_name: str = "benchmark_results.json") -> Path | None:
        if not benchmark_path.exists():
            return None
        try:
            data = json.loads(benchmark_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return self.add_file(benchmark_path, dest_name=dest_name)
        commit = self.context.current_commit()
        branch = self.context.branch
        if commit:
            data["git_commit"] = commit
        if branch:
            data["git_branch"] = branch
        destination = self.out_dir / dest_name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(data, indent=2), encoding="utf-8")
        self.manifest.append(destination)
        return destination

    def write_manifest(self, filename: str = "manifest.txt") -> Path:
        lines = [
            "Context manifest",
            f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
            "",
        ]
        for path in sorted(self.manifest):
            rel = path.relative_to(self.out_dir)
            token_estimate = estimate_tokens(read_text(path)) if path.is_file() else 0
            lines.append(f"- {rel} (est. tokens: {token_estimate})")
        return self.add_content(filename, "\n".join(lines) + "\n")


def lang_for_path(path: Path) -> str:
    mapping = {".rs": "rust", ".py": "python", ".toml": "toml", ".yaml": "yaml", ".yml": "yaml", ".md": "markdown", ".js": "javascript", ".html": "html"}
    return mapping.get(path.suffix, "")


def resolve_patterns(root: Path, patterns: Sequence[str]) -> list[Path]:
    seen: set[Path] = set()
    resolved: list[Path] = []
    for pattern in patterns:
        for path in sorted(root.glob(pattern)):
            if path.is_file() and path not in seen:
                seen.add(path)
                resolved.append(path)
    return resolved


def build_directory_tree(paths: Sequence[Path], root: Path) -> str:
    rel_files = [path.relative_to(root) for path in paths if not should_exclude_path(path.relative_to(root))]
    lines = ["/"]
    seen_dirs: set[Path] = set()
    for rel_path in sorted(rel_files):
        parts = list(rel_path.parts)
        for idx in range(len(parts) - 1):
            dir_path = Path(*parts[: idx + 1])
            if should_exclude_path(dir_path):
                break
            if dir_path not in seen_dirs:
                indent = "  " * (idx + 1)
                lines.append(f"{indent}{dir_path.name}/")
                seen_dirs.add(dir_path)
        if should_exclude_path(rel_path):
            continue
        indent = "  " * len(parts)
        lines.append(f"{indent}{parts[-1]}")
    return "\n".join(lines)

def get_last_updated_timestamps(ctx: ProjectContext) -> list[tuple[Path, str | None, int | None]]:
    targets = [
        ctx.root / "docs" / "rust_docs",
        ctx.root / "docs" / "projections",
        ctx.root / "docs" / "competitor_profiles",
    ]
    pattern = "Last updated:"
    results: list[tuple[Path, str | None, int | None]] = []
    now = datetime.now()

    for target_dir in targets:
        if not target_dir.exists():
            continue
        for file_path in target_dir.glob("*.md"):
            content = read_text(file_path)
            marker_index = content.find(pattern)
            timestamp_str = None
            elapsed = None
            if marker_index != -1:
                timestamp_line = content[marker_index : marker_index + 64]
                tokens = timestamp_line.split()
                if len(tokens) >= 3:
                    timestamp_candidate = " ".join(tokens[2:4]) if len(tokens) >= 4 else tokens[2]
                    try:
                        parsed = datetime.strptime(timestamp_candidate, "%Y-%m-%d %H:%M:%S")
                        timestamp_str = parsed.strftime("%Y-%m-%d %H:%M:%S")
                        elapsed = (now - parsed).days
                    except ValueError:
                        timestamp_str = timestamp_candidate
            results.append((file_path.relative_to(ctx.root), timestamp_str, elapsed))

    results.sort(key=lambda item: (item[2] is None, item[2] or 0), reverse=True)
    return results


def generate_timestamp_report(ctx: ProjectContext, output_file: str | None = None) -> str:
    rows = get_last_updated_timestamps(ctx)
    lines = [
        "# Documentation Freshness Report",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "## Document Update Status",
        "",
        "| Document | Last Updated | Days Ago | Status |",
        "|:---------|:-------------|:--------:|:------:|",
    ]
    for rel_path, timestamp, elapsed in rows:
        if timestamp is None:
            lines.append(f"| `{rel_path}` | (not found) | - | missing |")
            continue
        status = "fresh" if elapsed is not None and elapsed <= 7 else "aging" if elapsed and elapsed <= 30 else "stale"
        days_display = str(elapsed) if elapsed is not None else "?"
        lines.append(f"| `{rel_path}` | {timestamp} | {days_display} | {status} |")

    lines.extend(
        [
            "",
            "### Legend",
            "",
            "- fresh: updated in the last 7 days",
            "- aging: updated in the last 30 days",
            "- stale: older than 30 days",
            "- missing: timestamp not found",
            "",
        ]
    )
    report = "\n".join(lines)
    if output_file:
        out_path = ctx.script_dir / output_file
        out_path.write_text(report, encoding="utf-8")
        print(f"Timestamp report written to {out_path}")
    return report


def generate_review_context(ctx: ProjectContext, output_file: str = CODEBASE_CONTEXT_FILENAME) -> Path:
    repo_files = list(ctx.iter_repo_files())
    output_dir = ctx.downloads
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / output_file
    lines = ["# Codebase Context for Review", ""]
    lines.append("## Directory Structure")
    lines.append("")
    lines.append("```text")
    lines.append(build_directory_tree(repo_files, ctx.root))
    lines.append("```")
    lines.append("")
    lines.append("## File Contents")
    lines.append("")

    for path in sorted(repo_files):
        rel = path.relative_to(ctx.root)
        if should_exclude_path(rel):
            continue
        if path == output_path or path.resolve() == Path(__file__).resolve():
            continue
        if path.suffix not in INCLUDED_EXTENSIONS and path.name not in {".gitignore", "Dockerfile"}:
            continue
        content = read_text(path)
        lines.append(f"### File: `{rel}`")
        lines.append("")
        lines.append(f"```{lang_for_path(path)}")
        lines.append(content)
        if not content.endswith("\n"):
            lines.append("")
        lines.append("```")
        lines.append("")
        lines.append("---")
        lines.append("")

    rendered = "\n".join(lines)
    output_path.write_text(rendered, encoding="utf-8")
    print(f"Context generated at: {output_path}")
    print(f"Estimated tokens: {estimate_tokens(rendered)}")
    return output_path


def resolve_codebase_context(ctx: ProjectContext, source: Path | None = None) -> Path:
    if source:
        return Path(source)
    return ctx.downloads / CODEBASE_CONTEXT_FILENAME


def add_codebase_context(builder: ContextBuilder, ctx: ProjectContext, source: Path | None = None) -> Path | None:
    context_path = resolve_codebase_context(ctx, source)
    dest = builder.add_file(context_path, dest_name=CODEBASE_CONTEXT_FILENAME)
    if dest is None:
        print(f"Warning: codebase context not found at {context_path}")
    return dest


def collect_branch_logs(ctx: ProjectContext) -> list[tuple[str, str]]:
    logs_root = ctx.root / "docs" / "meta" / "logs"
    branch_logs: list[tuple[str, str]] = []
    if not logs_root.exists():
        return branch_logs
    for branch_dir in sorted(logs_root.iterdir()):
        if not branch_dir.is_dir():
            continue
        log_file = branch_dir / "activity_log.txt"
        if log_file.exists():
            branch_logs.append((branch_dir.name, read_text(log_file)))
    return branch_logs


def collect_test_results(ctx: ProjectContext) -> list[tuple[str, str]]:
    results_root = ctx.root / "docs" / "meta" / "results"
    results: list[tuple[str, str]] = []
    if not results_root.exists():
        return results
    for result_file in sorted(results_root.iterdir()):
        if result_file.is_file() and result_file.suffix == ".txt":
            results.append((result_file.stem, read_text(result_file)))
    return results


def get_latest_benchmark_result(ctx: ProjectContext) -> Path | None:
    benchmarks_dir = ctx.root / "benchmarks" / "results"
    if not benchmarks_dir.exists():
        return None
    json_files = sorted(benchmarks_dir.glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)
    return json_files[0] if json_files else None


def get_combined_benchmarks_csv(ctx: ProjectContext) -> Path | None:
    csv_path = ctx.root / "benchmarks" / "results" / "combined_results.csv"
    return csv_path if csv_path.exists() else None


def render_benchmark_results(benchmark_path: Path | None) -> str:
    lines = [
        "=" * 60,
        "PERFORMANCE BENCHMARK RESULTS",
        "=" * 60,
        "",
    ]
    if not benchmark_path or not benchmark_path.exists():
        lines.append("(No benchmark results found in benchmarks/results/)")
        return "\n".join(lines) + "\n"

    try:
        data = json.loads(benchmark_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as e:
        lines.append(f"(Error reading benchmark file: {e})")
        return "\n".join(lines) + "\n"

    lines.extend([
        f"Source: {benchmark_path.name}",
        f"Timestamp: {data.get('timestamp', 'unknown')}",
        f"Git Commit: {data.get('git_commit', 'unknown')}",
        f"Git Branch: {data.get('git_branch', 'unknown')}",
        f"Full Scale: {data.get('full_scale', False)}",
        "",
        "-" * 60,
        f"{'Test':<40} {'Time (ms)':>10} {'Rows':>10} {'Cells':>12}",
        "-" * 60,
    ])

    tests = data.get("tests", {})
    for test_name, metrics in sorted(tests.items()):
        time_ms = metrics.get("total_time_ms", 0)
        rows = metrics.get("rows_processed", 0)
        cells = metrics.get("cells_compared", 0)
        lines.append(f"{test_name:<40} {time_ms:>10,} {rows:>10,} {cells:>12,}")

    summary = data.get("summary", {})
    lines.extend([
        "-" * 60,
        f"{'TOTAL':<40} {summary.get('total_time_ms', 0):>10,} "
        f"{summary.get('total_rows_processed', 0):>10,} {summary.get('total_cells_compared', 0):>12,}",
        "=" * 60,
        "",
        "Performance Notes:",
        "- Times are in milliseconds for release builds",
        "- 'Full Scale' indicates 50K row tests vs 1K row quick tests",
        "- See benchmarks/README.md for threshold targets",
        "",
    ])

    return "\n".join(lines)


def render_activity_logs(branch_logs: Sequence[tuple[str, str]]) -> str:
    lines = [
        "=" * 60,
        "COMBINED ACTIVITY LOGS",
        "=" * 60,
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"Total branches with activity logs: {len(branch_logs)}",
        "",
    ]
    if not branch_logs:
        lines.append("(No activity logs found)")
        return "\n".join(lines) + "\n"

    for branch_name, content in branch_logs:
        lines.extend(
            [
                "-" * 60,
                f"BRANCH: {branch_name}",
                "-" * 60,
                "",
                content,
                "" if content.endswith("\n") else "\n",
            ]
        )
    return "\n".join(lines)


def render_test_results(test_results: Sequence[tuple[str, str]]) -> str:
    lines = [
        "=" * 60,
        "COMBINED TEST RESULTS",
        "=" * 60,
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"Total result files: {len(test_results)}",
        "",
    ]
    if not test_results:
        lines.append("(No test results found)")
        return "\n".join(lines) + "\n"

    for branch_name, content in test_results:
        lines.extend(
            [
                "-" * 60,
                f"BRANCH: {branch_name}",
                "-" * 60,
                "",
                content,
                "" if content.endswith("\n") else "\n",
            ]
        )
    return "\n".join(lines)


def render_development_history(branch_logs: Sequence[tuple[str, str]], latest_result: tuple[str, str] | None) -> str:
    lines = [
        "=" * 60,
        "DEVELOPMENT HISTORY",
        "=" * 60,
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "=" * 60,
        "PART 1: ACTIVITY LOGS",
        "=" * 60,
        "",
        f"Total branches with activity logs: {len(branch_logs)}",
        "",
    ]
    if branch_logs:
        for branch_name, content in branch_logs:
            lines.extend(
                [
                    "-" * 60,
                    f"BRANCH: {branch_name}",
                    "-" * 60,
                    "",
                    content,
                    "" if content.endswith("\n") else "\n",
                ]
            )
    else:
        lines.append("(No activity logs found)")
    lines.extend(
        [
            "",
            "=" * 60,
            "PART 2: LATEST TEST RESULTS",
            "=" * 60,
            "",
        ]
    )
    if latest_result:
        branch_name, content = latest_result
        lines.extend([f"Branch: {branch_name}", "-" * 60, "", content])
    else:
        lines.append("(No test results found)")
    return "\n".join(lines)


def render_competitor_profiles(profiles: Sequence[Path]) -> str:
    lines = [
        "# Combined Competitor Profiles",
        "",
        "This document consolidates all competitive intelligence research for the Excel/Power BI diff engine market.",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"Total profiles: {len(profiles)}",
        "",
        "---",
        "",
        "## Table of Contents",
        "",
    ]
    for idx, path in enumerate(profiles, 1):
        display_name = path.stem.replace("_", " ").title()
        lines.append(f"{idx}. [{display_name}](#{path.stem})")
    lines.append("")
    lines.append("---")
    lines.append("")
    for idx, path in enumerate(profiles, 1):
        display_name = path.stem.replace("_", " ").title()
        lines.extend(
            [
                f"<a id=\"{path.stem}\"></a>",
                "",
                f"# [{idx}/{len(profiles)}] {display_name}",
                "",
                f"*Source: `{path.name}`*",
                "",
                read_text(path),
                "",
                "---",
                "",
            ]
        )
    return "\n".join(lines)


def render_cycle_plan(branch_name: str, spec_file: Path | None, decision_file: Path | None) -> str:
    lines = [
        f"# Cycle Plan: {branch_name}",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "## Document Hierarchy Note",
        "",
        "The Mini-Spec below is the planning document that preceded implementation. Documented deviations in the activity",
        "log are acceptable when justified; undocumented deviations should be flagged.",
        "",
        "---",
        "",
        "## Decision Record",
        "",
    ]
    if decision_file and decision_file.exists() and decision_file.stat().st_size > 0:
        lines.extend(["```yaml", read_text(decision_file), "```", ""])
    else:
        lines.append("(No decision record found)")
        lines.append("")
    lines.extend(["---", "", "## Mini-Spec", ""])
    if spec_file and spec_file.exists():
        content = read_text(spec_file)
        lines.append(content)
        if not content.endswith("\n"):
            lines.append("")
    else:
        lines.append("(No spec found)")
    return "\n".join(lines)


def render_cycle_summary(
    branch_name: str, files: Sequence[tuple[Path, Path]], cycle_plan: Path, benchmark_path: Path | None = None
) -> str:
    lines = [
        "=" * 60,
        "POST-IMPLEMENTATION REVIEW CONTEXT",
        "=" * 60,
        "",
        f"Branch: {branch_name}",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "Files included in this review package:",
    ]
    for src, dst in files:
        if dst.exists():
            lines.append(f"  - {dst.name} (from {src})")
    if cycle_plan.exists():
        lines.append("  - cycle_plan.md (combined decision + spec)")
    if benchmark_path and benchmark_path.exists():
        lines.append(f"  - benchmark_results.json (from {benchmark_path.name})")
    lines.extend(["", "=" * 60, "ACTIVITY LOG", "=" * 60, ""])
    return "\n".join(lines)

def run_cargo_tests_and_save(ctx: ProjectContext, branch_name: str | None = None) -> bool:
    branch = branch_name or ctx.branch
    if not branch:
        print("Error: unable to determine branch for test run.")
        return False
    results_dir = ctx.root / "docs" / "meta" / "results"
    results_dir.mkdir(parents=True, exist_ok=True)
    results_file = results_dir / f"{branch}.txt"
    print(f"Running cargo test for branch: {branch}")
    try:
        result = subprocess.run(
            ["cargo", "test"],
            cwd=ctx.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        results_file.write_text(result.stdout, encoding="utf-8")
        print(f"Test output saved to {results_file}")
        return True
    except Exception as exc:
        print(f"Error running cargo test: {exc}")
        return False


def collate_post_implementation_review(
    ctx: ProjectContext,
    branch_name: str | None = None,
    downloads_dir: Path | None = None,
    codebase_context_path: Path | None = None,
) -> Path:
    branch = branch_name or ctx.branch
    if not branch:
        raise RuntimeError("Branch name is required for post-implementation review collation.")
    builder = ContextBuilder(branch, ctx, downloads_dir)
    rust_docs = ctx.root / "docs" / "rust_docs"
    files_to_copy: list[tuple[Path, Path]] = []
    if rust_docs.exists():
        for doc in sorted(rust_docs.glob("*.md")):
            if doc.name in POST_IMPL_EXCLUDED_DOCS:
                continue
            dest = builder.add_file(doc.relative_to(ctx.root))
            if dest:
                files_to_copy.append((doc.relative_to(ctx.root), dest))

    review_prompt = resolve_codebase_context(ctx, codebase_context_path)
    review_dest = add_codebase_context(builder, ctx, review_prompt)
    if review_dest:
        files_to_copy.append((review_prompt, review_dest))

    plans_branch_dir = ctx.root / "docs" / "meta" / "plans" / branch
    spec_file = plans_branch_dir / "spec.md" if plans_branch_dir.exists() else None
    decision_file = plans_branch_dir / "decision.yaml" if plans_branch_dir.exists() else None
    cycle_plan_path = builder.add_content(
        "cycle_plan.md",
        render_cycle_plan(branch, spec_file if spec_file and spec_file.exists() else None, decision_file),
    )

    activity_log = ctx.root / "docs" / "meta" / "logs" / branch / "activity_log.txt"
    test_results = ctx.root / "docs" / "meta" / "results" / f"{branch}.txt"
    benchmark_path = get_latest_benchmark_result(ctx)
    summary_lines = render_cycle_summary(branch, files_to_copy, cycle_plan_path, benchmark_path).splitlines()
    summary_lines.extend(
        [
            "",
            "=" * 60,
            "ACTIVITY LOG",
            "=" * 60,
            "",
            read_text(activity_log) if activity_log.exists() else "(Activity log not found)",
            "",
            "=" * 60,
            "TEST RESULTS",
            "=" * 60,
            "",
            read_text(test_results) if test_results.exists() else "(Test results not found)",
            "",
            render_benchmark_results(benchmark_path),
        ]
    )
    builder.add_content("cycle_summary.txt", "\n".join(summary_lines) + "\n")
    if benchmark_path and benchmark_path.exists():
        builder.add_benchmark_file(benchmark_path)
    combined_csv = get_combined_benchmarks_csv(ctx)
    if combined_csv:
        builder.add_file(combined_csv, dest_name="combined_benchmark_results.csv")

    reviews_branch_dir = ctx.root / "docs" / "meta" / "reviews" / branch
    remediation_files = sorted(
        [path for path in reviews_branch_dir.glob("remediation*.md")] if reviews_branch_dir.exists() else []
    )
    if remediation_files:
        sections: list[str] = ["# Combined Remediation Plans", "", f"Branch: {branch}", ""]
        for idx, rem_file in enumerate(remediation_files, 1):
            sections.extend(
                [
                    f"## [{idx}/{len(remediation_files)}] {rem_file.name}",
                    "",
                    "```markdown",
                    read_text(rem_file),
                    "```",
                    "",
                ]
            )
        builder.add_content("combined_remediations.md", "\n".join(sections))

    builder.inject_prompt(PROMPT_FILES["post_review"], dest_name="post_implementation_review_instruction.txt")
    builder.write_manifest()
    print(f"Collation complete: {builder.out_dir}")
    return builder.out_dir


def collate_percent_completion(ctx: ProjectContext, downloads_dir: Path | None = None) -> Path:
    builder = ContextBuilder("percent_completion", ctx, downloads_dir)
    rust_docs_dir = ctx.root / "docs" / "rust_docs"
    for doc_name in PRIORITY_DOCS:
        doc_path = rust_docs_dir / doc_name
        builder.add_file(doc_path)

    add_codebase_context(builder, ctx)
    builder.add_file(ctx.root / "docs" / "meta" / "todo.md")

    branch_logs = collect_branch_logs(ctx)
    builder.add_content("combined_activity_logs.txt", render_activity_logs(branch_logs))
    builder.add_content("combined_test_results.txt", render_test_results(collect_test_results(ctx)))

    benchmark_path = get_latest_benchmark_result(ctx)
    builder.add_content("benchmark_results.txt", render_benchmark_results(benchmark_path))
    if benchmark_path and benchmark_path.exists():
        builder.add_benchmark_file(benchmark_path)
    combined_csv = get_combined_benchmarks_csv(ctx)
    if combined_csv:
        builder.add_file(combined_csv, dest_name="combined_benchmark_results.csv")

    builder.inject_prompt(PROMPT_FILES["percent"])
    builder.write_manifest()
    print(f"Collation complete: {builder.out_dir}")
    return builder.out_dir


def collate_planner(ctx: ProjectContext, downloads_dir: Path | None = None) -> Path:
    builder = ContextBuilder("planner_context", ctx, downloads_dir)
    rust_docs_dir = ctx.root / "docs" / "rust_docs"
    for doc_name in PRIORITY_DOCS + EXTRA_PLANNER_DOCS:
        builder.add_file(rust_docs_dir / doc_name)

    add_codebase_context(builder, ctx)
    builder.add_file(ctx.root / "docs" / "meta" / "todo.md")

    branch_logs = collect_branch_logs(ctx)
    latest_result = None
    results = collect_test_results(ctx)
    if results:
        latest_result = results[-1]
    builder.add_content("development_history.txt", render_development_history(branch_logs, latest_result))

    benchmark_path = get_latest_benchmark_result(ctx)
    builder.add_content("benchmark_results.txt", render_benchmark_results(benchmark_path))
    if benchmark_path and benchmark_path.exists():
        builder.add_benchmark_file(benchmark_path)
    combined_csv = get_combined_benchmarks_csv(ctx)
    if combined_csv:
        builder.add_file(combined_csv, dest_name="combined_benchmark_results.csv")

    builder.inject_prompt(PROMPT_FILES["planner"])
    builder.write_manifest()
    print(f"Collation complete: {builder.out_dir}")
    return builder.out_dir


def update_remediation_implementer(ctx: ProjectContext) -> str | None:
    branch = ctx.branch
    if not branch:
        print("Error: Could not determine current git branch.")
        return None
    reviews_branch_dir = ctx.root / "docs" / "meta" / "reviews" / branch
    if not reviews_branch_dir.exists():
        print(f"Error: Reviews directory not found: {reviews_branch_dir}")
        return None
    remediation_files = sorted([f for f in reviews_branch_dir.iterdir() if f.is_file() and f.name.startswith("remediation")])
    if not remediation_files:
        print(f"Error: No remediation files found in {reviews_branch_dir}")
        return None
    latest = remediation_files[-1]
    template_path = ctx.script_dir / PROMPT_FILES["remediation_template"]
    if not template_path.exists():
        print(f"Error: Implementer template not found: {template_path}")
        return None
    content = read_text(template_path)
    output = (
        content.replace("{{BRANCH_NAME}}", branch).replace("{{REMEDIATION_PATH}}", f"docs/meta/reviews/{branch}/{latest.name}")
    )
    copy_to_clipboard(output, backup_dir=ctx.script_dir)
    print(f"Remediation implementer prompt generated for {latest}")
    return latest.name


def collate_projections(ctx: ProjectContext, downloads_dir: Path | None = None) -> Path:
    builder = ContextBuilder("projections_context", ctx, downloads_dir)
    rust_docs_dir = ctx.root / "docs" / "rust_docs"
    for doc in sorted(rust_docs_dir.glob("*.md")) if rust_docs_dir.exists() else []:
        if doc.name in PROJECTIONS_EXCLUDED_DOCS:
            continue
        builder.add_file(doc)

    competitor_profiles = []
    profiles_dir = ctx.root / "docs" / "competitor_profiles"
    if profiles_dir.exists():
        competitor_profiles = sorted([p for p in profiles_dir.iterdir() if p.is_file() and p.suffix == ".md"])
    builder.add_content("combined_competitor_profiles.md", render_competitor_profiles(competitor_profiles))
    builder.inject_prompt(PROMPT_FILES["projections"])
    builder.write_manifest()
    print(f"Collation complete: {builder.out_dir}")
    return builder.out_dir


def apply_verification(ctx: ProjectContext) -> str | None:
    plans_dir = ctx.root / "docs" / "meta" / "plans"
    if not plans_dir.exists():
        print(f"Error: Plans directory not found: {plans_dir}")
        return None
    branch_dirs = sorted([d for d in plans_dir.iterdir() if d.is_dir()])
    if not branch_dirs:
        print(f"Error: No branch directories found in {plans_dir}")
        return None
    latest_dir = branch_dirs[-1]
    branch_name = latest_dir.name
    decision_path = f"docs/meta/plans/{branch_name}/decision.yaml"
    spec_path = f"docs/meta/plans/{branch_name}/spec.md"
    verification_path = f"docs/meta/reviews/{branch_name}/verification.md"
    prompt_text = (
        f"I have a verification report (@{verification_path}) that references @{decision_path} and @{spec_path}. "
        "Please implement ____ from the verification report."
    )
    copy_to_clipboard(prompt_text, backup_dir=ctx.script_dir)
    print(f"Generated verification prompt for branch {branch_name}")
    return branch_name


def render_code_bundle(ctx: ProjectContext, title: str, patterns: Sequence[str]) -> str:
    files = resolve_patterns(ctx.root, patterns)
    lines = [f"# {title}", "", "This file contains the following source files:", ""]
    for f in files:
        lines.append(f"- {f.relative_to(ctx.root)}")
    lines.extend(["", "---", ""])
    for f in files:
        rel = f.relative_to(ctx.root)
        content = read_text(f)
        lines.extend(
            [
                f"## File: {rel}",
                "",
                f"```{lang_for_path(f)}",
                content,
                "```",
                "",
            ]
        )
    return "\n".join(lines)


def render_directory_snapshot(
    ctx: ProjectContext,
    path: Path,
    depth: int = 4,
    exclude_dirs: set[str] | None = None,
    exclude_files: set[str] | None = None,
) -> str:
    lines = [f"{path.name}/"]
    for entry in sorted(path.rglob("*")):
        rel = entry.relative_to(path)
        if should_exclude_path(rel):
            continue
        if exclude_dirs and rel.parts and rel.parts[0] in exclude_dirs:
            continue
        if exclude_files and len(rel.parts) == 1 and rel.name in exclude_files:
            continue
        if len(rel.parts) > depth:
            continue
        indent = "  " * len(rel.parts)
        suffix = "/" if entry.is_dir() else ""
        lines.append(f"{indent}{rel.name}{suffix}")
    return "\n".join(lines)


def collate_design_evaluation(ctx: ProjectContext, downloads_dir: Path | None = None) -> Path:
    builder = ContextBuilder("design_evaluation", ctx, downloads_dir)
    rust_docs_dir = ctx.root / "docs" / "rust_docs"
    for doc in DESIGN_EVAL_DOCS:
        builder.add_file(rust_docs_dir / doc)

    token_report: list[tuple[str, int]] = []
    for filename, config in DESIGN_CODE_BUNDLES.items():
        content = render_code_bundle(ctx, config["title"], config["patterns"])
        destination = builder.add_content(filename, content)
        token_report.append((destination.name, estimate_tokens(content)))

    benchmark_path = get_latest_benchmark_result(ctx)
    builder.add_content("benchmark_results.txt", render_benchmark_results(benchmark_path))
    if benchmark_path and benchmark_path.exists():
        builder.add_benchmark_file(benchmark_path)
    combined_csv = get_combined_benchmarks_csv(ctx)
    if combined_csv:
        builder.add_file(combined_csv, dest_name="combined_benchmark_results.csv")

    token_report.sort(key=lambda item: item[1], reverse=True)
    report_lines = ["Token estimates per bundle:", ""]
    for name, tokens in token_report:
        report_lines.append(f"- {name}: {tokens:,} tokens (est)")
    builder.add_content("token_report.txt", "\n".join(report_lines) + "\n")

    extra_text = (
        "\n---\n\n"
        "## Codebase Structure for Evaluation\n\n"
        "### Directory Tree\n\n"
        "```\n"
        f"{render_directory_snapshot(ctx, ctx.root, exclude_dirs=DESIGN_TREE_EXCLUDED_DIRS, exclude_files=DESIGN_TREE_EXCLUDED_FILES)}\n"
        "```\n\n"
        "### Codebase File Allocation Strategy\n\n"
    )
    for filename, config in DESIGN_CODE_BUNDLES.items():
        extra_text += f"- `{filename}`: {config['title']}\n"

    design_eval_file = ctx.script_dir / PROMPT_FILES["design_eval"]
    if design_eval_file.exists():
        design_prompt = read_text(design_eval_file)
        if DESIGN_PROMPT_MARKER in design_prompt:
            design_prompt = design_prompt.replace(DESIGN_PROMPT_MARKER, DESIGN_PROMPT_MARKER + extra_text)
        builder.add_content("design_evaluation_prompt.md", design_prompt)
        copy_to_clipboard(design_prompt, backup_dir=builder.out_dir)
        print("Design evaluation prompt copied to clipboard.")
    else:
        print(f"Warning: design evaluation prompt not found at {design_eval_file}")

    builder.write_manifest()
    print(f"Collation complete: {builder.out_dir}")
    return builder.out_dir

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Generate review and planning contexts.")
    parser.add_argument(
        "--downloads",
        type=Path,
        help="Override downloads/output directory (default: /mnt/c/users/dvora/Downloads)",
    )
    parser.add_argument("--root", type=Path, help="Override repository root search start")
    subparsers = parser.add_subparsers(dest="command")

    review_parser = subparsers.add_parser("review", help="Collate post-implementation review bundle")
    review_parser.add_argument("--branch", help="Branch name (defaults to current)")
    review_parser.add_argument("--skip-tests", action="store_true", help="Skip running cargo test")
    review_parser.add_argument(
        "--context-output",
        default=CODEBASE_CONTEXT_FILENAME,
        help="Output filename for review context",
    )
    review_parser.set_defaults(func=cmd_review)

    plan_parser = subparsers.add_parser("plan", help="Collate planner context for next cycle")
    plan_parser.set_defaults(func=cmd_plan)

    percent_parser = subparsers.add_parser("percent", help="Collate percent completion bundle")
    percent_parser.set_defaults(func=cmd_percent)

    projections_parser = subparsers.add_parser("projections", help="Collate revenue projections bundle")
    projections_parser.set_defaults(func=cmd_projections)

    apply_parser = subparsers.add_parser("apply", help="Copy verification apply prompt for latest branch")
    apply_parser.set_defaults(func=cmd_apply)

    remediate_parser = subparsers.add_parser("remediate", help="Update remediation implementer prompt")
    remediate_parser.set_defaults(func=cmd_remediate)

    design_parser = subparsers.add_parser("design", help="Collate design evaluation bundle")
    design_parser.set_defaults(func=cmd_design)

    context_parser = subparsers.add_parser("context", help="Generate review context markdown")
    context_parser.add_argument("--output", default=CODEBASE_CONTEXT_FILENAME, help="Output filename for context")
    context_parser.set_defaults(func=cmd_context)

    timestamps_parser = subparsers.add_parser("report", help="Generate documentation freshness report")
    timestamps_parser.add_argument("--output", default="timestamp_report.md", help="Output filename for report")
    timestamps_parser.set_defaults(func=cmd_report)

    return parser


def cmd_review(ctx: ProjectContext, args: argparse.Namespace) -> None:
    if not args.skip_tests:
        run_cargo_tests_and_save(ctx, branch_name=args.branch)
    context_path = generate_review_context(ctx, output_file=args.context_output)
    collate_post_implementation_review(
        ctx,
        branch_name=args.branch,
        downloads_dir=args.downloads,
        codebase_context_path=context_path,
    )


def cmd_plan(ctx: ProjectContext, args: argparse.Namespace) -> None:
    collate_planner(ctx, downloads_dir=args.downloads)


def cmd_percent(ctx: ProjectContext, args: argparse.Namespace) -> None:
    collate_percent_completion(ctx, downloads_dir=args.downloads)


def cmd_projections(ctx: ProjectContext, args: argparse.Namespace) -> None:
    collate_projections(ctx, downloads_dir=args.downloads)


def cmd_apply(ctx: ProjectContext, args: argparse.Namespace) -> None:
    apply_verification(ctx)


def cmd_remediate(ctx: ProjectContext, args: argparse.Namespace) -> None:
    update_remediation_implementer(ctx)


def cmd_design(ctx: ProjectContext, args: argparse.Namespace) -> None:
    collate_design_evaluation(ctx, downloads_dir=args.downloads)


def cmd_context(ctx: ProjectContext, args: argparse.Namespace) -> None:
    generate_review_context(ctx, output_file=args.output)


def cmd_report(ctx: ProjectContext, args: argparse.Namespace) -> None:
    generate_timestamp_report(ctx, output_file=args.output)


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)
    if not args.command:
        parser.print_help()
        return 0

    ctx = ProjectContext(start_dir=args.root, downloads_dir=args.downloads)
    args.func(ctx, args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
