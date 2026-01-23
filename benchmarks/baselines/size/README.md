Size baselines for release artifacts live in this directory.

Update workflow:
- Generate size reports with `scripts/size_report.py`.
- Promote reports to baselines with `scripts/update_size_baselines.py`.
- Enforce budgets with `scripts/check_size_budgets.py`.
