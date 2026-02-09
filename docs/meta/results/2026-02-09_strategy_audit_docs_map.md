# Strategy Audit Docs Map (2026-02-09)

Run date: 2026-02-09

This file is the corpus coverage tracker for executing `docs/meta/strategy_audit_plan.md`.

Legend:
- Category: `operating` | `runbook` | `checklist` | `guide` | `reference` | `config` | `script` | `log` | `results`
- Canonical candidate: `yes` | `no` | `unknown`
- Read status: `pending` | `skimmed` | `close-read` | `reconciled`

## Docs Map Table

| Path | Category | Canonical candidate | Read status | Notes (why it matters) |
| --- | --- | --- | --- | --- |
| `docs/index.md` | operating | yes | close-read | Canonical doc entrypoint + checklist auto-index. |
| `README.md` | reference | yes | skimmed | Primary product description + install + quick start. |
| `APP_INTENT.md` | operating | yes | skimmed | Declares MVP vs end-state intent. |
| `AGENTS.md` | operating | yes | skimmed | Repo guardrails + perf/formatting policies for changes. |
| `meta_methodology.md` | operating | yes | skimmed | Daily operator routine and meta-system goals. |
| `product_roadmap.md` | operating | yes | skimmed | Strategy roadmap and iteration plan (Iteration 0/1/2...). |
| `REPO_STATE.md` | operating | yes | close-read | Current state snapshot, risks/debt, next tasks. |
| `OPERATING_AGREEMENT.md` | operating | yes | skimmed | Business/legal operating context (LLC). |
| `todo.md` | operating | unknown | pending | Backlog; influences daily plan triage. |
| `docs/meta/README.md` | operating | yes | skimmed | Meta-docs index + guardrails summary. |
| `docs/meta/strategy_audit_plan.md` | operating | yes | close-read | The audit process being executed. |
| `docs/meta/logs/README.md` | operating | yes | skimmed | Logging conventions (daily/research/ops). |
| `docs/meta/results/README.md` | operating | yes | skimmed | Results conventions (deep research captures, perf reports). |
| `docs/meta/prompts/README.md` | operating | yes | skimmed | Prompt library index + tooling guidance. |
| `docs/meta/prompts/pro_context_payload.md` | operating | yes | pending | Spec for “ultimate context payload”. |
| `docs/meta/today.md` | operating | yes | close-read | Local-only daily scratchpad (gitignored); derived from `docs/meta/today.example.md`. |
| `docs/meta/today.example.md` | operating | yes | close-read | Committed template for `docs/meta/today.md`. |
| `docs/meta/prompts/daily_plan.md` | checklist | yes | pending | Daily-plan prompt + checklist (used during daily open). |
| `docs/meta/prompts/deep_research_synthesis.md` | checklist | yes | pending | Synthesis prompt + checklist (A/B -> synthesis). |
| `docs/meta/prompts/deep_research_market_analysis.md` | operating | yes | pending | Deep research prompt: market analysis. |
| `docs/meta/prompts/deep_research_acquisition_targets.md` | operating | yes | pending | Deep research prompt: acquisition targets. |
| `docs/meta/prompts/deep_research_competitor_watch.md` | operating | yes | pending | Deep research prompt: competitor watch. |
| `docs/meta/prompts/deep_research_distribution_experiments.md` | operating | yes | pending | Deep research prompt: distribution experiments. |
| `docs/meta/prompts/deep_research_security_watch.md` | operating | yes | pending | Deep research prompt: security watch. |
| `docs/meta/prompts/deep_research_ops_dashboard_apis.md` | operating | yes | pending | Deep research prompt: ops dashboard APIs. |
| `docs/meta/prompts/generate_review_context.py` | script | yes | pending | Prompt tooling referenced by prompt library. |
| `docs/meta/audio/README.md` | operating | yes | close-read | Audio (TTS) operator workflow. |
| `docs/meta/marketing_templates/README.md` | operating | yes | close-read | Marketing templates index + measurement conventions (UTM scheme + metrics file). |
| `docs/meta/marketing_templates/post_short_demo.md` | operating | yes | skimmed | Short demo post template. |
| `docs/meta/marketing_templates/post_before_after.md` | operating | yes | skimmed | Before/after post template. |
| `docs/meta/marketing_templates/post_user_story.md` | operating | yes | skimmed | Persona/user story post template. |
| `docs/meta/automation/overnight_operator_agent_plan.md` | operating | yes | skimmed | Design/invariants for overnight operator agent. |
| `docs/meta/automation/overnight_agent_runbook.md` | runbook | yes | close-read | How to run overnight agent; safety model; outputs. |
| `docs/meta/automation/overnight_agent.yaml` | config | yes | close-read | Overnight agent wiring/config (provider/model/pipeline). |
| `docs/meta/automation/README.md` | operating | yes | close-read | Automation index + output locations + failure recording. |
| `docs/meta/automation/guardrails.md` | operating | yes | close-read | Central automation forbidden-actions + safety properties (includes security tooling choice). |
| `docs/meta/automation/model_accounts.md` | operating | yes | skimmed | Model account inventory + capacity planning decisions. |
| `scripts/overnight_agent.py` | script | yes | skimmed | Overnight agent implementation (config schema + behavior). |
| `scripts/deep_research_prompt.py` | script | yes | skimmed | Copies prompts to clipboard/prints; must match docs. |
| `scripts/update_docs_index_checklists.py` | script | yes | close-read | Maintains `docs/index.md` checklist index. |
| `docs/operations.md` | operating | yes | close-read | Operator essentials and SOPs. |
| `docs/licensing_service.md` | operating | yes | close-read | Licensing backend (worker + service) SOP. |
| `STRIPE_WORKER_NEXT_STEPS.md` | operating | yes | pending | Stripe + licensing worker next steps (no checkboxes; still an operating plan). |
| `RESEND_SETUP_CHECKLIST.md` | checklist | yes | pending | Resend setup; license delivery email wiring. |
| `docs/release_checklist.md` | operating | yes | close-read | Release operations checklist/SOP. |
| `docs/release_readiness.md` | operating | yes | pending | Release readiness checklist/SOP. |
| `docs/release_signing.md` | operating | yes | close-read | Signing/notarization SOP (now includes current CI state). |
| `docs/auto_update_strategy.md` | operating | yes | skimmed | Auto-update strategy and constraints. |
| `docs/installer_ux.md` | operating | yes | skimmed | Installer UX plan. |
| `SECURITY_DAILY_CHECKLIST.md` | checklist | yes | close-read | Minimal recurring security checks (dependency audit/secret scan/SAST). |
| `.github/pull_request_template.md` | checklist | yes | pending | PR checklist; quality gates. |
| `.github/workflows/release.yml` | config | yes | close-read | Canonical release pipeline (repo reality evidence). |
| `docs/desktop_tab_switch_latency_plan.md` | checklist | yes | pending | Desktop performance plan (tab switch latency). |
| `docs/desktop_perf_ux_improvement_plan.md` | guide | yes | pending | Desktop performance + UX improvements. |
| `docs/desktop_native_ui_ux_roadmap.md` | guide | yes | pending | Native UI/UX roadmap (XRC). |
| `docs/ui_visual_regression.md` | guide | yes | pending | Visual regression guidance. |
| `docs/ui_visual_regression_manual.md` | guide | yes | pending | Manual visual regression workflow. |
| `docs/ui_visual_regression_plan.md` | guide | yes | pending | Visual regression plan/checklist. |
| `docs/desktop.md` | guide | yes | close-read | Desktop app build/run docs. |
| `docs/cli.md` | guide | yes | pending | CLI reference. |
| `docs/config.md` | guide | yes | pending | Library configuration guide. |
| `docs/migration.md` | guide | yes | pending | Migration guide. |
| `docs/git.md` | guide | yes | pending | Git integration guide. |
| `docs/database_mode.md` | guide | yes | pending | Database mode guide. |
| `docs/faq.md` | guide | yes | pending | FAQ. |
| `docs/architecture.md` | guide | yes | pending | Architecture overview. |
| `docs/perf_playbook.md` | guide | yes | close-read | Perf validation and playbook. |
| `docs/real_world_datasets_perf_plan.md` | checklist | yes | pending | Dataset procurement + perf/test program checklist. |
| `docs/test_suite_excellence_plan.md` | guide | yes | pending | Test suite improvement plan. |
| `docs/openxml_coverage.md` | guide | yes | pending | OpenXML coverage plan. |
| `docs/formula_coverage.md` | guide | yes | pending | Formula coverage plan. |
| `docs/diff_op_coverage.md` | guide | yes | pending | Diff-op coverage plan. |
| `docs/m_parser_coverage.md` | guide | yes | pending | Power Query M parser coverage plan. |
| `docs/rust_docs/mvp_checklist.md` | checklist | yes | pending | “Ship to real users” MVP checklist (bulleted checkboxes). |
| `docs/rust_docs/mvp_fulfillment_log.md` | checklist | yes | pending | Progress/decisions against `mvp_checklist.md`. |
| `docs/rust_docs/custom_crates/README.md` | guide | yes | pending | Custom-crate experiments index. |
| `docs/rust_docs/custom_crates/agentic_experiment_playbook.md` | guide | yes | pending | How to run custom-crate experiments. |
| `docs/product_iterations/iteration_1_daily_driver_plan.md` | checklist | yes | pending | Iteration 1 plan. |
| `docs/product_iterations/iteration_2_readiness.md` | checklist | yes | pending | Iteration 2 readiness checklist. |
| `docs/product_iterations/iteration_2_implementation_plan.md` | checklist | yes | pending | Iteration 2 implementation plan. |
| `docs/maintainers/entrypoints.md` | guide | yes | pending | Maintainer entrypoints and code map. |
| `docs/maintainers/fixtures.md` | guide | yes | pending | Fixture generation notes. |
| `docs/maintainers/architecture.md` | guide | yes | pending | Maintainer architecture notes. |
| `docs/maintainers/architecture_debt_post_iteration_1.md` | checklist | yes | pending | Architecture debt plan and checklist. |
| `fixtures/README.md` | guide | yes | pending | Fixture generator + manifests. |
| `benchmarks/README.md` | guide | yes | pending | Perf harness usage. |
| `desktop/ui_scenarios/README.md` | guide | yes | pending | Desktop UI scenarios. |
| `desktop/ui_snapshots/README.md` | guide | yes | pending | Desktop UI snapshot system. |
| `docs/errors.md` | reference | yes | pending | Error code reference. |
| `docs/ui_guidelines.md` | reference | yes | pending | UI guidelines. |
| `tabulensis_launch_to_dos_from_our_chat.md` | checklist | unknown | pending | Launch/vendor backlog from earlier planning. |
| `docs/rust_docs/` | reference | yes | pending | Directory: Rust deep dives/workstreams. |
| `docs/competitor_profiles/` | reference | yes | pending | Directory: competitor research. |
| `docs/finances_sales_marketing_legal/` | reference | yes | pending | Directory: finance/sales/marketing/legal notes. |
| `docs/meta/logs/` | reference | yes | skimmed | Directory: operator logs (daily/research/ops). |
| `docs/meta/results/` | reference | yes | skimmed | Directory: captured results (deep research, perf, etc.). |
| `scripts/utm.py` | script | yes | close-read | UTM generator for marketing attribution. |
| `scripts/security_audit.sh` | script | yes | close-read | Wrapper script to run minimal security checks and write ops report. |
| `scripts/docs_integrity.py` | script | yes | close-read | Docs integrity checks (docs/index links, checklist drift, decision-gate surfacing). |

## Unfinished Checklist Inventory (from `docs/index.md`)

Note: this section is a copyable working list; treat the `docs/index.md` checklist block as source-of-truth.

- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (checklist)
- `RESEND_SETUP_CHECKLIST.md` (checklist)
- `SECURITY_DAILY_CHECKLIST.md` (checklist)
- `tabulensis_launch_to_dos_from_our_chat.md` (checklist)
- `.github/pull_request_template.md` (checklist)
- `docs/desktop_tab_switch_latency_plan.md` (checklist)
- `docs/meta/prompts/daily_plan.md` (checklist)
- `docs/meta/prompts/deep_research_synthesis.md` (checklist)
- `docs/meta/today.example.md` (checklist)
- `docs/rust_docs/mvp_checklist.md` (checklist)
- `docs/rust_docs/mvp_fulfillment_log.md` (checklist)
- `docs/real_world_datasets_perf_plan.md` (checklist)
- `docs/maintainers/architecture_debt_post_iteration_1.md` (checklist)
- `docs/product_iterations/iteration_1_daily_driver_plan.md` (checklist)
- `docs/product_iterations/iteration_2_readiness.md` (checklist)
- `docs/product_iterations/iteration_2_implementation_plan.md` (checklist)

## Discovery Deltas (checkbox files not in index)

Populate this after running checkbox discovery and diffing against the auto-index list.

Notes:
- Some repo checklists use `* [ ]` instead of `- [ ]` (example: `docs/rust_docs/mvp_checklist.md`), so checkbox discovery should use a pattern like: `rg -l "^\\s*[-*]\\s+\\[\\s*\\]"`.

- (none recorded yet)
- Expected exclusion:
  - `docs/meta/today.md` contains checkboxes but is gitignored (local-only scratchpad), so it should not appear in `docs/index.md` auto-index.
