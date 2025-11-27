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
                ['powershell', '-command', f'Get-Content -Raw -Path "{temp_path}" | Set-Clipboard'],
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
    if rust_docs_dir.exists():
        for f in rust_docs_dir.iterdir():
            if f.is_file() and f.suffix == '.md':
                files_to_copy.append((f, output_dir / f.name))
    
    plans_branch_dir = repo_root / "docs" / "meta" / "plans" / branch_name
    if plans_branch_dir.exists():
        spec_file = plans_branch_dir / "spec.md"
        if spec_file.exists():
            files_to_copy.append((spec_file, output_dir / f"spec_{branch_name}.md"))
        decision_file = plans_branch_dir / "decision.yaml"
        if decision_file.exists() and decision_file.stat().st_size > 0:
            files_to_copy.append((decision_file, output_dir / f"decision_{branch_name}.yaml"))
    
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
        "excel_diff_technical_document.md",
        "excel_diff_testing_plan.md",
        "excel_diff_difficulty_analysis.md",
        "excel_diff_m_query_parse.md",
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
        "excel_diff_technical_document.md",
        "excel_diff_testing_plan.md",
        "excel_diff_difficulty_analysis.md",
        "excel_diff_m_query_parse.md",
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
    
    plans_dir = repo_root / "docs" / "meta" / "plans"
    combined_decisions_path = output_dir / "combined_decision_records.txt"
    
    decision_records = []
    if plans_dir.exists():
        for branch_dir in sorted(plans_dir.iterdir()):
            if branch_dir.is_dir():
                decision_file = branch_dir / "decision.yaml"
                if decision_file.exists():
                    branch_name = branch_dir.name
                    try:
                        content = decision_file.read_text(encoding='utf-8')
                        decision_records.append((branch_name, content))
                    except Exception:
                        pass
    
    with open(combined_decisions_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("COMBINED DECISION RECORDS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Total decision records: {len(decision_records)}\n\n")
        
        if decision_records:
            for branch_name, content in decision_records:
                f.write("-" * 60 + "\n")
                f.write(f"BRANCH: {branch_name}\n")
                f.write("-" * 60 + "\n\n")
                f.write(content)
                if not content.endswith('\n'):
                    f.write("\n")
                f.write("\n")
        else:
            f.write("(No decision records found)\n")
    
    print(f"  Created: combined_decision_records.txt ({len(decision_records)} records)")
    copied_count += 1
    
    reviews_dir = repo_root / "docs" / "meta" / "reviews"
    combined_verifications_path = output_dir / "recent_verification_reports.txt"
    
    verification_reports = []
    if reviews_dir.exists():
        for branch_dir in sorted(reviews_dir.iterdir(), reverse=True):
            if branch_dir.is_dir():
                verification_file = branch_dir / "verification.md"
                if verification_file.exists():
                    branch_name = branch_dir.name
                    try:
                        content = verification_file.read_text(encoding='utf-8')
                        verification_reports.append((branch_name, content))
                    except Exception:
                        pass
    
    recent_verifications = verification_reports[:3]
    
    with open(combined_verifications_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("RECENT VERIFICATION REPORTS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Showing {len(recent_verifications)} most recent (of {len(verification_reports)} total)\n\n")
        
        if recent_verifications:
            for branch_name, content in recent_verifications:
                f.write("-" * 60 + "\n")
                f.write(f"BRANCH: {branch_name}\n")
                f.write("-" * 60 + "\n\n")
                f.write(content)
                if not content.endswith('\n'):
                    f.write("\n")
                f.write("\n")
        else:
            f.write("(No verification reports found)\n")
    
    print(f"  Created: recent_verification_reports.txt ({len(recent_verifications)} reports)")
    copied_count += 1
    
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
    
    latest_results_path = output_dir / "latest_test_results.txt"
    with open(latest_results_path, 'w', encoding='utf-8') as f:
        f.write("=" * 60 + "\n")
        f.write("LATEST TEST RESULTS\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        if latest_results:
            branch_name, content = latest_results
            f.write(f"Branch: {branch_name}\n")
            f.write("-" * 60 + "\n\n")
            f.write(content)
        else:
            f.write("(No test results found)\n")
    
    print(f"  Created: latest_test_results.txt")
    copied_count += 1
    
    print(f"\nCollation complete: {copied_count} files in {output_dir}")
    print("\nFiles included:")
    print("  1-6. Technical blueprints from docs/rust_docs/")
    print("  7.   codebase_context.md (current code snapshot)")
    print("  8.   todo.md (current task list)")
    print("  9.   combined_activity_logs.txt (all branch activity)")
    print("  10.  combined_decision_records.txt (all cycle decisions)")
    print("  11.  recent_verification_reports.txt (last 3 reviews)")
    print("  12.  latest_test_results.txt (most recent test output)")
    
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


if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1 and sys.argv[1] == "--timestamps":
        output_file = sys.argv[2] if len(sys.argv) > 2 else "timestamp_report.md"
        generate_timestamp_report(output_file)
    elif len(sys.argv) > 1 and sys.argv[1] == "--collate":
        branch_name = sys.argv[2] if len(sys.argv) > 2 else None
        generate_review_context()
        collate_post_implementation_review(branch_name)
    elif len(sys.argv) > 1 and sys.argv[1] == "--percent":
        generate_review_context()
        collate_percent_completion()
    elif len(sys.argv) > 1 and sys.argv[1] == "--plan":
        generate_review_context()
        collate_planner()
    else:
        generate_review_context()
        print("\nTip: Run with --timestamps [output_file] to generate a document freshness report")
        print("Tip: Run with --collate [branch-name] to collate post-implementation review files")
        print("Tip: Run with --percent to collate percent completion analysis files")
        print("Tip: Run with --plan to collate planner context for next cycle planning")

