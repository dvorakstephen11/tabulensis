### What the benchmark data says is worth optimizing

Your **e2e suite is overwhelmingly parse-bound**: across all 5 e2e tests, `total_time_ms=24129` while `diff_time_ms` is only the remainder after parse, and the per-test breakdown shows parse dominating in every large case. 

The clearest example is **`e2e_p5_identical`**: `total_time_ms=15690`, `parse_time_ms=15679`, and `diff_time_ms=11` — so ~99.9% of wall time is parsing/opening, not diffing. It also peaks at ~762MB and builds a ~480MB string pool. 

Even the smaller “dense” case shows the same pattern: **`e2e_p1_dense`** is `parse_time_ms=2747` vs `diff_time_ms=8`. 

So: if the goal is “better benchmark numbers”, the highest-leverage work is **making worksheet parsing cheaper** (fewer allocations, less copying), especially for sheets with millions of string cells.

(Separately, your in-memory perf suite shows signature-building is the dominant phase in some cases — e.g. `perf_50k_alignment_block_move` spends `448ms` of `504ms` in `signature_build_time_ms`.  But that won’t move the e2e totals much until parsing is fixed.)

---

### The hot spot in the code: per-cell allocations in the worksheet XML parser

In `parse_sheet_xml`, every `<c>` (cell) calls `parse_cell(...)` and pushes a `ParsedCell` into a vector. 

Inside `parse_cell`, the current implementation allocates and copies **multiple heap strings per cell**:

* `get_attr_value(...).into_owned()` returns a new `String` for the cell address (`r="A1"`) and often also for `t="s"` (string type).
* `<v>` content is read via `read_text(...).into_owned()` into `value_text: Option<String>`, even though you immediately parse it into a number or shared-string index later. 
* `parse_cell` also creates a fresh `Vec` buffer (`let mut buf = Vec::new();`) per cell, which prevents reusing the XML scratch buffer capacity across millions of cells. 

For the largest fixture (`e2e_p5_identical`), you’re doing this on the order of **millions of cells** (`cells_compared=5000100`, and that’s just one test’s comparison surface) — so even “tiny” per-cell overhead becomes seconds. 

---

### A concrete, high-impact improvement

**Make `parse_cell` essentially allocation-free for the common cases** (shared string cells and numeric cells) by:

1. Changing `get_attr_value` to return a borrowed `Cow<str>` instead of always allocating a `String`.
2. Reusing one `Vec<u8>` scratch buffer for *all* cells in a worksheet (create it once in `parse_sheet_xml` and pass it into `parse_cell`).
3. Parsing `<v>` directly into `CellValue` immediately (using the `Cow<str>` from `read_text`), instead of storing `value_text: Option<String>`.

This targets exactly what your benchmarks are screaming about: e2e time is parse time. 

---

## Code to replace

```rust
pub fn parse_sheet_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Grid, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&reader, xml, &e, b"ref")? {
                    dimension_hint = dimension_from_ref(&r);
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, xml, e, shared_strings, pool)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                parsed_cells.push(cell);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
    }

    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r + 1);
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c + 1);
    }

    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<ParsedCell, GridParseError> {
    let address_raw =
        get_attr_value(reader, xml, &start, b"r")?.ok_or_else(|| {
            xml_msg_err(reader, xml, "cell missing address")
        })?;
    let (row, col) = address_to_index(&address_raw)
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.clone()))?;

    let cell_type = get_attr_value(reader, xml, &start, b"t")?;

    let mut value_text: Option<String> = None;
    let mut formula_text: Option<String> = None;
    let mut inline_text: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?
                    .into_owned();
                value_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?
                    .into_owned();
                let unescaped = quick_xml::escape::unescape(&text)
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?
                    .into_owned();
                formula_text = Some(unescaped);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                inline_text = Some(read_inline_string(reader, xml)?);
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(reader, xml, "unexpected EOF inside cell"));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    let value = match inline_text {
        Some(text) => Some(CellValue::Text(pool.intern(&text))),
        None => convert_value(
            value_text.as_deref(),
            cell_type.as_deref(),
            shared_strings,
            pool,
            reader,
            xml,
        )?,
    };

    Ok(ParsedCell {
        row,
        col,
        value,
        formula: formula_text.map(|f| pool.intern(&f)),
    })
}

fn get_attr_value(
    reader: &Reader<&[u8]>,
    xml: &[u8],
    element: &BytesStart<'_>,
    key: &[u8],
) -> Result<Option<String>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        if attr.key.as_ref() == key {
            return Ok(Some(
                attr.unescape_value()
                    .map_err(|e| xml_err(reader, xml, e))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}
```

## New code to replace it with

```rust
pub fn parse_sheet_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Grid, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut cell_buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&reader, xml, &e, b"ref")? {
                    dimension_hint = dimension_from_ref(r.as_ref());
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, xml, e, shared_strings, pool, &mut cell_buf)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                parsed_cells.push(cell);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
    }

    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r + 1);
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c + 1);
    }

    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    buf: &mut Vec<u8>,
) -> Result<ParsedCell, GridParseError> {
    let address_raw = get_attr_value(reader, xml, &start, b"r")?
        .ok_or_else(|| xml_msg_err(reader, xml, "cell missing address"))?;
    let (row, col) = address_to_index(address_raw.as_ref())
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.into_owned()))?;

    let cell_type = get_attr_value(reader, xml, &start, b"t")?;

    let mut value: Option<CellValue> = None;
    let mut formula: Option<StringId> = None;

    buf.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                value = convert_value(
                    Some(text.as_ref()),
                    cell_type.as_deref(),
                    shared_strings,
                    pool,
                    reader,
                    xml,
                )?;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                let unescaped = quick_xml::escape::unescape(text.as_ref())
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                formula = Some(pool.intern(unescaped.as_ref()));
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                let inline = read_inline_string(reader, xml)?;
                value = Some(CellValue::Text(pool.intern(&inline)));
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(reader, xml, "unexpected EOF inside cell"));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedCell {
        row,
        col,
        value,
        formula,
    })
}

fn get_attr_value<'a>(
    reader: &Reader<&[u8]>,
    xml: &[u8],
    element: &BytesStart<'a>,
    key: &[u8],
) -> Result<Option<std::borrow::Cow<'a, str>>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        if attr.key.as_ref() == key {
            let v = attr
                .unescape_value()
                .map_err(|e| xml_err(reader, xml, e))?;
            return Ok(Some(v));
        }
    }
    Ok(None)
}
```

---

### Why this should move your benchmark numbers

Visually: imagine the parser as an assembly line that processes one cell at a time. Right now, each cell causes several “go to the warehouse and get a fresh box” actions (heap allocations and copies). On a sheet with **millions of cells**, those trips dominate.

This change keeps the same logic but makes the common path:

* borrow the address/type text instead of allocating,
* reuse the same scratch buffer for every cell,
* parse `<v>` straight into the final enum without storing intermediate owned `String`s.

That targets the exact wall-time sink exposed by the e2e metrics (`parse_time_ms` dominating `total_time_ms`, especially for `e2e_p5_identical`). 
