#!/usr/bin/env python3
"""
Deterministic mutators for OpenXML workbooks (.xlsx/.xlsm/.xltx/.xltm).

Design goals:
- Generate a "B" workbook from an "A" workbook with controlled, deterministic edits.
- Avoid rewriting unrelated parts; patch a single worksheet part when possible.
- Be safe on untrusted inputs (zip-level operations only; no macro execution).
"""

from __future__ import annotations

import argparse
import random
import re
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


NUMERIC_RE = re.compile(r"^[+-]?(?:\d+\.?\d*|\d*\.?\d+)(?:[eE][+-]?\d+)?$")
ROW_R_RE = re.compile(rb'\br="(\d+)"')


def pick_default_worksheet_name(names: list[str]) -> str:
    if "xl/worksheets/sheet1.xml" in names:
        return "xl/worksheets/sheet1.xml"
    worksheets = [n for n in names if n.startswith("xl/worksheets/") and n.endswith(".xml")]
    if not worksheets:
        raise ValueError("No worksheet parts found under xl/worksheets/")
    return sorted(worksheets)[0]


def compute_new_numeric_value(old: str, *, seed: int, idx: int) -> str:
    # Keep changes small but deterministic.
    delta = float((seed % 7) + 1 + idx)
    try:
        val = float(old)
    except Exception:
        return old
    new_val = val + delta
    # Preserve integer formatting when possible.
    if "." not in old and "e" not in old.lower():
        try:
            return str(int(round(new_val)))
        except Exception:
            pass
    # Avoid scientific notation for small values.
    return f"{new_val:.6f}".rstrip("0").rstrip(".")


def mutate_cell_edit_numeric(xml: bytes, *, edits: int, seed: int) -> tuple[bytes, int]:
    """
    Patch the first N non-sharedString numeric cell <v> values inside <c> elements.
    """
    out = bytearray()
    i = 0
    edited = 0
    n = len(xml)

    while i < n:
        if edited >= edits:
            out.extend(xml[i:])
            break

        cell_start = xml.find(b"<c", i)
        if cell_start == -1:
            out.extend(xml[i:])
            break

        out.extend(xml[i:cell_start])

        tag_end = xml.find(b">", cell_start)
        if tag_end == -1:
            out.extend(xml[cell_start:])
            break

        start_tag = xml[cell_start : tag_end + 1]

        # Self-closing cell.
        if start_tag.rstrip().endswith(b"/>"):
            out.extend(start_tag)
            i = tag_end + 1
            continue

        # Skip string-like cell types (shared strings / inline strings / cached string).
        if b't="s"' in start_tag or b't="inlineStr"' in start_tag or b't="str"' in start_tag:
            end = xml.find(b"</c>", tag_end + 1)
            if end == -1:
                out.extend(xml[cell_start:])
                break
            end += len(b"</c>")
            out.extend(xml[cell_start:end])
            i = end
            continue

        end = xml.find(b"</c>", tag_end + 1)
        if end == -1:
            out.extend(xml[cell_start:])
            break
        end += len(b"</c>")

        cell = xml[cell_start:end]
        v0 = cell.find(b"<v>")
        if v0 == -1:
            out.extend(cell)
            i = end
            continue
        v1 = cell.find(b"</v>", v0 + 3)
        if v1 == -1:
            out.extend(cell)
            i = end
            continue

        old_bytes = cell[v0 + 3 : v1]
        old = old_bytes.decode("utf-8", errors="ignore").strip()
        if not old or not NUMERIC_RE.match(old):
            out.extend(cell)
            i = end
            continue

        new = compute_new_numeric_value(old, seed=seed, idx=edited)
        new_cell = cell[: v0 + 3] + new.encode("utf-8") + cell[v1:]
        out.extend(new_cell)
        edited += 1
        i = end

    return bytes(out), edited


@dataclass
class RowSpan:
    start: int
    end: int
    r: int


def extract_row_spans(xml: bytes) -> list[RowSpan]:
    spans: list[RowSpan] = []
    i = 0
    n = len(xml)
    while i < n:
        row_start = xml.find(b"<row", i)
        if row_start == -1:
            break
        tag_end = xml.find(b">", row_start)
        if tag_end == -1:
            break
        start_tag = xml[row_start : tag_end + 1]
        m = ROW_R_RE.search(start_tag)
        if not m:
            i = tag_end + 1
            continue
        r = int(m.group(1))

        if start_tag.rstrip().endswith(b"/>"):
            row_end = tag_end + 1
        else:
            close = xml.find(b"</row>", tag_end + 1)
            if close == -1:
                break
            row_end = close + len(b"</row>")

        spans.append(RowSpan(start=row_start, end=row_end, r=r))
        i = row_end
    return spans


def rewrite_row_numbers(row_bytes: bytes, *, new_r: int) -> bytes:
    # Only rewrite within this row element.
    tag_end = row_bytes.find(b">")
    if tag_end == -1:
        return row_bytes
    start_tag = row_bytes[: tag_end + 1]
    rest = row_bytes[tag_end + 1 :]

    m = ROW_R_RE.search(start_tag)
    if not m:
        return row_bytes
    old_r = int(m.group(1))

    start_tag2 = ROW_R_RE.sub(lambda _m: f'r="{new_r}"'.encode("utf-8"), start_tag, count=1)

    # Update cell references inside <c ... r="A{old_r}">
    cell_rx = re.compile(
        rb'(<c\b[^>]*\br=")([A-Za-z]{1,3})' + str(old_r).encode("utf-8") + rb'(")'
    )
    rest2 = cell_rx.sub(lambda m: m.group(1) + m.group(2) + str(new_r).encode("utf-8") + m.group(3), rest)

    return start_tag2 + rest2


def mutate_row_block_swap_by_index(xml: bytes, *, row_count: int, seed: int) -> tuple[bytes, dict[str, Any]]:
    spans = extract_row_spans(xml)
    if len(spans) < row_count * 2:
        raise ValueError(f"Not enough <row> elements to swap: have {len(spans)}, need {row_count*2}")

    rng = random.Random(seed)
    max_start = len(spans) - row_count

    a_start = rng.randrange(0, max_start + 1)
    b_start = rng.randrange(0, max_start + 1)
    # Ensure non-overlap.
    attempts = 0
    while not (b_start + row_count <= a_start or a_start + row_count <= b_start):
        b_start = rng.randrange(0, max_start + 1)
        attempts += 1
        if attempts > 1000:
            raise ValueError("Failed to choose non-overlapping row blocks; reduce row_count")

    if a_start > b_start:
        a_start, b_start = b_start, a_start

    block_a = spans[a_start : a_start + row_count]
    block_b = spans[b_start : b_start + row_count]

    a_bytes_list: list[bytes] = []
    b_bytes_list: list[bytes] = []

    for i in range(row_count):
        ra = block_a[i]
        rb = block_b[i]
        row_a_bytes = xml[ra.start : ra.end]
        row_b_bytes = xml[rb.start : rb.end]
        a_bytes_list.append(rewrite_row_numbers(row_b_bytes, new_r=ra.r))
        b_bytes_list.append(rewrite_row_numbers(row_a_bytes, new_r=rb.r))

    new_block_a = b"".join(a_bytes_list)
    new_block_b = b"".join(b_bytes_list)

    out = bytearray()
    out.extend(xml[: block_a[0].start])
    out.extend(new_block_a)
    out.extend(xml[block_a[-1].end : block_b[0].start])
    out.extend(new_block_b)
    out.extend(xml[block_b[-1].end :])

    meta = {
        "row_count": row_count,
        "seed": seed,
        "a_start_index": a_start,
        "b_start_index": b_start,
        "a_row_r_start": block_a[0].r,
        "b_row_r_start": block_b[0].r,
    }
    return bytes(out), meta


def rewrite_openxml_package(
    input_path: Path,
    output_path: Path,
    *,
    mutate_part: str,
    mutate_fn,
) -> dict[str, Any]:
    with zipfile.ZipFile(input_path, "r") as zin:
        names = zin.namelist()
        if mutate_part not in names:
            raise ValueError(f"Worksheet part not found: {mutate_part}")

        # Compute mutated bytes.
        original = zin.read(mutate_part)
        mutated, meta = mutate_fn(original)

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(output_path, "w") as zout:
            for info in zin.infolist():
                data = zin.read(info.filename)
                if info.filename == mutate_part:
                    data = mutated
                # Preserve metadata best-effort.
                out_info = zipfile.ZipInfo(info.filename, date_time=info.date_time)
                out_info.compress_type = zipfile.ZIP_DEFLATED
                out_info.external_attr = info.external_attr
                zout.writestr(out_info, data)

    return {"mutated_part": mutate_part, **(meta or {})}


def main() -> int:
    parser = argparse.ArgumentParser(description="Deterministically mutate an OpenXML workbook.")
    parser.add_argument("--input", type=Path, required=True, help="Input .xlsx/.xlsm/.xltx/.xltm")
    parser.add_argument("--output", type=Path, required=True, help="Output workbook path")
    parser.add_argument(
        "--mode",
        type=str,
        required=True,
        choices=["cell_edit_numeric", "row_block_swap"],
        help="Mutation mode",
    )
    parser.add_argument(
        "--worksheet-part",
        type=str,
        default="",
        help='Worksheet part path inside zip (default: pick sheet1.xml or first worksheet), e.g. "xl/worksheets/sheet1.xml"',
    )
    parser.add_argument("--seed", type=int, default=0, help="Deterministic seed")
    parser.add_argument("--edits", type=int, default=10, help="Number of numeric cell edits (cell_edit_numeric)")
    parser.add_argument("--row-count", type=int, default=200, help="Row count for row_block_swap")
    args = parser.parse_args()

    if args.input.suffix.lower() not in {".xlsx", ".xlsm", ".xltx", ".xltm"}:
        raise SystemExit("ERROR: mutate_openxml_xlsx.py supports only .xlsx/.xlsm/.xltx/.xltm inputs")

    with zipfile.ZipFile(args.input, "r") as zin:
        part = args.worksheet_part.strip() or pick_default_worksheet_name(zin.namelist())

    mode = args.mode
    seed = int(args.seed)

    if mode == "cell_edit_numeric":
        edits = int(args.edits)

        def mutate_fn(xml: bytes):
            mutated, edited = mutate_cell_edit_numeric(xml, edits=edits, seed=seed)
            return mutated, {"mode": mode, "seed": seed, "edits_requested": edits, "edits_applied": edited}

    elif mode == "row_block_swap":
        row_count = int(args.row_count)

        def mutate_fn(xml: bytes):
            mutated, meta = mutate_row_block_swap_by_index(xml, row_count=row_count, seed=seed)
            meta = dict(meta)
            meta["mode"] = mode
            return mutated, meta

    else:
        raise SystemExit(f"Unsupported mode: {mode}")

    meta = rewrite_openxml_package(args.input, args.output, mutate_part=part, mutate_fn=mutate_fn)
    print(f"Wrote: {args.output}")
    print("Meta:", meta)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
