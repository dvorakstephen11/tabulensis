import os
import fnmatch

def generate_review_context(output_file="review_prompt.md", root_dir="../../../"):
    # Adjust root_dir relative to where this script is: docs/meta/prompts/ -> repo root is ../../../
    # If running from repo root, it would be "."
    
    # Determine absolute path to repo root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(script_dir, root_dir))
    
    output_path = os.path.join(script_dir, output_file)
    
    # Define what to include
    included_extensions = {
        '.rs', '.py', '.toml', '.yaml', '.yml', '.gitignore', '.txt', '.md'
    }
    
    # Explicitly ignore these directories
    ignored_dirs = {
        'target', '.git', '.cursor', 'node_modules', '__pycache__', 
        '.idea', '.vscode', 'venv', 'env', 'terminals', 'debug', 'incremental'
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
    generate_review_context()

