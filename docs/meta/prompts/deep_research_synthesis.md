<!--
Paste this into ChatGPT.
Shortcut: `python3 scripts/deep_research_prompt.py --prompt synthesis` (copies to clipboard).
-->

Today is {{RUN_DATE}}.

You are helping operate Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

You will be given two separate deep research outputs (A and B) covering overlapping areas. Your job is to synthesize them into a single, high-ROI plan for what the operator should do *today*.

Constraints:
- Assume a tiny team (often 1 operator). Prefer actions that can be completed end-to-end quickly.
- Deduplicate: merge overlapping items; prefer the clearest/most actionable formulation.
- Be concrete: every checklist item should be an action with a clear deliverable.
- If a claim is uncertain or needs verification, convert it into a short verification step (with the fastest way to confirm).

## Inputs

### Output A

<PASTE OUTPUT A HERE>

### Output B

<PASTE OUTPUT B HERE>

## Required Output (Markdown)

Produce exactly this structure:

- `## Today’s Checklist`
  - `### 30 minutes` (3-7 actions)
  - `### 60 minutes` (2-5 actions)
  - `### 120 minutes` (1-3 actions)

Rules for the checklist:
- Use `- [ ]` checkbox items for every action.
- Each item must be phrased as "verb + object" and end with a concrete deliverable (file created, PR opened, email sent, metric captured, decision recorded, etc.).
- Prefer sequencing: put the highest leverage / highest dependency actions first in each time bucket.
- If something is important but too big, convert it into a 30- or 60-minute "first slice" action (scoping, spike, or writing a checklist).

Optional (only if it helps clarity, keep it short):
- `## Notes` (up to 10 bullets): why these actions are prioritized and what to ignore/defer.

## Post-Synthesis Actions (Operator Checklist)

These are for the human operator after you paste the synthesized checklist into `docs/meta/today.md`. Do not claim you performed these actions.

- [ ] Update `docs/meta/today.md` with the synthesized "Today’s Checklist".
- [ ] Append 5-10 bullets to today’s daily log: `docs/meta/logs/daily/YYYY-MM-DD.md`.
- [ ] Create 1 marketing experiment task (small, measurable, <= 2 hours to run).
- [ ] Create 1 product friction task (a concrete UX/paper-cut fix).
- [ ] Create 1 perf/UI/security task (pick the highest risk or biggest win from the research).

