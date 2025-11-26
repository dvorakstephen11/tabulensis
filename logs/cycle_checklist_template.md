# Cycle Checklist: [Branch Name]

Copy this template into your branch-specific logs directory (e.g., `logs/feature-name/checklist.md`) and update as you progress through the cycle.

---

## Pre-Implementation

- [ ] Decision record created (`plans/YYYY-MM-DD-cycle.yml`)
- [ ] Mini-spec written with scope, behavioral contract, and constraints
- [ ] Test plan defined with specific tests and fixtures
- [ ] `prompt_original.md` saved to logs directory
- [ ] `plan_response.md` saved to logs directory

## Implementation

- [ ] Activity log started (`activity_log.txt`)
- [ ] All scope items from mini-spec addressed
- [ ] All specified tests created
- [ ] Code compiles without errors
- [ ] `cargo fmt` passes without warnings
- [ ] `cargo clippy` passes without warnings
- [ ] Activity log completed with summary of changes

## Initial Review
## Evaluation

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Performance tests pass (if applicable)
- [ ] Scenario tests pass (if applicable)
- [ ] Test results recorded (`results/[branch]/test_YYYY-MM-DD.yml`)

## Post-Implementation Verification
- [ ] Verification review prompted
- [ ] Verification report saved (`verification_report.md`)

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

