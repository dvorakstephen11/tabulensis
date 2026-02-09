# Model Accounts / Capacity Planning

This file tracks which model accounts exist, what each is used for, and when to
switch (to support long overnight runs without thrashing).

## Accounts

| Account | Primary use | Notes (limits / gotchas) |
| --- | --- | --- |
| (fill) | (fill) | (fill) |

## Switching Policy

- Prefer running long background work via the overnight agent in `codex_exec` mode when feasible.
- If ChatGPT Pro usage limits block deep research or planning:
  - record the date and the blocking condition
  - record the mitigation (second account, schedule shift, smaller prompts, etc.)

## Decisions

- Second OpenAI Pro account:
  - Status: chosen (2026-02-09)
  - Decision: defer until limits materially block progress (see `docs/meta/results/decision_register.md` `DR-0015`).
  - Trigger to revisit: the first time an overnight run or deep research session is blocked by usage limits.
