Core compare engine (MVP baseline)
- Grid diffs
- Cell edits
- Row operations
- Column operations
- Moved blocks
- Keyed database-mode comparisons for table-like sheets
- Auto key suggestion patterns
- Power Query (M) diffs
- Query added detection
- Query removed detection
- Query renamed detection
- Query semantic vs formatting-only change detection
- Query metadata change detection
- Step-level query diffs
- Tabular model diffs
- Model table changes
- Model column changes
- Model type changes
- Model property changes
- Calculated column changes
- Relationship changes
- Measure changes
- Semantic-noise suppression options for model diffs
- Object diffs
- Named range diffs
- Chart change detection (hash-based)
- VBA module diffs (normalized text compare)

Front-ends and workflows (MVP baseline)
- CLI output: JSON
- CLI output: JSONL
- CLI output: text
- CLI output: payload
- CLI output: outcome
- git_diff-style output
- Desktop GUI (wxDragon)
- Web/WASM demo
- Local execution (files are not uploaded)
- Batch compare
- Deep search index patterns
- Licensing/trial infrastructure
- Local-first privacy posture

Platform promise (north star)
- Compare workbooks
- Compare PBIX/PBIT
- Compare PBIP projects
- Compare report definitions
- Compare semantic models
- Compare Power Query (M)
- Compare DAX
- Compare VBA
- Compare charts
- Compare pivots
- Compare connections
- Explain differences
- Semantic vs noise classification
- Root-cause chains
- Impact scoring
- Merge safely
- 2-way merges
- 3-way merges
- Conflict resolution for tables
- Conflict resolution for models
- Conflict resolution for code
- Operationalize change
- Version history
- Shareable reviews
- Approvals
- Alerts
- Audit logs
- Policies

Feature universe - Excel surface area coverage
- Workbook structure: sheet add
- Workbook structure: sheet remove
- Workbook structure: sheet rename
- Workbook structure: sheet reordering
- Workbook structure: hidden sheets
- Workbook structure: very hidden sheets
- Workbook structure: protection settings
- Workbook structure: defined names
- Workbook structure: name scope
- Workbook structure: external links
- Workbook structure: workbook connections
- Workbook structure: Power Pivot / data model presence flags
- Grid content: values
- Grid content: formulas
- Grid content: semantic normalization for formulas
- Grid content: errors
- Grid content: types
- Grid content: blanks
- Grid content: large-sheet performance
- Grid content: streaming comparisons
- Grid content: limits handling
- Grid content: determinism guarantees
- Formatting: cell styles
- Formatting: number formats
- Formatting: fonts
- Formatting: fills
- Formatting: borders
- Formatting: conditional formatting
- Formatting: table styles
- Objects: chart add/remove/change detection
- Objects: chart metadata diffs
- Objects: pivot tables
- Objects: pivot caches
- Objects: slicers
- Objects: timelines
- Objects: data validation rules
- Objects: shapes
- Objects: images
- Objects: SmartArt
- Objects: sparklines
- Objects: comments
- Objects: notes
- Objects: threaded comments
- Code/advanced: VBA modules
- Code/advanced: VBA textual diff views
- Code/advanced: VBA diff export
- Code/advanced: Office Scripts
- Code/advanced: add-in custom functions metadata

Feature universe - Modern Excel and BI semantic layer
- Power Query: parse
- Power Query: diff
- Power Query: step graph diff
- Power Query: metadata (load to sheet)
- Power Query: metadata (load to model)
- Power Query: grouping paths
- Power Pivot / Excel tabular: measures
- Power Pivot / Excel tabular: calculated columns
- Power Pivot / Excel tabular: relationships
- Power Pivot / Excel tabular: formatting strings
- Power Pivot / Excel tabular: hierarchies
- Power Pivot / Excel tabular: roles
- DAX semantic diffs

Feature universe - Power BI / Tabular formats
- PBIP support
- PBIR support
- TMDL support
- Diff/merge/insight support for PBIP/PBIR/TMDL repositories

Feature universe - Workflow and platform
- 2-way merge (apply diffs)
- 3-way merge
- Version history
- Rollback
- Shareable diffs
- Comments
- Approvals
- Assignments
- Notifications
- Automation via CLI
- CI/CD hooks
- APIs
- Git difftool integrations
- Monitoring and alerts
- Watch folders
- Watch SharePoint
- Watch Power BI workspaces
- Logs
- Permissions
- Backups
- Retention
- Intelligent insights: noise suppression
- Intelligent insights: risk scoring
- Intelligent insights: change impact

Iteration 0 features
- Grid diff
- Database mode
- Batch compare
- Deep search
- Audit export
- Power Query (M) diffs
- Tabular model diffs
- Named ranges
- Charts
- VBA detection
- CLI
- Desktop app
- Web/WASM local demo

Iteration 1 features
- Change Summary panel
- Change counts by sheet
- Change counts by category
- Change counts by severity
- Noise controls in UI
- Ignore formatting
- Ignore whitespace-only changes
- Collapse moved blocks
- Semantic-only mode for DAX
- Semantic-only mode for M
- "Why did this number change?" view
- Link changed pivot output cell to upstream changed query step
- Link changed pivot output cell to upstream changed DAX measure
- Link changed pivot output cell to upstream changed input table cell
- Saved comparison presets
- Performance instrumentation: time
- Performance instrumentation: memory
- Performance instrumentation: ops count
- Show when limits triggered

Iteration 2 features
- PBIP import and compare (folder-to-folder diff)
- PBIR diff viewer
- PBIR page-level diffs
- PBIR visual-level diffs
- PBIR bookmark diffs
- PBIR selection pane diffs
- PBIR theme diffs
- Robust JSON semantic diff (ignore ordering)
- Robust JSON semantic diff (normalize GUID noise)
- TMDL diff viewer
- Git UX kit
- Repo templates: .gitattributes
- Recommended difftool config
- CI recipes
- "Explain diffs in PR language" output mode

Iteration 3 features
- Formatting diffs (optional / noise-filtered)
- Cell style changes
- Conditional formatting changes
- Number format changes
- Pivot diff v1
- Pivot table definition changes
- Pivot cache changes
- Slicer changes
- Timeline changes
- Basic "pivot output changed" to "source changed" hints
- Data validation diffs
- Named styles diffs
- Table diffs
- External links diffs
- Connections diffs
- Chart diff v2
- Chart type deltas
- Chart data range deltas

Iteration 4 features
- Apply selected diffs into a target workbook
- Produce a new merged workbook
- Table-aware merge
- Key-based row alignment
- Key column mapping
- Fuzzy key matching
- 2-way conflict detection
- Highlight ambiguous merges
- Require user conflict resolution
- Merge report: applied
- Merge report: skipped
- Merge report: conflicted

Iteration 5 features
- 3-way merge (base + mine + theirs)
- 3-way merge for table-like sheets
- 3-way merge for general grid
- Conflict UI
- Side-by-side resolution
- Accept mine
- Accept theirs
- Manual edit resolution
- Output merged workbook
- Conflict log

Iteration 6 features
- Project concept
- Track versions of a workbook/model
- Show diffs between any two points
- Rollback any version
- Export any version
- Local repository backend
- Optional Git backend
- Manual snapshot trigger
- "watchrate with CI" snapshot trigger

Iteration 7 features
- Impact scoring
- Measure change impact: used in which visuals
- Query change impact: affects which outputs
- High-riskrnal links detection
- Hidden sheets risk detection
- Macros risk detection
- Noise suppression: auto classify formatting-only changes
- Noise suppression: auto classify reorder-only changes
- Narrative summaries: "What changed?"
- Narrative summaries: "Why it matters?"
- Rules engine
- Customizable policies per project
- Customizable policies per org

Iteration 8 features
- Scheduled snapshots
- Scheduled diffs
- Watch OneDrive folders
- Watch SharePoint folders
- Watch Power BI workspaces (datasets/reports)
- Alert policy: "Query changed"
- Alert policy: "Measure changed"
- Alert policy: "External connection changed"
- Alert policy: "New hidden sheet added"
- Change feed
- Audit export

Iteration 9 features
- Shareable read-only diff links (hosted)
- Comment threads on changes
- Comment threads on cells
- Comment threads on ranges
- Comment threads on queries
- Comment threads on measures
- Approvals
- Reviewers
- Change requests
- Notifications via email
- Notifications via collaboration tool integrations (later)

Iteration 10 features
- SSO
- RBAC
- SCIM
- Immutable audit logs
- Retention policies
- Central inventory of critical spreadsheets and models
- On-prem deployments
- Private cloud deployments
- DLP-friendly architecture

Iteration 11 features
- Plugin SDK
- Plugin SDK for new artifact types (custom Excel objects)
- Plugin SDK for custom diff rules
- Plugin SDK for normalizers
- Plugin SDK for severity scoring
- REST API
- API: submit artifacts
- API: get diff payloads
- API: post review status
- Integrations with CI pipelines
- Integrations with ticketing systems
- Integrations with chat ops

Iteration 12 features
- Power BI External Tool entry
- External tool: diff current PBIX/PBIT vs "PBected baseline"
- External tool: diff current vs last exported snapshot
- External tool: focused view on model/query/report changes
- Export project artifacts helper
- Guided export from PBIX to PBIP/PBIR/TMDL
- Desktop app workspace concept
- Workspace remembers last baseline
- Workspace stores snapshots
- Workspace supports quick compare

Packaging model features
- Free tier: local compare + view
- Free tier: limited comparisons per month
- Free tier: file-size cap
- Free tier: usable web demo
- Pro tier: unlimited compares
- Pro tier: advanced diff presets
- Pro tier: semantic toggles
- Pro tier: formatting toggles
- Pro tier: CLI access
- Pro tier: git-friendly output
- Pro tier: export HTML
- Pro tier: export audit workbook
- Pro tier: export JSON
- Pro+ Merge tier: apply diffs / 2-way merge
- Pro+ Merge tier: 3-way merge upsell
- Team tier: shareable diffs
- Team tier: review workflow (comments)
- Team tier: review workflow (approvals)
- Team tier: shared history
- Enterprise tier: monitoring
- Enterprise tier: connectors
- Enterprise tier: alerts
- Enterprise tier: governance suite
- Enterprise tier: SSO
- Enterprise tier: RBAC
- Enterprise tier: logs
- Enterprise tier: retention
- Enterprise tier: on-prem
