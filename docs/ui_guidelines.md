# Tabulensis Desktop UI Guidelines

**Last updated:** 2026-02-05

These guidelines keep the desktop UI consistent and readable across changes. Treat them as non-negotiable unless explicitly approved.

## Typography
- Default font: platform UI font (no custom font overrides unless justified).
- Titles: 12–14 pt, semibold, single line.
- Body: 10–12 pt, regular weight.
- Avoid text truncation; if unavoidable, prefer ellipsis and tooltip.

## Spacing Scale
Use a consistent spacing scale:
- `4`, `8`, `12`, `16`, `24` px only.
- Avoid one-off spacing values.

## Layout & Alignment
- Left-align labels and text fields.
- Primary actions align right in their group.
- Maintain consistent column widths across list views.
- Keep side panels at or above minimum width (e.g., sheets list at 260px).

## Controls & States
- Buttons: consistent height; primary action clearly labeled.
- Checkboxes: align with related controls; avoid orphaned toggles.
- Progress: use visible text state and not just a gauge.
- Error states: show error code + human-readable message.

## Visual Hierarchy
- Use grouping: panels or sizers must create clear visual blocks.
- Do not mix unrelated controls in the same horizontal row.
- Maintain a strong “top-to-bottom” flow for the compare workflow.

## Colors & Contrast
- Prefer system colors for text and backgrounds.
- Ensure error/warning states are readable without relying on color alone.

## Summary/Details Tabs
- Summary must remain scannable at a glance.
- Details should never overflow the viewport without scroll.

## Consistency
- Any new UI element must follow existing naming and layout conventions in `desktop/wx/ui/main.xrc`.
- If you introduce a new alignment pattern, update this document and the visual baselines.
