import os
import fnmatch
import re
from datetime import datetime


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

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1 and sys.argv[1] == "--timestamps":
        output_file = sys.argv[2] if len(sys.argv) > 2 else "timestamp_report.md"
        generate_timestamp_report(output_file)
    else:
        generate_review_context()
        print("\nTip: Run with --timestamps [output_file] to generate a document freshness report")

