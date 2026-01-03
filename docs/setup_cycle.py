#!/usr/bin/env python3
"""
Setup script for a new development cycle.

This script automates the creation of cycle artifacts based on the current git branch:
1. Creates a logs directory: docs/meta/logs/[branch-name]/
2. Copies the checklist template to that directory as checklist.md
3. Creates a plans directory: docs/meta/plans/[branch-name]/
   - spec.md (for spec)
   - decision.yaml (for decision record)
4. Creates a reviews directory: docs/meta/reviews/[branch-name]/
   - verification.md (for verification notes)
   - remediation.md (for remediation tracking)

Usage:
    python docs/setup_cycle.py

Run this after creating a new git branch for your development cycle.
"""

import subprocess
import sys
from pathlib import Path


def get_current_branch() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def main():
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent
    
    meta_dir = script_dir / "meta"
    logs_dir = meta_dir / "logs"
    plans_dir = meta_dir / "plans"
    
    branch_name = get_current_branch()
    
    if branch_name in ("master", "main"):
        print(f"Error: You are on the '{branch_name}' branch.")
        print("Please create and checkout a feature branch before running this script.")
        sys.exit(1)
    
    print(f"Setting up cycle for branch: {branch_name}")
    
    branch_logs_dir = logs_dir / branch_name
    if branch_logs_dir.exists():
        print(f"Warning: Logs directory already exists: {branch_logs_dir}")
    else:
        branch_logs_dir.mkdir(parents=True)
        print(f"Created: {branch_logs_dir}")
    
    checklist_template = logs_dir / "cycle_checklist_template.md"
    checklist_dest = branch_logs_dir / "checklist.md"
    
    if checklist_dest.exists():
        print(f"Warning: Checklist already exists: {checklist_dest}")
    else:
        if checklist_template.exists():
            content = checklist_template.read_text(encoding="utf-8")
            content = content.replace("[Branch Name]", branch_name)
            content = content.replace("[branch-name]", branch_name)
            checklist_dest.write_text(content, encoding="utf-8")
            print(f"Created: {checklist_dest}")
        else:
            print(f"Error: Checklist template not found: {checklist_template}")
            sys.exit(1)
    
    branch_plans_dir = plans_dir / branch_name
    if branch_plans_dir.exists():
        print(f"Warning: Plans directory already exists: {branch_plans_dir}")
    else:
        branch_plans_dir.mkdir(parents=True)
        print(f"Created: {branch_plans_dir}")
    
    spec_file = branch_plans_dir / "spec.md"
    if spec_file.exists():
        print(f"Warning: Spec file already exists: {spec_file}")
    else:
        spec_file.write_text(
            f"# Spec: {branch_name}\n\n<!-- Paste spec content here -->\n",
            encoding="utf-8",
        )
        print(f"Created: {spec_file}")
    
    decision_file = branch_plans_dir / "decision.yaml"
    if decision_file.exists():
        print(f"Warning: Decision record already exists: {decision_file}")
    else:
        decision_file.write_text(
            f"# Decision Record: {branch_name}\n# Paste decision YAML content here\n",
            encoding="utf-8",
        )
        print(f"Created: {decision_file}")
    
    reviews_dir = meta_dir / "reviews"
    branch_reviews_dir = reviews_dir / branch_name
    if branch_reviews_dir.exists():
        print(f"Warning: Reviews directory already exists: {branch_reviews_dir}")
    else:
        branch_reviews_dir.mkdir(parents=True)
        print(f"Created: {branch_reviews_dir}")
    
    verification_file = branch_reviews_dir / "verification.md"
    if verification_file.exists():
        print(f"Warning: Verification file already exists: {verification_file}")
    else:
        verification_file.write_text("", encoding="utf-8")
        print(f"Created: {verification_file}")
    
    remediation_file = branch_reviews_dir / "remediation.md"
    if remediation_file.exists():
        print(f"Warning: Remediation file already exists: {remediation_file}")
    else:
        remediation_file.write_text("", encoding="utf-8")
        print(f"Created: {remediation_file}")
    
    completion_estimates_dir = meta_dir / "completion_estimates"
    if not completion_estimates_dir.exists():
        completion_estimates_dir.mkdir(parents=True)
        print(f"Created: {completion_estimates_dir}")
    
    for suffix in ("gemini", "openai"):
        estimate_file = completion_estimates_dir / f"{branch_name}-{suffix}.md"
        if estimate_file.exists():
            print(f"Warning: Completion estimate already exists: {estimate_file}")
        else:
            estimate_file.write_text("", encoding="utf-8")
            print(f"Created: {estimate_file}")
    
    print("\nCycle setup complete!")
    print(f"\nNext steps:")
    print(f"  1. Paste your decision YAML into: {decision_file.relative_to(repo_root)}")
    print(f"  2. Paste your spec into: {spec_file.relative_to(repo_root)}")
    print(f"  3. Track progress using: {checklist_dest.relative_to(repo_root)}")
    print(f"  4. Document review in: {branch_reviews_dir.relative_to(repo_root)}/")


if __name__ == "__main__":
    main()

