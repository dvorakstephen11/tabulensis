# Meta Development Guide

This document defines the development process for the Excel / Power BI diff engine. It is written for both automated agents and human contributors. Any agent operating on this codebase should treat this as the authoritative description of how work is selected, specified, implemented, evaluated, and improved over time.

The process assumes:

* A planner model (for example, GPT-5.1-Pro or Deepthink) that can read the codebase, documentation, test results, and logs and make structured decisions.
* An implementer model (for example, Gemini 3 Pro or GPT-5.1-Codex-Max-XHigh) that edits code and tests.
* A test harness that can run unit, integration, performance, and scenario-level checks.
* A version control system (GitHub) where releases are tagged.

The goal of this process is to keep the system aligned with its technical blueprints and product goals while letting AI agents drive most of the day-to-day work.

---

## 1. Product context and constraints

The core product is a diff engine for large Excel and Power BI artifacts. It:

* Parses workbooks (and later PBIX/PBIT) into a structured internal representation (IR).
* Extracts and understands Power Query M from DataMashup payloads.
* Eventually understands data models and DAX.
* Performs hierarchic diffs across workbook structure, grids (database and spreadsheet modes), queries, models, and metadata.
* Must stay performant and memory-safe on large files (on the order of 100 MB), with streaming and near-linear algorithms where possible.
* Is designed to run cross-platform and integrate into various workflows.

All decisions and actions in this guide are in service of those goals.

---

## 2. Roles in the process

The process mentions several roles. A single person or agent may play multiple roles in practice, but the responsibilities stay distinct.

### Planner Agent

The planner agent chooses what to work on next and expresses that choice in structured artifacts. It:

* Reads the current code, recent test and performance results, dev logs, and product documentation.
* Compares the current implementation to the technical blueprints and testing milestones.
* Chooses whether the next cycle should:

  * Refactor to improve codebase health.
  * Develop code toward an existing milestone.
  * Propose a new, more incremental milestone on the way to an existing one.
* Writes a decision record and a mini-spec, and defines or extends tests.
* Estimates the difficulty of the work compared to all proposed milestones past, present, and future (indexed at 10 for the hardest proposed milestone, which it should identify by name).

### Implementer Agent

The implementer agent turns the plan into code and tests. It:

* Consumes the mini-spec and test definitions.
* Edits source files and test files to implement the plan.
* Writes its own activity log (.txt) while it works.
* Prepares the change for evaluation (builds, scripts, configuration).

### Reviewer Agent

The reviewer agent acts as an automated counterpart to code review. It:

* Examines the diff in the context of the spec, tests, and architecture.
* Interprets static analysis and compiler output.
* Flags potential invariant violations, complexity issues, and coverage gaps.

### Post-Implementation Verification Reviewer

The post-implementation verification reviewer is a fresh agent instance that operates after implementation and testing. It:

* Receives the same documentation as the original planner plus the implementation plan and activity log.
* Compares what was planned against what was actually implemented.
* Verifies that all specified tests were created and are meaningful.
* Hunts for hidden bugs, gaps, and edge cases that existing tests may not catch.
* Produces a verification report with findings and severity assessments.
* Creates a remediation plan if issues are found that must be fixed before release.

This role exists because tests alone cannot guarantee correctness. The verification reviewer provides a second set of eyes that specifically looks for discrepancies between intent and execution.

### Test Runner

The test runner can be a person or an agent that controls the automated tests. It:

* Runs unit, integration, performance, and scenario tests.
* Collects structured results for later analysis by the planner and reviewer.

### Human Maintainer

The human maintainer is responsible for the overall direction of the project. They:

* Approve changes to the meta process and prompts.
* Decide when to cut releases.
* Resolve ambiguity or conflict between agents.

---

## 3. The development cycle

The development cycle is repeated continuously. Each cycle is defined by:

* A clear decision about the work type.
* A small, precise spec and test plan.
* An implementation guarded by checks and review.
* Structured evaluation, including scenario-level behavior.
* A release decision.
* A retrospective and potential updates to process and prompts.

### 3.1 Planning and decision records

At the start of a cycle, the planner agent evaluates the current state:

* It reads:

  * Technical blueprints and architecture docs.
  * Testing plan and milestones.
  * Recent dev logs and previous decision records.
  * Latest test and performance outputs.
* It compares the current implementation to the intended architecture and planned milestones.
* It considers the current risk picture (for example, which subsystems are inherently difficult or still immature).

The planner then chooses one of three work types:

* **Refactor**: improve internal structure, reduce complexity, or align code with architecture without changing external behavior.
* **Milestone progress**: implement work toward an existing testing or product milestone.
* **New incremental milestone**: define a smaller milestone that sits between the current state and a larger existing milestone, when the gap is too large to cross in a single cycle.

This choice is recorded in a decision file in a branch-named subdirectory, for example:

`plans/feature-grid-alignment/decision.yaml`

A typical decision record contains:

* The chosen work type.
* The primary target subsystem (for example, grid diff, M parser, DataMashup host, semantic diff).
* A brief list of reasons grounded in documentation and observed test behavior.
* Risks of deferring this work.
* Pointers to relevant design and test documents.

Decision records are append-only. They provide a historical trace of why each cycle focussed where it did.

### 3.2 Specification and tests-first

Once the work type is chosen, the planner writes a mini-spec for the cycle, stored in the same branch-named subdirectory as the decision record (for example, `plans/feature-grid-alignment/spec.md`).

The mini-spec describes:

* The scope of the change:

  * Which types and modules are in play.
  * What behavior is intended to change, at a high level.
* The behavioral contract:

  * Plain language examples of expected behavior before and after the change.
  * For diff behavior, this usually means describing what diff operations should appear for particular kinds of source changes.
* Constraints:

  * Complexity and performance expectations.
  * Memory or streaming constraints.
  * Any invariants that must stay true (for example, consistency of the internal representation).
* Interfaces:

  * Any public APIs or IR types that are allowed to change.
  * Any that must remain stable for this cycle.

In addition, the mini-spec must link the work to a testing milestone:

* Each change is associated with at least one existing testing milestone, or introduces a new incremental milestone if needed.
* The planner defines concrete tests that express the milestone:

  * New test cases to add.
  * Existing tests to extend.
  * Fixture pairs to create or modify.
  * Any new metrics to capture.

These test descriptions are written in a form that the implementer can turn into actual test code. The key rule is that no cycle is “just code”; every cycle must be grounded in explicit tests.

---

## 4. Implementation with guardrails

After the mini-spec and tests are defined, the implementer agent takes over. Its input is:

* The current codebase.
* The mini-spec for this cycle.
* The referenced sections of the architecture and testing documents.
* Any implementation guidelines from its own prompt file.

### 4.1 Activity logging

During implementation, the agent writes a log to a plain text file under a dedicated subdirectory in `logs/`.

This subdirectory must be named exactly matching the current git branch name (e.g., `logs/feature-excel-parsing-v1/`).

Inside this directory, the following files are required:

1.  `activity_log.txt`: The running log of actions taken during implementation.

The standard implementer prompt is stored at `prompts/implementer.md` and is used together with the cycle's mini-spec to initiate implementation. If a cycle requires a customized prompt, it may be stored as `prompt_original.md` in this directory.

The planning artifacts (decision record and mini-spec) are stored in `plans/[branch-name]/` as described in Section 3.

The activity log includes:

* Files touched and why.
* Key structural decisions (for example, new helper abstractions, changes to IR).
* Deviations from the mini-spec and reasons, if any.
* Open questions or follow-ups for future cycles.

Logs are intended for later analysis in retrospectives and by the planner.

### 4.2 Static analysis and compile checks

Before moving on to test execution, the change must pass basic quality gates. These include, at minimum:

* Successful compilation of all relevant components.
* Code formatting tools (for example, `cargo fmt`) run clean.
* Static analysis tools (for example, `clippy`, lints) run and any warnings are either fixed or explicitly recorded with justification.

If static tools report significant issues, the implementer is expected to address them within the same cycle, or to annotate the decision record and spec with the reasons they could not be addressed.

### 4.3 Automated review

Once the change builds cleanly, the reviewer agent examines:

* The code diff.
* The mini-spec and test plan.
* The relevant architecture sections.

The reviewer checks for:

* Violations of stated invariants in the spec and architecture.
* Accidental growth in algorithmic complexity (such as linear scans replaced by nested loops over large structures).
* New public interfaces that duplicate or conflict with existing IR concepts.
* Test coverage gaps, especially around edge cases mentioned in documentation.

The reviewer produces a short report for the cycle, which is stored alongside the decision record. If severe flaws are found, the change is sent back to the implementer for correction before proceeding.

---

## 5. Evaluation: tests, performance, and scenarios

After implementation and review, the change is evaluated by running automated checks. The evaluation is not limited to unit tests; it also includes performance metrics and scenario-level behavior.

### 5.1 Standard tests and metrics

The test runner executes:

* Unit tests in the core libraries.
* Integration tests that exercise full diff pipelines.
* Tests associated with the specific testing milestone described in the mini-spec.
* Performance tests on designated large or complex fixtures.

The outcome is written in a structured artifact under a branch-specific subdirectory, for example:

`results/feature-excel-parsing-v1/test_2025-11-25.yml`

The subdirectory must be named exactly matching the current git branch name, consistent with the activity logs structure.

This record includes, for each suite:

* The status (pass/fail).
* The list of tests added or changed in this cycle.
* Key performance measurements and whether they remain within established budgets.
* Any failures, with pointers to logs.

The planner and reviewer use this record in future cycles.

### 5.2 Scenario book

In addition to tests defined in the testing plan, the project maintains a “scenario book.” A scenario describes a real workflow or use case that the engine must support. Each scenario includes:

* A human-readable description of the user’s intent.
* A set of input artifacts (for example, workbook pairs, PBIX files).
* An expectation for how the diff result should look at a high level.

Scenarios are stored under `scenarios/` with a machine-readable format that:

* Points to fixture files.
* Encodes high-level expectations (for example, “treat this as a database reconciliation”, “surface semantic query changes”, “highlight measure changes rather than raw grid differences”).

Whenever a change affects subsystems that participate in a scenario, the relevant scenarios must be re-run as part of evaluation. An agent interprets the diff outputs and verifies that:

* The core story of each scenario remains intact.
* No major regressions or new noise are introduced.
* Any improvements align with the scenario description.

If a scenario fails, the change is rejected or the scenario is updated only after the failure is well-understood and the change in behavior is considered desirable.

### 5.3 Structured evaluation outcome

At the end of evaluation, the project records a consolidated outcome for the cycle, typically as a small summary in the decision file or a separate `results/` entry. This includes:

* A summary of test and scenario outcomes.
* Whether performance remains within budget.
* Any known issues that must be addressed in future cycles.

Only if the evaluation is acceptable does the cycle proceed to post-implementation verification.

### 5.4 Post-implementation verification review

After the initial implementation passes tests and evaluation, a second reviewer agent performs an independent verification review. This step exists because:

* Tests may be improperly written or incomplete.
* Hidden bugs may exist that no existing test reveals.
* The implementation plan may have specified tests that were never actually created.
* Subtle gaps between the spec and the implementation may have been overlooked.

The post-implementation reviewer is a fresh agent instance that receives:

* The same documentation that the original planner received.
* The same codebase snapshot (now reflecting the implemented changes).
* The original decision record and mini-spec produced by the planner.
* The implementer's activity log.
* The test results from evaluation.

The post-implementation reviewer's goal is to find discrepancies, gaps, or bugs by comparing:

* What the plan said would be done versus what was actually done.
* What tests were supposed to be added versus what tests exist.
* Whether the implementation adheres to the behavioral contract in the mini-spec.
* Whether any edge cases or error paths were neglected.
* Whether the code introduces risks not anticipated in the original plan.

#### Verification review output

The post-implementation reviewer produces artifacts stored under a branch-specific reviews directory, for example:

`reviews/feature-excel-parsing-v1/verification.md`

The verification report contains:

* A summary of findings (gaps, bugs, missing tests, deviations from spec).
* A severity assessment for each finding (critical, moderate, minor).
* A recommendation: either "proceed to release" or "remediation required."

If remediation is required, the post-implementation reviewer also produces a remediation plan:

`reviews/feature-excel-parsing-v1/remediation.md`

This plan contains:

* A description of what must be fixed and why.
* Specific code changes or additions needed.
* Tests to add or modify to cover the identified gaps.

The remediation plan is then executed by the implementer agent in a follow-up pass within the same cycle. If multiple rounds of remediation are needed, subsequent plans are numbered:

* `remediation.md` — first remediation
* `remediation-1.md` — second remediation
* `remediation-2.md` — and so on

After each remediation, tests are re-run and the verification review may be repeated. Only after the final verification report recommends proceeding does the cycle move to release. The final `verification.md` file serves as the sign-off for release.

---

## 6. Release

When a cycle’s implementation passes static checks, review, tests, performance checks, and scenario verification, the code is eligible for release.

A release consists of:

* Merging the change into the main branch.
* Tagging a release version in Git with an appropriate semantic version.
* Optionally updating release notes to mention which milestone progressed or completed in this cycle.

The decision record and test results referenced by the tag make it possible to reconstruct why the release was cut and what it contains.

---

## 7. Architecture and risk governance

Some aspects of the system are inherently challenging: grid alignment on large spreadsheets, streaming parsing, semantic diff of M, robust handling of malformed or unusual inputs, and so on. These are tracked as explicit “difficulty items” in a risk ledger.

### 7.1 Risk ledger

The risk ledger is a document (for example, `risk/hurdles.yml`) that lists each known difficult area along with:

* A short description of the challenge.
* Its importance to the product.
* A maturity level (for example, from “not started” to “hardened”).
* A qualitative risk rating.

On a regular schedule (for example, weekly), a planner or reviewer agent:

* Reviews recent code changes and tests.
* Updates the maturity and risk ratings based on actual progress.
* Notes any new challenges that have emerged.

The ledger is then used as input to future planning cycles. High-risk and low-maturity areas should be favored when choosing work types and targets.

### 7.2 Architecture drift review

Alongside risk assessment, the project performs an architecture drift review. In this review, an agent compares:

* The current codebase structure and public interfaces.
* The intended architecture described in the technical documentation (IR definitions, layers, pipelines).

The review identifies:

* Where implementation matches design.
* Where it diverges.
* Whether divergence is accidental or intentional.

The outcome is recorded and used to guide refactor-type cycles and updates to the architecture docs. Refactor cycles should typically be directed at reducing unintentional drift and at restoring or improving clear boundaries in the system.

---

## 8. Retrospectives and prompt evolution

The process itself is subject to improvement. Each cycle and each set of cycles feed information back into how agents are instructed to behave.

### 8.1 Cycle retrospectives

After one or several cycles, an agent runs a retrospective over:

* Decision records.
* Mini-specs.
* Implementer logs.
* Reviewer reports.
* Test and scenario results.

The retrospective answers questions such as:

* What went well in recent cycles?
* What recurring problems are visible (for example, missing tests, repeated static analysis issues, confusion about IR boundaries)?
* Are planner decisions consistently aligned with the risk ledger and milestones, or is work drifting toward easier but less valuable tasks?
* Which parts of the documentation or test plan are causing confusion?

The outcome is documented under `retrospectives/` with a date. The human maintainer can add comments or decisions in response.

### 8.2 Prompt sets as versioned artifacts

The behavior of the planner, implementer, and reviewer agents is largely determined by their prompts. These prompts are stored in the repository under `prompts/` (for example, `prompts/planner.md`, `prompts/implementer.md`, `prompts/reviewer.md`).

Retrospectives may identify changes that should be made to these prompts:

* Additional constraints for planners (for example, always reference specific testing milestones when proposing work).
* More explicit instructions for implementers (for example, always update scenario manifests when adding new fixtures).
* Additional checks for reviewers (for example, perf regressions on known heavy fixtures).

Proposed prompt changes are treated like any other change to the codebase:

* They are written as diffs against the existing prompt files.
* They are reviewed and approved by the human maintainer.
* They are referenced in decision records so that behavior changes can be correlated with process changes.

By versioning prompts and evolving them deliberately, the project ensures that agent behavior adapts over time without losing track of why it changed.

---

## 9. Summary

This guide defines a development process where AI agents and humans work together under explicit structure:

* The planner ties each cycle to a clear choice, a mini-spec, and specific tests.
* The implementer works within that spec and is constrained by static checks and review.
* Evaluation includes not only correctness but performance and scenario-level product behavior.
* The post-implementation verification reviewer independently validates that the implementation matches the plan, catches hidden bugs, and confirms all specified tests were created.
* Architecture and risk are managed at a higher level of abstraction, with their own documents and regular reviews.
* The process itself evolves through retrospectives and versioned prompts.

A cycle checklist template is provided in `docs/meta/logs/cycle_checklist_template.md` to help track progress through each phase of the development cycle.

Any new agent or contributor should read this document before making changes, and should treat the described artifacts and flows as the canonical way to move the system forward.

---

Last updated: 2025-11-26
