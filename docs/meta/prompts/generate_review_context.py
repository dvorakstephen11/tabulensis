import os
import fnmatch
import platform
import re
import shutil
import subprocess
from datetime import datetime
from pathlib import Path


def copy_to_clipboard(text):
    system = platform.system()
    if system == 'Windows':
        import tempfile
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False, encoding='utf-8') as f:
            f.write(text)
            temp_path = f.name
        try:
            subprocess.run(
                ['powershell', '-command', f'Get-Content -Raw -Encoding UTF8 -Path "{temp_path}" | Set-Clipboard'],
                check=True,
                capture_output=True
            )
        finally:
            os.unlink(temp_path)
    elif system == 'Darwin':
        process = subprocess.Popen(['pbcopy'], stdin=subprocess.PIPE)
        process.communicate(text.encode('utf-8'))
    else:
        process = subprocess.Popen(['xclip', '-selection', 'clipboard'], stdin=subprocess.PIPE)
        process.communicate(text.encode('utf-8'))


def get_last_updated_timestamps(root_dir="."):
    """
    Scans documentation files for 'Last updated: YYYY-MM-DD HH:MM:SS' timestamps.
    Returns a list of tuples: (relative_path, timestamp_str, days_elapsed)
    """
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(script_dir, root_dir))
    
    target_dirs = [
        os.path.join(repo_root, "docs", "rust_docs"),
        os.path.join(repo_root, "docs", "projections"),
        os.path.join(repo_root, "docs", "competitor_profiles"),
    ]
    
    timestamp_pattern = re.compile(r'Last updated:\s*(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})')
    results = []
    now = datetime.now()
    
    for target_dir in target_dirs:
        if not os.path.exists(target_dir):
            continue
        for filename in os.listdir(target_dir):
            if not filename.endswith('.md'):
                continue
            filepath = os.path.join(target_dir, filename)
            try:
                with open(filepath, 'r', encoding='utf-8') as f:
                    content = f.read()
                match = timestamp_pattern.search(content)
                if match:
                    timestamp_str = match.group(1)
                    try:
                        timestamp_dt = datetime.strptime(timestamp_str, "%Y-%m-%d %H:%M:%S")
                        days_elapsed = (now - timestamp_dt).days
                    except ValueError:
                        days_elapsed = None
                else:
                    timestamp_str = None
                    days_elapsed = None
                
                rel_path = os.path.relpath(filepath, repo_root)
                results.append((rel_path, timestamp_str, days_elapsed))
            except Exception:
                pass
    
    results.sort(key=lambda x: (x[2] is None, x[2] if x[2] is not None else 0), reverse=True)
    return results


def generate_timestamp_report(output_file=None):
    """
    Generates a visually appealing Markdown table showing document timestamps
    and how many days have elapsed since last update.
    """
    script_dir = os.path.dirname(os.path.abspath(__file__))
    results = get_last_updated_timestamps("../../../")
    
    lines = []
    lines.append("# Documentation Freshness Report")
    lines.append("")
    lines.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append("")
    lines.append("## Document Update Status")
    lines.append("")
    lines.append("| Document | Last Updated | Days Ago | Status |")
    lines.append("|:---------|:-------------|:--------:|:------:|")
    
    for rel_path, timestamp_str, days_elapsed in results:
        doc_name = os.path.basename(rel_path)
        folder = os.path.basename(os.path.dirname(rel_path))
        display_name = f"`{folder}/{doc_name}`"
        
        if timestamp_str is None:
            ts_display = "❌ No timestamp"
            days_display = "-"
            status = "⚠️"
        else:
            ts_display = timestamp_str
            if days_elapsed is not None:
                days_display = str(days_elapsed)
                if days_elapsed == 0:
                    status = "🟢"
                elif days_elapsed <= 7:
                    status = "🟢"
                elif days_elapsed <= 30:
                    status = "🟡"
                else:
                    status = "🔴"
            else:
                days_display = "?"
                status = "⚠️"
        
        lines.append(f"| {display_name} | {ts_display} | {days_display} | {status} |")
    
    lines.append("")
    lines.append("### Legend")
    lines.append("")
    lines.append("- 🟢 Fresh (≤7 days)")
    lines.append("- 🟡 Aging (8-30 days)")
    lines.append("- 🔴 Stale (>30 days)")
    lines.append("- ⚠️ Missing timestamp or parse error")
    lines.append("")
    
    report_content = "\n".join(lines)
    
    if output_file:
        output_path = os.path.join(script_dir, output_file)
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(report_content)
        print(f"Timestamp report generated at: {output_path}")
    
    return report_content


def generate_review_context(output_file="review_prompt.md", root_dir="../../../"):
    # Adjust root_dir relative to where this script is: docs/meta/prompts/ -> repo root is ../../../
    # If running from repo root, it would be "."
    
    # Determine absolute path to repo root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(script_dir, root_dir))
    
    output_path = os.path.join(script_dir, output_file)
    
    # Define what to include
    included_extensions = {
        '.rs', '.py', '.toml', '.yaml', '.yml', '.gitignore'
    }
    
    # Explicitly ignore these directories
    ignored_dirs = {
        'target', '.git', '.cursor', 'node_modules', '__pycache__', 
        '.idea', '.vscode', '.venv', '.pytest_cache', 'venv', 'env', 
        'terminals', 'debug', 'incremental', 'docs', 
        'fixtures/templates', 'fixtures/generated'
    }
    
    # Simple gitignore parser (limited support)
    ignored_patterns = []
    gitignore_path = os.path.join(repo_root, '.gitignore')
    if os.path.exists(gitignore_path):
        with open(gitignore_path, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    ignored_patterns.append(line)

    def is_ignored(path, is_dir=False):
        name = os.path.basename(path)
        if is_dir and name in ignored_dirs:
            return True
        
        # Basic pattern matching against gitignore lines
        # This is a heuristic and not a full gitignore implementation
        for pattern in ignored_patterns:
            if fnmatch.fnmatch(name, pattern):
                return True
            if pattern.endswith('/') and is_dir:
                if fnmatch.fnmatch(name + '/', pattern):
                    return True
        return False

    with open(output_path, 'w', encoding='utf-8') as out:
        out.write("# Codebase Context for Review\n\n")
        
        # 1. Output Directory Structure
        out.write("## Directory Structure\n\n")
        out.write("```text\n")
        
        for dirpath, dirnames, filenames in os.walk(repo_root):
            # Filter directories in-place
            dirnames[:] = [d for d in dirnames if not is_ignored(os.path.join(dirpath, d), is_dir=True)]
            
            # Calculate relative path from repo root
            rel_path = os.path.relpath(dirpath, repo_root)
            
            # Skip if the directory itself matches a pattern relative to root (basic check)
            if rel_path != "." and is_ignored(rel_path, is_dir=True):
                continue

            if rel_path == ".":
                level = 0
            else:
                level = rel_path.count(os.sep) + 1
            
            indent = "  " * level
            dirname = os.path.basename(dirpath)
            if rel_path == ".":
                out.write("/\n")
            else:
                out.write(f"{indent}{dirname}/\n")
            
            for f in filenames:
                if f.startswith('.git'): continue
                if is_ignored(os.path.join(dirpath, f)): continue
                out.write(f"{indent}  {f}\n")
                
        out.write("```\n\n")
        
        # 2. Output File Contents
        out.write("## File Contents\n\n")
        
        for dirpath, dirnames, filenames in os.walk(repo_root):
            dirnames[:] = [d for d in dirnames if not is_ignored(os.path.join(dirpath, d), is_dir=True)]
            
            for f in filenames:
                file_path = os.path.join(dirpath, f)
                rel_path = os.path.relpath(file_path, repo_root)
                
                if is_ignored(file_path):
                    continue
                    
                _, ext = os.path.splitext(f)
                
                # Check if it's a file we want to include content for
                if ext in included_extensions or f in ['.gitignore', 'Dockerfile']:
                    # Exclude this generated file itself if it ends up in the scan
                    if os.path.abspath(file_path) == os.path.abspath(output_path):
                        continue
                    # Exclude the script itself
                    if os.path.abspath(file_path) == os.path.abspath(__file__):
                        continue

                    try:
                        with open(file_path, 'r', encoding='utf-8') as source_file:
                            content = source_file.read()
                            
                        out.write(f"### File: `{rel_path}`\n\n")
                        
                        # Determine markdown code block language
                        lang = ""
                        if ext == '.rs': lang = "rust"
                        elif ext == '.py': lang = "python"
                        elif ext in ['.toml', '.yaml', '.yml']: lang = "yaml"
                        elif ext == '.md': lang = "markdown"
                        
                        out.write(f"```{lang}\n")
                        out.write(content)
                        if not content.endswith('\n'):
                            out.write("\n")
                        out.write("```\n\n")
                        out.write("---\n\n")
                        
                    except Exception as e:
                        # Just skip binary files or read errors
                        pass

    print(f"Context generated at: {output_path}")

def get_current_branch():
    try:
        result = subprocess.run(
            ['git', 'rev-parse', '--abbrev-ref', 'HEAD'],
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError:
        return None


def collate_post_implementation_review(branch_name=None, downloads_dir=None):
    script_dir = Path(__file__).parent.resolve()
    repo_root = script_dir.parent.parent.parent
    
    if branch_name is None:
        os.chdir(repo_root)
        branch_name = get_current_branch()
        if branch_name is None:
            print("Error: Could not determine current git branch.")
            return None
    
    if downloads_dir is None:
        downloads_dir = Path.home() / "Downloads"
    else:
        downloads_dir = Path(downloads_dir)
    
    output_dir = downloads_dir / branch_name
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Collating post-implementation review context for branch: {branch_name}")
    print(f"Output directory: {output_dir}")
    
    files_to_copy = []
    
    rust_docs_dir = repo_root / "docs" / "rust_docs"
    excluded_docs = {"excel_diff_meta_programming.md", "2025-11-30-docs-vs-implementation.md"}
    if rust_docs_dir.exists():
        for f in rust_docs_dir.iterdir():
            if f.is_file() and f.suffix == '.md' and f.name not in excluded_docs:
                files_to_copy.append((f, output_dir / f.name))
    
    review_prompt = script_dir / "review_prompt.md"
    if review_prompt.exists():
        files_to_copy.append((review_prompt, output_dir / "codebase_context.md"))
    
    copied_count = 0
    for src, dst in files_to_copy:
        try:
            shutil.copy2(src, dst)
            print(f"  Copied: {src.name} -> {dst.name}")
            copied_count += 1
        except Exception as e:
            print(f"  Error copying {src.name}: {e}")
    
    plans_branch_dir = repo_root / "docs" / "meta" / "plans" / branch_name
    cycle_plan_path = output_dir / "cycle_plan.md"
    spec_file = plans_branch_dir / "spec.md" if plans_branch_dir.exists() else None
    decision_file = plans_branch_dir / "decision.yaml" if plans_branch_dir.exists() else None
    
    with open(cycle_plan_path, 'w', encoding='utf-8') as f:
        f.write(f"# Cycle Plan: {branch_name}\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        f.write("## Decision Record\n\n")
        if decision_file and decision_file.exists() and decision_file.stat().st_size > 0:
            f.write("```yaml\n")
            f.write(decision_file.read_text(encoding='utf-8'))
            f.write("```\n\n")
        else:
            f.write("(No decision record found)\n\n")
        
        f.write("---\n\n")
        f.write("## Mini-Spec\n\n")
        if spec_file and spec_file.exists():
            spec_content = spec_file.read_text(encoding='utf-8')
            f.write(spec_content)
            if not spec_content.endswith('\n'):
                f.write('\n')
        else:
            f.write("(No spec found)\n")
    
    print(f"  Created: cycle_plan.md (combined decision + spec)")
    copied_count += 1
    
    activity_log = repo_root / "docs" / "meta" / "logs" / branch_name / "activity_log.txt"
    test_results = repo_root / "docs" / "meta" / "results" / f"{branch_name}.txt"
    
    combined_path = output_dir / "cycle_summary.txt"
    with open(combined_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("POST-IMPLEMENTATION REVIEW CONTEXT\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Branch: {branch_name}\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        f.write("Files included in this review package:\n")
        for src, dst in files_to_copy:
            if dst.exists():
                f.write(f"  - {dst.name} (from {src.relative_to(repo_root)})\n")
        if cycle_plan_path.exists():
            f.write(f"  - cycle_plan.md (combined decision + spec)\n")
        f.write("\n")
        
        f.write("=" * 60 + "\n")
        f.write("ACTIVITY LOG\n")
        f.write("=" * 60 + "\n\n")
        if activity_log.exists():
            f.write(activity_log.read_text(encoding='utf-8'))
            if not f.tell() == 0:
                f.write("\n")
        else:
            f.write("(Activity log not found)\n")
        f.write("\n")
        
        f.write("=" * 60 + "\n")
        f.write("TEST RESULTS\n")
        f.write("=" * 60 + "\n\n")
        if test_results.exists():
            try:
                content = test_results.read_text(encoding='utf-8')
            except UnicodeDecodeError:
                try:
                    content = test_results.read_text(encoding='utf-16')
                except UnicodeDecodeError:
                    content = test_results.read_text(encoding='utf-8', errors='replace')
            f.write(content)
        else:
            f.write("(Test results not found)\n")
    
    print(f"  Created: cycle_summary.txt (combined manifest, activity log, test results)")
    copied_count += 1
    
    reviews_branch_dir = repo_root / "docs" / "meta" / "reviews" / branch_name
    if reviews_branch_dir.exists():
        remediation_files = []
        for f in reviews_branch_dir.iterdir():
            if f.is_file() and f.name.startswith("remediation") and f.name.endswith(".md"):
                remediation_files.append(f)
        
        if remediation_files:
            remediation_files.sort(key=lambda x: x.name)
            combined_remediation_path = output_dir / "combined_remediations.md"
            
            with open(combined_remediation_path, 'w', encoding='utf-8') as f:
                f.write("# Combined Remediation Plans\n\n")
                f.write(f"Branch: `{branch_name}`\n")
                f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
                f.write(f"Total remediation files: {len(remediation_files)}\n\n")
                f.write("---\n\n")
                
                for i, rem_file in enumerate(remediation_files, 1):
                    f.write(f"## [{i}/{len(remediation_files)}] {rem_file.name}\n\n")
                    f.write("```markdown\n")
                    content = rem_file.read_text(encoding='utf-8')
                    f.write(content)
                    if not content.endswith('\n'):
                        f.write('\n')
                    f.write("```\n\n")
                    if i < len(remediation_files):
                        f.write("---\n\n")
            
            print(f"  Created: combined_remediations.md ({len(remediation_files)} files)")
            copied_count += 1
    
    print(f"\nCollation complete: {copied_count} files in {output_dir}")
    
    instruction_file = script_dir / "post_implementation_review_instruction.txt"
    if instruction_file.exists():
        try:
            instruction_text = instruction_file.read_text(encoding='utf-8')
            copy_to_clipboard(instruction_text)
            print(f"\nPost-implementation review instruction copied to clipboard!")
        except Exception as e:
            print(f"\nWarning: Could not copy instruction to clipboard: {e}")
    else:
        print(f"\nWarning: Instruction file not found at {instruction_file}")
    
    return output_dir


def collate_percent_completion(downloads_dir=None):
    script_dir = Path(__file__).parent.resolve()
    repo_root = script_dir.parent.parent.parent
    
    if downloads_dir is None:
        downloads_dir = Path.home() / "Downloads"
    else:
        downloads_dir = Path(downloads_dir)
    
    output_dir = downloads_dir / "percent_completion"
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("Collating percent completion context...")
    print(f"Output directory: {output_dir}")
    
    files_to_copy = []
    
    rust_docs_dir = repo_root / "docs" / "rust_docs"
    priority_docs = [
        "excel_diff_meta_programming.md",
        "excel_diff_specification.md",
        "excel_diff_testing_plan.md",
        "excel_diff_difficulty_analysis.md",
        "excel_diff_product_differentiation_plan.md",
    ]
    
    for doc_name in priority_docs:
        doc_path = rust_docs_dir / doc_name
        if doc_path.exists():
            files_to_copy.append((doc_path, output_dir / doc_name))
    
    review_prompt = script_dir / "review_prompt.md"
    if review_prompt.exists():
        files_to_copy.append((review_prompt, output_dir / "codebase_context.md"))
    
    todo_file = repo_root / "docs" / "meta" / "todo.md"
    if todo_file.exists():
        files_to_copy.append((todo_file, output_dir / "todo.md"))
    
    copied_count = 0
    for src, dst in files_to_copy:
        try:
            shutil.copy2(src, dst)
            print(f"  Copied: {src.name} -> {dst.name}")
            copied_count += 1
        except Exception as e:
            print(f"  Error copying {src.name}: {e}")
    
    logs_dir = repo_root / "docs" / "meta" / "logs"
    combined_logs_path = output_dir / "combined_activity_logs.txt"
    
    branch_logs = []
    if logs_dir.exists():
        for branch_dir in sorted(logs_dir.iterdir()):
            if branch_dir.is_dir():
                activity_log = branch_dir / "activity_log.txt"
                if activity_log.exists():
                    branch_name = branch_dir.name
                    try:
                        content = activity_log.read_text(encoding='utf-8')
                        branch_logs.append((branch_name, content))
                    except Exception:
                        pass
    
    with open(combined_logs_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("COMBINED ACTIVITY LOGS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Total branches with activity logs: {len(branch_logs)}\n\n")
        
        if branch_logs:
            for branch_name, content in branch_logs:
                f.write("-" * 60 + "\n")
                f.write(f"BRANCH: {branch_name}\n")
                f.write("-" * 60 + "\n\n")
                f.write(content)
                if not content.endswith('\n'):
                    f.write("\n")
                f.write("\n")
        else:
            f.write("(No activity logs found)\n")
    
    print(f"  Created: combined_activity_logs.txt ({len(branch_logs)} branches)")
    copied_count += 1
    
    results_dir = repo_root / "docs" / "meta" / "results"
    combined_results_path = output_dir / "combined_test_results.txt"
    
    test_results = []
    if results_dir.exists():
        for result_file in sorted(results_dir.iterdir()):
            if result_file.is_file() and result_file.suffix == '.txt':
                branch_name = result_file.stem
                try:
                    content = result_file.read_text(encoding='utf-8')
                except UnicodeDecodeError:
                    try:
                        content = result_file.read_text(encoding='utf-16')
                    except UnicodeDecodeError:
                        content = result_file.read_text(encoding='utf-8', errors='replace')
                test_results.append((branch_name, content))
    
    with open(combined_results_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("COMBINED TEST RESULTS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Total result files: {len(test_results)}\n\n")
        
        if test_results:
            for branch_name, content in test_results:
                f.write("-" * 60 + "\n")
                f.write(f"BRANCH: {branch_name}\n")
                f.write("-" * 60 + "\n\n")
                f.write(content)
                if not content.endswith('\n'):
                    f.write("\n")
                f.write("\n")
        else:
            f.write("(No test results found)\n")
    
    print(f"  Created: combined_test_results.txt ({len(test_results)} result files)")
    copied_count += 1
    
    print(f"\nCollation complete: {copied_count} files in {output_dir}")
    print("\nFiles included (10 key documents):")
    print("  1-6. Technical blueprints from docs/rust_docs/")
    print("  7.   codebase_context.md (current code snapshot)")
    print("  8.   todo.md (current task list)")
    print("  9.   combined_activity_logs.txt (all branch activity)")
    print("  10.  combined_test_results.txt (all test outputs)")
    
    prompt_file = script_dir / "percent_completion.md"
    if prompt_file.exists():
        try:
            prompt_text = prompt_file.read_text(encoding='utf-8')
            copy_to_clipboard(prompt_text)
            print(f"\nPercent completion prompt copied to clipboard!")
        except Exception as e:
            print(f"\nWarning: Could not copy prompt to clipboard: {e}")
    else:
        print(f"\nWarning: Prompt file not found at {prompt_file}")
    
    return output_dir


def collate_planner(downloads_dir=None):
    script_dir = Path(__file__).parent.resolve()
    repo_root = script_dir.parent.parent.parent
    
    if downloads_dir is None:
        downloads_dir = Path.home() / "Downloads"
    else:
        downloads_dir = Path(downloads_dir)
    
    output_dir = downloads_dir / "planner_context"
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("Collating planner context...")
    print(f"Output directory: {output_dir}")
    
    files_to_copy = []
    
    rust_docs_dir = repo_root / "docs" / "rust_docs"
    priority_docs = [
        "excel_diff_meta_programming.md",
        "excel_diff_specification.md",
        "excel_diff_testing_plan.md",
        "excel_diff_difficulty_analysis.md",
        "excel_diff_product_differentiation_plan.md",
    ]
    
    for doc_name in priority_docs:
        doc_path = rust_docs_dir / doc_name
        if doc_path.exists():
            files_to_copy.append((doc_path, output_dir / doc_name))
    
    extra_rust_docs = [
        "unified_grid_diff_algorithm_specification.md",
    ]
    for doc_name in extra_rust_docs:
        doc_path = rust_docs_dir / doc_name
        if doc_path.exists():
            files_to_copy.append((doc_path, output_dir / doc_name))
    
    review_prompt = script_dir / "review_prompt.md"
    if review_prompt.exists():
        files_to_copy.append((review_prompt, output_dir / "codebase_context.md"))
    
    todo_file = repo_root / "docs" / "meta" / "todo.md"
    if todo_file.exists():
        files_to_copy.append((todo_file, output_dir / "todo.md"))
    
    copied_count = 0
    for src, dst in files_to_copy:
        try:
            shutil.copy2(src, dst)
            print(f"  Copied: {src.name} -> {dst.name}")
            copied_count += 1
        except Exception as e:
            print(f"  Error copying {src.name}: {e}")
    
    logs_dir = repo_root / "docs" / "meta" / "logs"
    branch_logs = []
    if logs_dir.exists():
        for branch_dir in sorted(logs_dir.iterdir()):
            if branch_dir.is_dir():
                activity_log = branch_dir / "activity_log.txt"
                if activity_log.exists():
                    branch_name = branch_dir.name
                    try:
                        content = activity_log.read_text(encoding='utf-8')
                        branch_logs.append((branch_name, content))
                    except Exception:
                        pass
    
    results_dir = repo_root / "docs" / "meta" / "results"
    latest_results = None
    if results_dir.exists():
        result_files = sorted(results_dir.iterdir(), reverse=True)
        for result_file in result_files:
            if result_file.is_file() and result_file.suffix == '.txt':
                try:
                    content = result_file.read_text(encoding='utf-8')
                except UnicodeDecodeError:
                    try:
                        content = result_file.read_text(encoding='utf-16')
                    except UnicodeDecodeError:
                        content = result_file.read_text(encoding='utf-8', errors='replace')
                latest_results = (result_file.stem, content)
                break
    
    dev_history_path = output_dir / "development_history.txt"
    with open(dev_history_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("DEVELOPMENT HISTORY\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        f.write("=" * 60 + "\n")
        f.write("PART 1: ACTIVITY LOGS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Total branches with activity logs: {len(branch_logs)}\n\n")
        
        if branch_logs:
            for branch_name, content in branch_logs:
                f.write("-" * 60 + "\n")
                f.write(f"BRANCH: {branch_name}\n")
                f.write("-" * 60 + "\n\n")
                f.write(content)
                if not content.endswith('\n'):
                    f.write("\n")
                f.write("\n")
        else:
            f.write("(No activity logs found)\n\n")
        
        f.write("=" * 60 + "\n")
        f.write("PART 2: LATEST TEST RESULTS\n")
        f.write("=" * 60 + "\n\n")
        
        if latest_results:
            branch_name, content = latest_results
            f.write(f"Branch: {branch_name}\n")
            f.write("-" * 60 + "\n\n")
            f.write(content)
        else:
            f.write("(No test results found)\n")
    
    print(f"  Created: development_history.txt ({len(branch_logs)} branches + latest test results)")
    copied_count += 1
    
    print(f"\nCollation complete: {copied_count} files in {output_dir}")
    print("\nFiles included:")
    print("  1-5. Technical blueprints from docs/rust_docs/")
    print("  6.   Design doc (unified grid diff algorithm specification)")
    print("  7.   codebase_context.md (current code snapshot)")
    print("  8.   todo.md (current task list)")
    print("  9.   development_history.txt (activity logs + latest test results)")
    
    prompt_file = script_dir / "planner_instruction.txt"
    if prompt_file.exists():
        try:
            prompt_text = prompt_file.read_text(encoding='utf-8')
            copy_to_clipboard(prompt_text)
            print(f"\nPlanner instruction copied to clipboard!")
        except Exception as e:
            print(f"\nWarning: Could not copy prompt to clipboard: {e}")
    else:
        print(f"\nWarning: Planner instruction file not found at {prompt_file}")
    
    return output_dir


def run_cargo_tests_and_save(repo_root=None, branch_name=None):
    if repo_root is None:
        script_dir = Path(__file__).parent.resolve()
        repo_root = script_dir.parent.parent.parent
    else:
        repo_root = Path(repo_root)
    
    if branch_name is None:
        os.chdir(repo_root)
        branch_name = get_current_branch()
        if branch_name is None:
            print("Error: Could not determine current git branch.")
            return False
    
    results_dir = repo_root / "docs" / "meta" / "results"
    results_dir.mkdir(parents=True, exist_ok=True)
    results_file = results_dir / f"{branch_name}.txt"
    
    print(f"Running cargo test for branch: {branch_name}")
    print(f"Output will be saved to: {results_file}")
    
    try:
        result = subprocess.run(
            ['cargo', 'test'],
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True
        )
        
        with open(results_file, 'w', encoding='utf-8') as f:
            f.write(result.stdout)
        
        print(f"Test output saved to: {results_file}")
        return True
    except Exception as e:
        print(f"Error running cargo test: {e}")
        return False


def update_remediation_implementer():
    script_dir = Path(__file__).parent.resolve()
    repo_root = script_dir.parent.parent.parent
    
    os.chdir(repo_root)
    branch_name = get_current_branch()
    if branch_name is None:
        print("Error: Could not determine current git branch.")
        return None
    
    reviews_branch_dir = repo_root / "docs" / "meta" / "reviews" / branch_name
    if not reviews_branch_dir.exists():
        print(f"Error: Reviews directory not found: {reviews_branch_dir}")
        return None
    
    remediation_files = []
    for f in reviews_branch_dir.iterdir():
        if f.is_file() and f.name.startswith("remediation") and f.name.endswith(".md"):
            remediation_files.append(f.name)
    
    if not remediation_files:
        print(f"Error: No remediation files found in {reviews_branch_dir}")
        return None
    
    remediation_files.sort()
    latest_remediation = remediation_files[-1]
    latest_path = f"docs/meta/reviews/{branch_name}/{latest_remediation}"
    
    print(f"Branch: {branch_name}")
    print(f"Found remediation files: {remediation_files}")
    print(f"Latest remediation file: {latest_remediation}")
    
    implementer_file = script_dir / "remediation_implementer.md"
    if not implementer_file.exists():
        print(f"Error: Implementer file not found: {implementer_file}")
        return None
    
    template_content = implementer_file.read_text(encoding='utf-8')
    
    output_content = template_content.replace('{{BRANCH_NAME}}', branch_name)
    output_content = output_content.replace('{{REMEDIATION_PATH}}', latest_path)
    
    print(f"\nGenerated prompt for: {latest_path}")
    
    try:
        copy_to_clipboard(output_content)
        print(f"Remediation implementer prompt copied to clipboard!")
    except Exception as e:
        print(f"\nWarning: Could not copy to clipboard: {e}")
    
    return latest_path


def collate_projections(downloads_dir=None):
    script_dir = Path(__file__).parent.resolve()
    repo_root = script_dir.parent.parent.parent
    
    if downloads_dir is None:
        downloads_dir = Path.home() / "Downloads"
    else:
        downloads_dir = Path(downloads_dir)
    
    output_dir = downloads_dir / "projections_context"
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("Collating projections context...")
    print(f"Output directory: {output_dir}")
    
    files_to_copy = []
    
    rust_docs_dir = repo_root / "docs" / "rust_docs"
    excluded_docs = {"2025-11-30-docs-vs-implementation.md"}
    if rust_docs_dir.exists():
        for f in rust_docs_dir.iterdir():
            if f.is_file() and f.suffix == '.md' and f.name not in excluded_docs:
                files_to_copy.append((f, output_dir / f.name))
    
    copied_count = 0
    for src, dst in files_to_copy:
        try:
            shutil.copy2(src, dst)
            print(f"  Copied: {src.name} -> {dst.name}")
            copied_count += 1
        except Exception as e:
            print(f"  Error copying {src.name}: {e}")
    
    competitor_profiles_dir = repo_root / "docs" / "competitor_profiles"
    combined_profiles_path = output_dir / "combined_competitor_profiles.md"
    
    profile_files = []
    if competitor_profiles_dir.exists():
        for f in sorted(competitor_profiles_dir.iterdir()):
            if f.is_file() and f.suffix == '.md':
                profile_files.append(f)
    
    with open(combined_profiles_path, 'w', encoding='utf-8') as f:
        f.write("# Combined Competitor Profiles\n\n")
        f.write("This document consolidates all competitive intelligence research for the Excel/Power BI diff engine market. ")
        f.write("It includes detailed profiles of incumbent tools, their technical architectures, pricing models, ")
        f.write("market positioning, and estimated revenue footprints.\n\n")
        f.write(f"**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"**Total profiles:** {len(profile_files)}\n\n")
        f.write("---\n\n")
        f.write("## Table of Contents\n\n")
        for i, pf in enumerate(profile_files, 1):
            display_name = pf.stem.replace('_', ' ').title()
            f.write(f"{i}. [{display_name}](#{pf.stem})\n")
        f.write("\n---\n\n")
        
        for i, pf in enumerate(profile_files, 1):
            display_name = pf.stem.replace('_', ' ').title()
            f.write(f"<a id=\"{pf.stem}\"></a>\n\n")
            f.write(f"# [{i}/{len(profile_files)}] {display_name}\n\n")
            f.write(f"*Source: `{pf.name}`*\n\n")
            try:
                content = pf.read_text(encoding='utf-8')
                f.write(content)
                if not content.endswith('\n'):
                    f.write('\n')
            except Exception as e:
                f.write(f"(Error reading file: {e})\n")
            f.write("\n\n---\n\n")
    
    print(f"  Created: combined_competitor_profiles.md ({len(profile_files)} profiles)")
    copied_count += 1
    
    print(f"\nCollation complete: {copied_count} files in {output_dir}")
    print("\nFiles included:")
    print(f"  1-{len(files_to_copy)}. Technical blueprints from docs/rust_docs/")
    print(f"  {len(files_to_copy) + 1}.   combined_competitor_profiles.md (all competitor research)")
    
    prompt_file = script_dir / "revenue_projections.md"
    if prompt_file.exists():
        try:
            prompt_text = prompt_file.read_text(encoding='utf-8')
            copy_to_clipboard(prompt_text)
            print(f"\nRevenue projections prompt copied to clipboard!")
        except Exception as e:
            print(f"\nWarning: Could not copy prompt to clipboard: {e}")
    else:
        print(f"\nWarning: Prompt file not found at {prompt_file}")
    
    return output_dir


if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1 and sys.argv[1] == "--timestamps":
        output_file = sys.argv[2] if len(sys.argv) > 2 else "timestamp_report.md"
        generate_timestamp_report(output_file)
    elif len(sys.argv) > 1 and sys.argv[1] == "--collate":
        branch_name = sys.argv[2] if len(sys.argv) > 2 else None
        run_cargo_tests_and_save(branch_name=branch_name)
        generate_review_context()
        collate_post_implementation_review(branch_name)
    elif len(sys.argv) > 1 and sys.argv[1] == "--percent":
        generate_review_context()
        collate_percent_completion()
    elif len(sys.argv) > 1 and sys.argv[1] == "--plan":
        generate_review_context()
        collate_planner()
    elif len(sys.argv) > 1 and sys.argv[1] == "--remediate":
        update_remediation_implementer()
    elif len(sys.argv) > 1 and sys.argv[1] == "--projections":
        collate_projections()
    else:
        generate_review_context()
        print("\nTip: Run with --timestamps [output_file] to generate a document freshness report")
        print("Tip: Run with --collate [branch-name] to collate post-implementation review files")
        print("Tip: Run with --percent to collate percent completion analysis files")
        print("Tip: Run with --plan to collate planner context for next cycle planning")
        print("Tip: Run with --remediate to update remediation_implementer.md with latest remediation file")
        print("Tip: Run with --projections to collate revenue projection analysis context")

