# Feature List (Refined)

This document consolidates all features from feature_list.md with minimal repetition.

## S. Surfaces, outputs, and privacy
- Interfaces: CLI, desktop GUI (wxDragon), web/WASM demo
- CLI outputs: JSON, JSONL, text, payload, outcome, git_diff-style
- Local execution (files are not uploaded)
- Licensing/trial infrastructure
- Local-first privacy posture

## U. Utility workflows
- Batch compare
- Deep search index patterns
- Audit export

## P. Platform promise (outcomes)
- Compare anything (see D for scope)
- Explain differences (see E and AI for semantic vs noise, root-cause chains, impact scoring)
- Merge safely (see M for 2-way, 3-way, and conflict resolution for tables, models, code)
- Operationalize change (see H, C, G for version history, shareable reviews, approvals, alerts, audit logs, policies)

## D. Diff coverage (catalog)
### D0. Diff primitives and modes
- Grid diffs: cell edits, row operations, column operations, moved blocks
- Keyed database-mode comparisons for table-like sheets
- Auto key suggestion patterns

### D1. Excel workbook and objects
- Workbook structure: sheet add/remove/rename/reorder, hidden/very hidden, protection settings
- Names and connections: defined names + scope (named ranges), external links, workbook connections, Power Pivot / data model presence flags
- Grid content: values, formulas (semantic normalization), errors, types, blanks
- Grid performance: large-sheet performance, streaming comparisons, limits handling, determinism guarantees
- Formatting: cell styles, number formats, fonts, fills, borders, conditional formatting, table styles
- Objects: charts (add/remove/change detection, metadata diffs, v2 chart type + data range deltas), pivot tables, pivot caches, slicers, timelines, data validation rules, shapes, images, SmartArt, sparklines, comments/notes/threaded comments
- Code/advanced: VBA modules (normalized text compare + detection), textual diff views, diff export, Office Scripts, add-in custom functions metadata

### D2. Modern Excel and semantic layer
- Power Query (M): parse, diff, step graph diff, query added/removed/renamed, semantic vs formatting-only changes, metadata changes (load to sheet/model), grouping paths, step-level diffs
- Power Pivot / tabular model: tables, columns, types, properties, calculated columns, relationships, measures, formatting strings, hierarchies, roles
- DAX semantic diffs
- Semantic-noise suppression options for model diffs

### D3. Power BI / Tabular formats
- PBIX/PBIT compare
- PBIP import + compare (folder-to-folder)
- PBIR diff viewer: page/visual diffs, bookmark/selection pane diffs, theme diffs, robust JSON semantic diff (ignore ordering, normalize GUID noise)
- TMDL diff viewer/support
- Report definitions and semantic models
- Diff/merge/insight support for PBIP/PBIR/TMDL repositories

## E. Diff UX, explainability, and controls
- Change Summary panel: counts by sheet, category, severity
- Noise controls: ignore formatting, ignore whitespace-only, collapse moved blocks, semantic-only mode for DAX/M
- "Why did this number change?" view: link pivot output changes to upstream query steps, DAX measures, input table cells
- Saved comparison presets
- Performance instrumentation: time, memory, ops count; show when limits triggered

## M. Merge and conflict resolution
- 2-way apply selected diffs into a target workbook; produce a new merged workbook
- Table-aware merge: key-based row alignment, key column mapping, fuzzy key matching
- 2-way conflict detection: highlight ambiguous merges, require user resolution
- Merge report: applied, skipped, conflicted
- 3-way merge (base + mine + theirs) for table-like sheets, then general grid
- Conflict UI: side-by-side resolution, accept mine, accept theirs, manual edit resolution
- Output merged workbook + conflict log

## H. Versioning and history
- Project concept
- Track versions of a workbook/model
- Show diffs between any two points
- Rollback any version
- Export any version
- Storage backends: local repository, optional Git backend
- Snapshot triggers: manual snapshot, "watchrate with CI"

## C. Collaboration and review workflow
- Shareable diffs (hosted read-only links)
- Comment threads on changes (cells, ranges, queries, measures)
- Approvals, reviewers, change requests, assignments
- Notifications: email; collaboration tool integrations (later)
- Shareable reviews

## G. Monitoring and governance
- Scheduled snapshots and diffs
- Watch folders; watch OneDrive folders; watch SharePoint folders; watch Power BI workspaces (datasets/reports)
- Alert policies: query changed, measure changed, external connection changed, new hidden sheet added
- Change feed
- Audit export
- Logs, permissions, backups, retention
- SSO, RBAC, SCIM
- Immutable audit logs
- Retention policies
- Central inventory of critical spreadsheets and models
- On-prem deployments
- Private cloud deployments
- DLP-friendly architecture

## I. Automation, integrations, and ecosystem
- Automation via CLI (see S) plus CI/CD hooks, APIs, Git difftool integrations
- Git UX kit: repo templates (.gitattributes), recommended difftool config, CI recipes, "Explain diffs in PR language" output mode
- Power BI External Tool entry:
  - Diff current PBIX/PBIT vs "PBected baseline"
  - Diff current vs last exported snapshot
  - Focused view on model/query/report changes
  - Export project artifacts helper
  - Guided export from PBIX to PBIP/PBIR/TMDL
  - Desktop app workspace: remembers last baseline, stores snapshots, supports quick compare
- Plugin SDK: new artifact types, custom diff rules, normalizers, severity scoring
- REST API: submit artifacts, get diff payloads, post review status
- Integrations: CI pipelines, ticketing systems, chat ops

## AI. Insights and intelligence
- Intelligent insights: noise suppression, risk scoring, change impact
- Impact scoring: measure changes used in visuals; query changes affect outputs; high-riskrnal links; hidden sheets; macros
- Noise suppression: auto classify formatting-only changes; auto classify reorder-only changes
- Narrative summaries: "What changed?" and "Why it matters?"
- Rules engine: customizable policies per project and per org

## T. Packaging and tiers
- Free: local compare + view; limited comparisons/month; file-size cap; usable web demo
- Pro: unlimited compares; advanced diff presets; semantic toggles; formatting toggles; CLI; git-friendly output; export HTML, audit workbook, JSON
- Pro+ Merge: apply diffs / 2-way merge; 3-way merge upsell
- Team: shareable diffs; review workflow (comments, approvals); shared history
- Enterprise: monitoring; connectors; alerts; governance suite; SSO; RBAC; logs; retention; on-prem

## Roadmap iteration map (compact)
- Iteration 0 (Baseline): S, U, D0-D2, plus named ranges, charts, VBA detection, and CLI/desktop/web/WASM demo
- Iteration 1: E (change summary, noise controls, why-number-changed view), saved presets, performance instrumentation
- Iteration 2: D3 (PBIP/PBIR/TMDL) + I (Git UX kit, PR-language output)
- Iteration 3: D1 (formatting diffs, pivot diff v1, validation/styles/tables, external links/connections, chart diff v2)
- Iteration 4: M (2-way apply, table-aware merge, conflicts, merge report)
- Iteration 5: M (3-way merge, conflict UI, conflict log)
- Iteration 6: H (project history, storage backends, snapshot triggers)
- Iteration 7: AI (impact scoring, auto noise suppression, narrative summaries, rules engine)
- Iteration 8: G (monitoring, alerts, change feed, audit export)
- Iteration 9: C (shareable diffs, comments, approvals, notifications)
- Iteration 10: G (enterprise governance: SSO/RBAC/SCIM/audit logs/retention/inventory/on-prem/private cloud/DLP)
- Iteration 11: I (plugin SDK, REST API, integrations)
- Iteration 12: I (Power BI External Tool entry + workspace)
