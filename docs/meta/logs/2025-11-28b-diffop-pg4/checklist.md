# Cycle Checklist: 2025-11-28b-diffop-pg4

Copy this template into your branch-specific logs directory (e.g., `docs/meta/logs/feature-name/checklist.md`) and update as you progress through the cycle.

---

## Pre-Implementation

- [ ] Decision record created (`plans/2025-11-28b-diffop-pg4.yml`)
- [ ] Mini-spec written (`plans/2025-11-28b-diffop-pg4.md`) with scope, behavioral contract, and constraints
- [ ] Test plan defined with specific tests and fixtures
- [ ] Initiate Cursor implementation agent with prompt (`prompts/implementer.md`

## Implementation

- [ ] Activity log started (`activity_log.txt`)
- [ ] All scope items from mini-spec addressed:
  - [ ] Item 1: ____________________
  - [ ] Item 2: ____________________
  - [ ] Item 3: ____________________
- [ ] All specified tests created:
  - [ ] Test 1: ____________________
  - [ ] Test 2: ____________________
  - [ ] Test 3: ____________________
- [ ] Code compiles without errors
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes (or warnings documented)
- [ ] Activity log completed with summary of changes

## Initial Review

- [ ] Automated review completed
- [ ] No invariant violations identified
- [ ] No unintended complexity growth
- [ ] Test coverage adequate for scope

## Evaluation

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Performance tests pass (if applicable)
- [ ] Scenario tests pass (if applicable)
- [ ] Test results recorded (`results/[branch]/test_YYYY-MM-DD.yml`)

## Post-Implementation Verification

- [ ] Run `python docs/meta/prompts/generate_review_context.py --collate` to collate review files
- [ ] Start new chat, paste instruction from clipboard, attach collated files from `~/Downloads/2025-11-28b-diffop-pg4/`
- [ ] Verification review completed
- [ ] Verification report saved (`verification_report.md`)
- [ ] Recommendation: ____________________
  - [ ] Proceed to release
  - [ ] Remediation required

### If Remediation Required

- [ ] Remediation plan created
- [ ] Remediation implemented
- [ ] Tests re-run and passing
- [ ] Follow-up verification completed (if significant changes)

## Release

- [ ] Merged to main branch
- [ ] Version tagged
- [ ] Release notes updated (if applicable)

---

## Notes

[Add any cycle-specific notes, blockers, or follow-up items here]

---

## Sign-Off

- **Cycle Started**: YYYY-MM-DD
- **Cycle Completed**: YYYY-MM-DD
- **Final Status**: ____________________

