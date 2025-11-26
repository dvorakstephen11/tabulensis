Here’s a blueprint you can actually build from.

I’ll walk from outer container down to bytes-on-the-wire, then show how binwalk and Kaitai fit into the workflow and what a production-quality parser should look like.

---

## 0. Scope: what “DataMashup parser” actually means

For Excel/Power BI, “DataMashup” is:

* For **Excel**: base64-encoded inside a `customXml/item*.xml` `<DataMashup>` element (namespace `http://schemas.microsoft.com/DataMashup`).
* For **older Power BI (PBIX/PBIT)**: a file named `DataMashup` in the outer OPC/ZIP package.([bengribaudo.com][1])

Once you decode/unzip to the raw **Data Mashup Binary Stream**, the format is **fully specified by MS‑QDEFF** (Query Definition File Format).([bengribaudo.com][1])

Your parser’s job:

1. Find and decode the DataMashup stream from host file (Excel/PBIX/etc).
2. Parse the **top-level binary stream** (version, 4 length-prefixed sections).
3. Interpret:

   * Package Parts → OPC/ZIP with `Formulas/Section1.m`, `/Config/Package.xml`, `/Content/*`…
   * Permissions XML
   * Metadata XML (+ Metadata Content OPC/ZIP)
   * Permission Bindings (DPAPI-protected hash blob)

And then expose a clean API: “here are the queries, their M code, metadata, embedded contents, permissions”.

Because MS‑QDEFF is public, this is more “engineering + validation” than “pure black-box RE”, but you still want RE tooling (binwalk, Kaitai, DataMashupExplorer) to handle weird/legacy/non-compliant files.

---

## 1. Outer container → DataMashup bytes

### 1.1 Excel (.xlsx/.xlsm/.xlsb)

1. Treat the workbook as an **OPC / Open XML package** (ZIP with `[Content_Types].xml`).([bengribaudo.com][1])
2. Iterate `/customXml/item*.xml` parts:

   * Look for a document whose root element is `DataMashup` in namespace `http://schemas.microsoft.com/DataMashup`. ([bengribaudo.com][1])
3. The `<DataMashup>` element’s text content is **base64**; decode it → this is your **top-level binary stream**.

Edge cases / invariants:

* There should be exactly one `DataMashup` part if the workbook uses Power Query.([bengribaudo.com][1])
* The `sqmid` attribute (optional GUID) is telemetry only; ignore for semantics.

### 1.2 Older PBIX/PBIT

1. Treat `.pbix` / `.pbit` as OPC/ZIP.
2. Open the `DataMashup` file at the root of the package. No base64 wrapper; this *is* the top-level binary stream.([bengribaudo.com][1])

Caveat: newer PBIX with **enhanced dataset metadata** may no longer store a DataMashup file; Power BI regenerates mashups from the tabular model and the M code lives elsewhere (DMVs etc.).([Power BI Community][2])

Your parser should therefore:

* Detect absence of `DataMashup` and clearly report “new-style PBIX without DataMashup; use tabular model path instead.”

---

## 2. Top-level DataMashup binary layout (MS‑QDEFF)

MS‑QDEFF explicitly defines the binary stream as:

* Root: `DataMashup` XML element containing base64 of a **top-level binary stream** (for Excel), or the bare binary stream in `DataMashup` file (PBIX).

### 2.1 Canonical layout

From MS‑QDEFF §2.2: the top-level stream is:

```text
offset  size  field
0       4     Version                     (uint32 LE, MUST be 0 currently)
4       4     PackagePartsLength          (uint32 LE)
8       N     PackageParts                (N bytes)
...     4     PermissionsLength           (uint32 LE)
...     M     Permissions                 (M bytes)
...     4     MetadataLength              (uint32 LE)
...     K     Metadata                    (K bytes)
...     4     PermissionBindingsLength    (uint32 LE)
...     P     PermissionBindings          (P bytes)
```

Each `*Length` is a 4‑byte unsigned **little-endian** integer.

Invariants you should enforce:

* `Version == 0` (for now). Treat any other value as either “future version” (warn but attempt) or hard error, depending on your tolerance.
* Total stream length must be **at least** 4 + 4 + 4 + 4 + 4 (header + four zero-length sections).
* Sum of lengths must not exceed the buffer length:

  ```text
  4 + (4+N) + (4+M) + (4+K) + (4+P) == total_bytes
  ```

  or, at minimum, `running_offset <= total_bytes` at each step.

This layout is simple enough that a Kaitai spec is trivial (more on that later).

---

## 3. Section-by-section semantics

### 3.1 Package Parts (embedded OPC / ZIP)

MS‑QDEFF: `Package Parts` is itself an **OPC package** with at least these parts:

| Part path              | Purpose                                                |
| ---------------------- | ------------------------------------------------------ |
| `/Config/Package.xml`  | Client version, minimum reader version, culture, etc.  |
| `/Formulas/Section1.m` | The Power Query M code (section document).             |
| `/Content/{GUID}`      | 0+ embedded content items, each itself an OPC package. |

These inner OPC packages begin with `PK\x03\x04` signatures; binwalk sees them as embedded ZIPs.([The Biccountant][3])

Practical parsing strategy:

1. Treat `PackageParts` bytes as a ZIP/OPC stream.
2. Use a normal ZIP/OPC library to list entries and extract required parts.
3. Read `/Config/Package.xml` as UTF‑8 XML; parse fields:

   * Client version, minimal compatible version, culture, etc. (helps with diagnostics).
4. Read `/Formulas/Section1.m` as UTF‑8 text:

   * This is a Power Query “section document”; Excel/Power BI currently enforce a single section called `Section1` with all members shared if they’re loadable.([bengribaudo.com][1])
5. For each `/Content/{GUID}`:

   * Treat as another OPC/ZIP; inside you’ll find its own `/Formulas/Section1.m` and `/Config/Formulas.xml`. These are the “embedded contents” used by `Embedded.Value`.([bengribaudo.com][1])

This is exactly what Imke’s M code is doing: decode → unzip → select `"Formulas/Section1.m"`.([The Biccountant][4])

### 3.2 Permissions (XML)

Permissions is a small UTF‑8 XML document storing 3 main values:([bengribaudo.com][1])

* `CanEvaluateFuturePackages` (always false, effectively ignored)
* `FirewallEnabled` (privacy/firewall on/off)
* `WorkbookGroupType` (privacy level when queries read from the workbook)

You mostly just want to surface these as flags. Excel & Power BI override them if Permission Bindings check fails.

### 3.3 Metadata (XML + optional OPC ZIP)

MS‑QDEFF splits this into:

* **Metadata XML**: `LocalPackageMetadataFile`, with:

  * `AllFormulas` section (query groups, relationships-to-data-model, etc.).
  * `Formulas` entries (one per query), keyed as `SectionName/FormulaName` (URL-encoded).
  * Lots of properties: load destination, result type, last refresh columns, etc.
  * Some values are base64 or custom binary encodings.
* **Metadata Content (OPC)**: rarely used legacy binary stream; can often be ignored safely.

Ben’s tools (Data Mashup Explorer + Cmdlets) translate this verbose mix of XML, JSON-ish content and binary fields into a neat JSON view—that’s your reference oracle for “what the metadata really means in practice”.([bengribaudo.com][1])

Your parser should:

1. Treat `Metadata` section as:

   * A small header + XML + possibly an embedded OPC stream (see MS‑QDEFF’s §2.5.2 for exact layout).
   * For normal Excel/Power BI, you can just parse the entire `Metadata` bytes as UTF‑8 XML; the XML is the “Metadata XML binary stream” described in a separate page.([bengribaudo.com][5])
2. Map the known attributes (IsPrivate, LoadToDataModel, etc.) into a strongly typed struct.
3. Preserve unknown attributes as a generic bag for forward compatibility.

### 3.4 Permission Bindings (cryptographic checksum)

This is a blob used to protect `Permissions` from tampering. On save, Excel/Power BI compute SHA‑256 hashes of Package Parts + Permissions, combine them, encrypt with DPAPI (Windows, user-scoped), and store here. On load, if decrypt+verify fails, they ignore `Permissions` and revert to defaults.([bengribaudo.com][1])

For a **cross-platform parser** that only *reads* M code:

* You can treat Permission Bindings as **opaque bytes**, and:

  * Optionally expose “bindings_present: bool”.
  * Don’t attempt to verify them. Even Data Mashup Cmdlets currently assume bindings are valid.([bengribaudo.com][6])
* If you’re on Windows and want to fully emulate Excel’s behavior, you can use DPAPI (`CryptUnprotectData`) with the current user context and re-hash to validate.

---

## 4. Using binwalk & Kaitai Struct effectively

### 4.1 Binwalk: recon and sanity checks

While you *know* from the spec where the OPC package is (Package Parts), binwalk is still useful:

* **Recon on unknown sections or future versions**:

  * Run binwalk on the raw DataMashup bytes, look for additional embedded ZIPs, zlib streams, etc. This can highlight implementation quirks or vendor extensions.([GitHub][7])
* **Validation**:

  * Confirm that the Package Parts slice starting at `offset = 8` really contains a ZIP signature (`PK\x03\x04`) near the start.
  * Quickly eyeball corrupted or partially-truncated DataMashup streams.

For automation, you can:

* Call binwalk as a subprocess in a test harness to check your own slicing.
* Or embed a **Rust binwalk-like crate** (e.g. `binwalk` on crates.io) to scan for ZIP signatures and validate `PackagePartsLength`.([Crates.io][8])

### 4.2 Kaitai Struct: formalizing the binary layout

Kaitai is perfect for expressing the **top-level stream** and delegating sub-parsers (ZIP, XML) to other code.([doc.kaitai.io][9])

You’d define a `datamashup.ksy` something like (conceptual, not verbatim):

```yaml
meta:
  id: datamashup
  endian: le

seq:
  - id: version
    type: u4
  - id: package_parts_len
    type: u4
  - id: package_parts
    size: package_parts_len
  - id: permissions_len
    type: u4
  - id: permissions
    size: permissions_len
  - id: metadata_len
    type: u4
  - id: metadata
    size: metadata_len
  - id: permission_bindings_len
    type: u4
  - id: permission_bindings
    size: permission_bindings_len

instances:
  is_supported_version:
    value: version == 0
```

Then:

* Use **Kaitai Web IDE** (`ide.kaitai.io`) to load:

  * Input: a sample DataMashup binary.
  * Spec: your `datamashup.ksy`.
* Inspect parsed fields & offsets, ensuring lengths line up correctly.
* Once happy, compile to your target language (Rust, C#, etc.) and wrap with:

  * “Decode base64 → feed into Kaitai parser → get slices → pass slices to ZIP/XML/DPAPI libraries”.

You can iteratively expand the KSY with:

* Nested specs for Permissions XML (if you want to treat it as opaque bytes, ignore this).
* A spec for “Metadata header + XML size” — but XML itself is much easier to parse with an XML library.

---

## 5. Parser architecture blueprint

### 5.1 Layered design

Think in layers:

1. **Host container layer (Excel/PBIX)**
   Responsibility: locate DataMashup stream and decode base64.

2. **Binary framing layer (MS‑QDEFF top-level)**
   Responsibility: parse `Version` + four length-prefixed streams, validate lengths.

3. **Semantic layer:**

   * PackageParts → OPC/ZIP → Section1.m, Package.xml, embedded contents.
   * Permissions → XML.
   * Metadata → XML (+ optional OPC).
   * PermissionBindings → optional DPAPI verification.

4. **Domain layer: M queries & metadata API**

   * Provide convenient structs: `Query{ name, code, is_private, load_to_sheet, load_to_model, group, … }`.

You can implement layer 2 either:

* By hand (simple struct reading integers and slicing a byte array).
* Or via Kaitai-generated parser, which reduces “off-by-n” errors and gives you a visual debugging tool.

### 5.2 Suggested public API

Something roughly like:

```text
struct DataMashup {
    version: u32,
    package_parts: PackageParts,
    permissions: Permissions,
    metadata: Metadata,
    permission_bindings_raw: Vec<u8>,
}

struct PackageParts {
    package_xml: PackageXml,
    main_section: SectionDocument,           // Section1.m parsed into AST or kept as text
    embedded_contents: Vec<EmbeddedContent>  // each with its own SectionDocument, etc.
}

struct Query {
    name: String,               // Section1/QueryName
    section_member: String,     // QueryName
    expression_m: String,       // M code
    metadata: QueryMetadata,
}

struct Metadata {
    general: GeneralMetadata,
    queries: Vec<QueryMetadata>,
    // plus raw XML if you want
}
```

Make sure your domain layer always references **queries by section-member name** (`Section1/Foo`) because that’s what metadata uses.([bengribaudo.com][1])

### 5.3 Step-by-step algorithm

Pseudo‑pipeline:

1. **Open host file**:

   * If extension in `{xlsx,xlsm,xlsb}`:

     1. Open as ZIP.
     2. Enumerate `/customXml/item*.xml`.
     3. Find `<DataMashup xmlns="http://schemas.microsoft.com/DataMashup">`.
     4. Base64 decode its text → `dm_bytes`.
   * Else if extension in `{pbix,pbit}`:

     1. Open as ZIP.
     2. If `DataMashup` entry exists, read bytes → `dm_bytes`.
     3. Else: bail out with “no DataMashup; likely enhanced metadata PBIX”.

2. **Parse top-level binary framing**:

   * Require `dm_bytes.len >= 4+4*4` (min header).
   * Read `version = u32_le(dm_bytes[0..4])`.
   * Read `len_pp, len_perm, len_meta, len_bind` sequentially.
   * Bounds-check each slice; if any overflow, error out.
   * Assign slices:

     ```text
     package_bytes         = dm_bytes[8 .. 8+len_pp]
     permissions_bytes     = next_slice(len_perm)
     metadata_bytes        = next_slice(len_meta)
     permission_bind_bytes = next_slice(len_bind)
     ```

3. **Parse Package Parts**:

   * Treat `package_bytes` as ZIP/OPC:

     * Use normal ZIP lib to locate `/Config/Package.xml`, `/Formulas/Section1.m`, `/Content/*.`
   * Read and parse `Package.xml` into struct:

     * Fields like `CreatedVersion`, `MinimumVersion`, `Culture` are simple XML elements.
   * Read `/Formulas/Section1.m` as UTF‑8:

     * You can initially keep as plain text.
     * Optionally plug in a M-parser to build ASTs (e.g., reuse Ben’s M grammar if available, or roll your own).
   * For each `/Content/{GUID}`:

     * Extract, treat as ZIP again, parse its Section1.m & Config/Formulas.xml; map GUID ↔ `Embedded.Value` semantics.([bengribaudo.com][10])

4. **Parse Permissions**:

   * Interpret `permissions_bytes` as UTF‑8 XML.
   * Extract boolean and enum values; default gracefully if XML missing or malformed (Excel does).([bengribaudo.com][1])

5. **Parse Metadata**:

   * Interpret `metadata_bytes` as UTF‑8 XML. The root is `LocalPackageMetadataFile`.([RADACAD][11])
   * For each `Item` whose `ItemType` is `Formula`, parse its `ItemPath` (`Section1/QueryName`) and associated entries:

     * Example properties: `IsPrivate`, `FillEnabled`, `FillToDataModelEnabled`, `ResultType`, etc. Data Mashup Cmdlets print a good JSON representation.([bengribaudo.com][6])
   * Hydrate your `QueryMetadata` struct by joining metadata and the lines for that query in `Section1.m` (names must match after URL-decoding).

6. **Permission Bindings**:

   * For now: store `permission_bind_bytes` as raw and expose a flag `has_bindings = !permission_bind_bytes.is_empty()`.
   * Optionally on Windows: implement DPAPI verification according to MS‑QDEFF §2.6.

7. **Build query list**:

   * Parse `Section1.m` into section members:

     * M syntax: `section Section1;`, followed by `shared Foo = ...;` etc.
     * Many tools (Data Mashup Cmdlets, Imke Feldmann’s functions) already split Section1.m into members; you can mimic their heuristics (split by semicolons and `shared` declarations).([bengribaudo.com][6])
   * Map each `Query` to its `QueryMetadata` entry via `SectionName/MemberName` (usually `Section1/Foo`).

---

## 6. Reverse-engineering “beyond the spec”

MS‑QDEFF covers the big pieces, but there are still corners that benefit from RE:

* Binary metadata property values (e.g., hashes, change keys).
* Behavior differences between Excel and older PBIX.
* Noncompliant or legacy files (Ben explicitly mentions adding support for those).([bengribaudo.com][12])

### 6.1 Workflow for unknown binary properties

1. **Locate property**:

   * In Metadata XML, look for attributes or text that are marked as base64 or appear as binary blobs.
2. **Isolate bytes**:

   * Base64-decode that property; save as a standalone `.bin`.
3. **Run binwalk**:

   * If it identifies ZIP/zlib/etc., you know it’s a nested container.
   * If not, look at entropy and patterns: is it fixed-length? Contains recognizable ASCII?
4. **Define a mini Kaitai spec**:

   * Start with length-prefixed fields, GUIDs, timestamps, etc.
   * Iterate in Web IDE until the hex view and parsed fields line up sensibly.
5. **Codify**:

   * Once you understand the structure, add it as a dedicated decoder, but keep the original bytes around for future-proofing.

This “property‑focused” RE loop is where you can be clever and incremental: you don’t need to decode every obscure field up-front; just enough to support your product’s use cases (e.g., showing which queries are loaded where, last refresh schema, etc.).

---

## 7. Testing & validation plan

To have high confidence that your DataMashup parser is correct and robust:

### 7.1 Golden oracles

Use existing tools as oracles:

* **Data Mashup Explorer**:

  * Cross-check your interpretation of Package Parts, Metadata, and embedded contents.([bengribaudo.com][12])
* **Data Mashup Cmdlets**:

  * `Export-DataMashup -Raw` should produce the same Section1.m text as your parser.
  * `Export-DataMashup -Item Metadata` JSON view is a good reference for the semantics of metadata.([bengribaudo.com][6])

### 7.2 Corpus

Build a sample set that includes:

* Multiple Excel workbooks using Power Query:

  * Queries loading to sheet, to data model, to both.
  * Queries grouped into folders.
  * Hidden/non-loaded queries.
  * Embedded contents using `Embedded.Value`.
* Older PBIX/PBIT:

  * With, and without, embedded contents.
* Intentionally corrupted files:

  * Truncated DataMashup base64.
  * Incorrect length fields, etc.

### 7.3 Property-based tests

For example:

* Round-trip tests with Data Mashup Cmdlets:

  1. Use Cmdlets to export raw mashup and metadata.
  2. Use your library to parse those bytes and re-emit them.
  3. Use Cmdlets to re-open and compare.

* Invariants:

  * Sum of lengths matches stream size.
  * Number of queries in metadata equals number of shared members in Section1.m (modulo known oddities like `/Section1/Query/Step` entries).([bengribaudo.com][1])

---

## 8. Concrete use of binwalk & Kaitai in your workflow

Putting it all together:

1. **Exploration stage**:

   * Use `binwalk datamashup.bin` to quickly see where the ZIP(s) are.
   * Use Kaitai Web IDE with a simple `datamashup.ksy` to visualize the framing while you manually compare to MS‑QDEFF’s diagrams.
2. **Spec refinement**:

   * Gradually add fields (e.g., metadata substructures) to the KSY file as you understand them.
   * Use Web IDE’s partial object-tree on errors to debug your struct definitions.
3. **Code generation**:

   * Compile KSY into your implementation language and wrap that in a clean, ergonomic API.
4. **Regression**:

   * During development of your Rust library or other implementation, keep running binwalk and Kaitai on your parsing output slices to ensure lengths and offsets are still correct after refactors.

---

## 9. Summary: the mental model

If you want one compact mental picture to guide implementation:

* Excel / PBIX host file = **OPC/ZIP**.
* Inside that, **DataMashup** = base64 (Excel) or raw (PBIX) **MS‑QDEFF top-level stream**.
* MS‑QDEFF top-level stream =
  `Version(=0)` + `len+PackageParts(OPC ZIP)` + `len+Permissions(XML)` + `len+Metadata(XML)` + `len+PermissionBindings(binary)`.
* `PackageParts` contains:

  * `/Config/Package.xml` (who wrote this, culture, versions).
  * `/Formulas/Section1.m` (all M code).
  * `/Content/*` (embedded mini‑mashups for `Embedded.Value`).
* `Metadata` glues the M code to workbook/model semantics.
* `PermissionBindings` is a DPAPI-protected hash that you can safely treat as opaque for read-only tools.

Binwalk helps you *find* and *sanity-check* embedded containers; Kaitai helps you *encode the spec as executable schema* and avoid off‑by‑one bugs.

If you build your parser as a clean hierarchy with strong invariants at each layer, you’ll be able to bolt it straight into your Excel diff engine (or any other tooling) and have very high confidence it will behave correctly—even on weird, non-standard, or future files.

[1]: https://bengribaudo.com/blog/2020/04/22/5198/data-mashup-binary-stream "The Data Mashup Binary Stream: How Power Queries Are Stored | Ben Gribaudo"
[2]: https://community.powerbi.com/t5/Desktop/DataMashup-file-no-longer-exists/td-p/1145141?utm_source=chatgpt.com "DataMashup file no longer exists"
[3]: https://www.thebiccountant.com/2017/10/15/bulk-extracting-power-query-m-code-from-multiple-pbix-files-in-power-bi/?utm_source=chatgpt.com "Bulk-extracting Power Query M-code from multiple pbix ..."
[4]: https://www.thebiccountant.com/2017/10/15/bulk-extracting-power-query-m-code-from-multiple-pbix-files-in-power-bi/ "Bulk-extracting Power Query M-code from multiple pbix files in Power BI –"
[5]: https://bengribaudo.com/tools/datamashupcmdlets?utm_source=chatgpt.com "Data Mashup Cmdlets"
[6]: https://bengribaudo.com/tools/datamashupcmdlets "Data Mashup Cmdlets | Ben Gribaudo"
[7]: https://github.com/ReFirmLabs/binwalk?utm_source=chatgpt.com "ReFirmLabs/binwalk: Firmware Analysis Tool"
[8]: https://crates.io/crates/binwalk?utm_source=chatgpt.com "binwalk - crates.io: Rust Package Registry"
[9]: https://doc.kaitai.io/user_guide.html?utm_source=chatgpt.com "Kaitai Struct User Guide"
[10]: https://bengribaudo.com/blog/2020/06/04/5298/shedding-light-on-the-mysterious-embedded-value "Shedding Light on the Mysterious Embedded.Value | Ben Gribaudo"
[11]: https://radacad.com/exposing-m-code-and-query-metadata-of-power-bi-pbix-file/ "Exposing M Code and Query Metadata of Power BI (PBIX) File - RADACAD"
[12]: https://bengribaudo.com/tools/datamashupexplorer "Data Mashup Explorer | Ben Gribaudo"

---

Last updated: 2025-11-24 14:52:35