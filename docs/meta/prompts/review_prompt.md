# Codebase Context for Review

## Directory Structure

```text
/
  README.md
  core/
    Cargo.lock
    Cargo.toml
    src/
      main.rs
    tests/
      integration_test.rs
  docs/
    competitor_profiles/
      Excel Diff Engine Competitive Landscape.md
      Excel Diff Tool Licensing Scheme Design.md
    design/
    meta/
      notes.md
      todo.md
      logs/
      plans/
      prompts/
        generate_review_context.py
        new_idea_rubric.md
        planning_instructions.txt
        recreate_success_of_excel_diff_discovery.md
        review_prompt.md
        unsubmitted_deep_research_prompts.md
        conversation_data/
          initial_general_purpose_tools_revenue_conversation.md
          initial_quick_software_product_brainstorm_conversation.md
      results/
      retrospectives/
      risk/
      scenarios/
    rust_docs/
      excel_diff_difficulty_analysis.md
      excel_diff_education_plan.md
      excel_diff_marketing_plan.md
      excel_diff_meta_programming.md
      excel_diff_m_query_parse.md
      excel_diff_product_differentiation_plan.md
      excel_diff_technical_document.md
      excel_diff_testing_plan.md
  fixtures/
    manifest.yaml
    pyproject.toml
    README.md
    requirements.txt
    generated/
      corrupt_base64.xlsx
      db_equal_ordered_a.xlsx
      db_equal_ordered_b.xlsx
      db_row_added_b.xlsx
      grid_large_dense.xlsx
      grid_large_noise.xlsx
      minimal.xlsx
      m_change_literal_b.xlsx
      no_content_types.xlsx
      pg1_basic_two_sheets.xlsx
      pg1_empty_and_mixed_sheets.xlsx
      pg1_sparse_used_range.xlsx
      pg2_addressing_matrix.xlsx
      pg3_value_and_formula_cells.xlsx
      random_zip.zip
    src/
      generate.py
      __init__.py
      generators/
        base.py
        corrupt.py
        database.py
        grid.py
        mashup.py
        perf.py
        __init__.py
    templates/
      base_query.xlsx
```

## File Contents

### File: `.gitignore`

```
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
```

---

### File: `README.md`

```markdown
# Excel Diff Monorepo

This repository consolidates the plan, implementation, and test fixtures for the Excel Diff engine.

## Directory Structure

- **`core/`**: The Rust implementation of the diff engine.
- **`fixtures/`**: Python tools to generate Excel file fixtures for testing.
- **`docs/`**: Project documentation, plans, and meta-programming logs.

## Quick Start

### Core (Rust)
```bash
cd core
cargo build
cargo test
```

### Fixtures (Python)
```bash
cd fixtures
# Install dependencies (using uv or pip)
uv pip install -r requirements.txt
# Generate fixtures
python src/generate.py
```

## Documentation
See `docs/` for detailed architectural plans and meta-programming logs.

```

---

### File: `core\Cargo.toml`

```yaml
[package]
name = "excel_diff"
version = "0.1.0"
edition = "2024"

[dependencies]
```

---

### File: `core\src\main.rs`

```rust
fn main() {
    println!("Hello, world!");
}
```

---

### File: `core\tests\integration_test.rs`

```rust
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated"); 
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(path.exists(), "Fixture minimal.xlsx should exist at {:?}", path);
}

```

---

### File: `docs\competitor_profiles\Excel Diff Engine Competitive Landscape.md`

```markdown


# **Strategic Competitive Analysis: The Modern Excel Diff Engine Landscape**

## **1\. Executive Strategic Overview**

The global financial modeling and data analysis landscape is currently undergoing a silent but profound architectural shift, transitioning from the traditional "grid-centric" paradigm of Excel to a "model-centric" approach characterized by the usage of Power Query (Get & Transform), Power Pivot, and DAX (Data Analysis Expressions). This evolution, often termed "Modern Excel," has effectively transformed the world’s most popular spreadsheet software into a sophisticated Business Intelligence (BI) platform. However, the ecosystem of support tools—specifically those designed for auditing, version control, and change detection—has largely failed to pivot in tandem with this architectural transformation.

The current market for Excel comparison tools is bifurcated. On one side, legacy incumbents dominate the Windows ecosystem with powerful but aging COM-based add-ins that excel at formula auditing but remain blind to the semantic logic of modern data models. On the other side, a fragmented collection of superficial text-based comparison tools attempts to treat Excel files as generic data containers, ignoring the complex dependency graphs that define modern financial models.

This report evaluates the competitive landscape for a proposed **Rust-based, Mac-first, Modern Excel Diff Engine**. The analysis validates that a significant market void exists at the intersection of three critical trends: the growing professional usage of macOS in finance and data science, the increasing complexity of "invisible" logic in Power Query and DAX, and the limitations of current cloud-based solutions regarding data privacy and performance.

The incumbent landscape is populated by "superficial competitors" like Diffchecker, which offer broad but shallow utility, and "legacy workflow competitors" like Ablebits and Operis, which offer deep utility but suffer from platform lock-in (Windows-only) and technological debt. A new class of "workflow competitors" like xltrail has begun to address the version control gap, yet their reliance on cloud infrastructure creates friction for privacy-conscious users.

Furthermore, the emergence of Large Language Models (LLMs) such as ChatGPT and Microsoft Copilot presents both a disruption and an opportunity. While these AI tools excel at semantic summarization, they lack the deterministic precision required for rigorous audit trails. The proposed Rust-based engine, by offering deterministic accuracy with local-first performance, is uniquely positioned to serve as the "ground truth" layer that purely probabilistic AI models cannot replicate.

The following comprehensive analysis details the technical, functional, and commercial characteristics of the current market, confirming the strategic viability of a high-performance, cross-platform engine targeting the logic of Modern Excel.

## **2\. The Technological Context: The Shift from Grid to Graph**

To accurately evaluate the competitive threat posed by existing tools, it is imperative to first understand the technical chasm between "Classic Excel" and "Modern Excel." The failure of most incumbents to bridge this chasm constitutes the primary market opportunity for a new entrant.

### **2.1 The Architectural Bifurcation of the Workbook**

Historically, an Excel workbook (.xls or early .xlsx) was a two-dimensional grid of cells. Value resided in the intersection of rows and columns, and logic was defined by A1 referencing B2. In this era, a "diff" tool merely needed to parse XML structures to identify that the value in Sheet1\!A1 changed from "100" to "200."

Modern Excel, introduced progressively since Excel 2010 and solidified in Excel 2016/365, introduces two invisible layers of abstraction that traditional parsing engines miss:

1. **The ETL Layer (Power Query/M):** Logic now resides in the DataMashup binary stream within the OpenXML package. This stream contains the M code (Power Query Formula Language) which extracts, transforms, and loads data. A user might change a filter step in Power Query that excludes 20% of revenue data. A traditional grid-based diff tool will see the *result* (changed values in the output table) but will be completely blind to the *cause* (the change in the M script).  
2. **The Semantic Layer (DAX/Data Model):** Measures calculated in the internal VertiPaq engine do not exist in the grid until invoked by a PivotTable. A DAX measure defined as Total Sales \= SUM(Sales\[Amount\]) might be changed to Total Sales \= CALCULATE(SUM(Sales\[Amount\]), ALL(Date)). This change fundamentally alters the logic of the entire report, yet because the measure definition lives in the Model.bim or internal binary structures rather than a cell, legacy tools like Spreadsheet Compare often fail to visualize this change meaningfully.

### **2.2 The OpenXML and Binary Challenge**

The file format itself acts as a barrier to entry. While .xlsx files are ostensibly zipped XMLs, the critical Modern Excel components are often encapsulated in binary blobs (Base64 encoded streams) to improve performance and obfuscate logic.

* **The XML Noise Problem:** Simply unzipping an Excel file and running a standard git diff on the XML generates massive amounts of noise. A simple act of saving the file can reorder internal IDs, change calculation chain attributes, or update metadata timestamps, resulting in thousands of "changes" that are semantically meaningless to the user.  
* **The Binary Black Box:** The DataMashup stream (Power Query) is a proprietary binary format. Parsing it requires specific reverse-engineering or the use of Microsoft's own SDK libraries (which are typically.NET based). This explains why so many Python-based or generic tools fail to support Power Query diffing—they lack the specialized parsers to interpret the binary stream.

### **2.3 The Rust and WASM Strategic Advantage**

The choice of Rust for the proposed engine is not merely an implementation detail but a strategic differentiator against the primary incumbent architectures:

| Architecture | Representative Tools | Strengths | Weaknesses |
| :---- | :---- | :---- | :---- |
| **VBA / COM** | Ablebits, Operis (OAK), Inquire | Deep integration with Excel UI. | **Windows Only.** Single-threaded. Security risks. |
| **.NET / C\#** | VSTO Add-ins, Spreadsheet Compare | Native access to MS libraries. | **Windows Only.** Requires runtime installation. |
| **Python** | xlwings, Custom Scripts | Flexible, data science friendly. | Slow start-up. Dependency hell. **Memory intensive.** |
| **JavaScript / Office.js** | Script Lab, Modern Add-ins | Cross-platform (Web/Mac/Win). | **Sandboxed.** Cannot read full file system. Limited API. |
| **Rust / WASM** | **Proposed Product** | **Native Speed. Memory Safety. Zero Dependencies.** | Higher dev complexity. |

The Rust architecture allows for direct binary parsing of the OpenXML package without relying on the slow, heavy Excel Object Model. This enables "instant" diffs of large (100MB+) files, a capability that COM-based add-ins struggle to match due to the overhead of the Excel application itself. Furthermore, Rust's ability to compile to WebAssembly (WASM) 1 enables a "write once, deploy everywhere" strategy, allowing the engine to run locally on a Mac, in a browser, or on a Windows desktop with equal fidelity.

## **3\. Competitive Segment: The "Superficial" Competitors**

*Definition: Tools that treat Excel files as generic data grids or text sources, lacking context of Excel's internal dependency graph, formula chain, or modern data model structures.*

This segment contains the highest volume of tools, often targeting casual users. While they serve a purpose for quick, ad-hoc comparisons, they pose the lowest functional threat to a specialized Modern Excel engine, primarily because they solve a "content" problem rather than a "logic" problem.

### **3.1 Diffchecker (Desktop and Web)**

Diffchecker 2 is a widely recognized brand for general file comparison, offering solutions for text, images, and PDFs.

* **Workflow:** The user uploads two Excel files to the web interface or selects them in the desktop app. The tool converts the spreadsheets into a static grid view or text representation to highlight cell-level discrepancies.  
* **Capabilities & Limitations:**  
  * **Value-Centric:** Diffchecker is effective at identifying that Cell A1 changed from "Foo" to "Bar." It provides a clean, visual heatmap of changes.4  
  * **Context Blindness:** The tool treats the Excel file effectively as a CSV. It does not visualize the "Why." If a value changed because a formula was updated, Diffchecker shows the result difference, not the formulaic driver.  
  * **Row & Size Limits:** The web version imposes strict limits on file size and row counts, rendering it useless for serious financial models or large datasets.5  
  * **Modern Excel Gap:** It has zero visibility into Power Query M code, the Data Model, or VBA macros. A user relying on Diffchecker to audit a financial model would miss 90% of the risk vectors if the logic is embedded in Modern Excel features.6  
* **Strategic Implication:** Diffchecker captures the "low-end" market of users who need to compare simple lists. It is not a competitor for professional modelers.

### **3.2 Generic Text/Code Diff Tools (WinMerge, Beyond Compare)**

Developers often attempt to force-fit standard software development diff tools into the Excel workflow. Tools like WinMerge 7 and Beyond Compare are industry standards for code, but they struggle with the binary nature of .xlsx.

* **The "TextConv" Approach:** Advanced users often configure .gitattributes to use a "text converter" (like git-xlsx-textconv 8) which strips Excel data into a text format before diffing.  
* **Capabilities:** This allows standard tools like git diff to show changes in cell values.  
* **Limitations:**  
  * **Destructive Abstraction:** The conversion process destroys the grid structure. A change in Cell Z100 might appear lines away from a change in Cell A1, losing the spatial context that is critical for spreadsheet comprehension.  
  * **Noise:** As noted in user discussions, unzipping and diffing the raw XML generates immense noise.10 A simple change in print settings or window position can trigger hundreds of lines of XML diffs, obscuring the actual data changes.  
* **Strategic Implication:** These tools are "hostile" to non-technical finance professionals. They serve the developer demographic but fail the analyst demographic.

### **3.3 The "Data Science" Approach: Pandas-Based Comparison**

With the rise of Python in finance, many analysts build custom diff scripts using the Pandas library.11

* **Workflow:** Scripts typically load two Excel files into DataFrames (pd.read\_excel) and run a comparison function (df.compare or DeepDiff 13).  
* **Capabilities:** Extremely powerful for comparing *data values* in large datasets. It handles millions of rows more efficiently than Excel itself.  
* **Limitations:**  
  * **Loss of Spreadsheet Integrity:** Loading into Pandas flattens the workbook. Formulas are evaluated or lost; formatting is stripped; VBA and Power Query are completely ignored.14  
  * **Setup Friction:** It requires a Python environment, dependency management (pip install pandas openpyxl), and coding knowledge. This creates a high barrier to entry for the average Excel user.  
* **Strategic Implication:** While powerful for *data* validation (e.g., comparing two CSV outputs), this approach is not a substitute for *model* auditing. A Rust-based tool offers the performance of this approach without the setup friction or data loss.

## **4\. Competitive Segment: The Legacy Windows Incumbents**

*Definition: Established commercial add-ins deeply integrated into the Windows Excel ecosystem via COM (Component Object Model) or VSTO technology. They offer robust features for "Classic Excel" but struggle with platform portability.*

This segment represents the primary "professional" competition. These tools are entrenched in large financial institutions and accounting firms. Their dominance, however, is built on the legacy Windows-Intel (Wintel) architecture, creating a massive vulnerability as the workforce shifts toward Mac and Cloud.

### **4.1 Ablebits Ultimate Suite**

Ablebits 15 is a dominant player in the general Excel utility market, offering a "Swiss Army Knife" of over 70 tools.

* **Product Positioning:** Ablebits positions itself as a productivity enhancer for daily tasks (merge, split, clean, de-dupe). "Compare Sheets" is just one tool in a massive suite.17  
* **Comparison Capabilities:**  
  * **Algorithm:** It focuses on cell-by-cell comparison and row alignment. It is particularly good at aligning rows in two tables that might have been sorted differently, using key columns.17  
  * **Presentation:** Differences are highlighted directly in the workbook (e.g., Cell A1 turns red). This is highly intuitive for users who want to stay inside Excel.  
* **Pricing Strategy:** Ablebits uses a perpetual licensing model with volume discounts. A personal license is \~$49, and a business license is \~$99.15 This is highly attractive to SMBs and individuals compared to SaaS subscriptions.  
* **The Critical Weakness:**  
  * **Windows Only:** The documentation explicitly states: "Note: for Windows only".15 There is no Mac support because the tool is built on the.NET/COM framework, which has no equivalent on macOS.  
  * **Scope Limitation:** Ablebits is a "Data Cleaning" tool, not a "Model Audit" tool. It does not parse VBA, it does not diff Power Query M code, and it does not visualize DAX measures. It helps a user clean a mailing list, not debug a billion-dollar financial model.

### **4.2 Operis Analysis Kit (OAK) & Spreadsheet Advantage**

Operis is a heavyweight in the Project Finance world. Their software, OAK 18, is considered the gold standard for model auditing in infrastructure and banking.

* **Target Persona:** High-end financial modelers, project finance auditors, and infrastructure bankers.  
* **Comparison Capabilities:**  
  * **Structural Alignment:** OAK excels at comparing two versions of a complex model where rows may have been inserted or deleted. It uses sophisticated logic to "re-align" the models before comparing, ensuring that row shifts don't register as thousands of errors.19  
  * **Risk Analysis:** Beyond simple diffing, OAK analyzes the *risk* of the spreadsheet—identifying hardcoded numbers in calculation blocks, inconsistent formulas, and circular references.20  
* **Commercials:** OAK has moved to a subscription model (\~£311/year) 21, reflecting its premium enterprise positioning.  
* **The Modern Excel Gap:**  
  * **Legacy Architecture:** Like Ablebits, OAK is a Windows-only COM add-in. Mac users are explicitly told to use virtualization software like Parallels or VMWare 22, which introduces significant performance overhead and battery drain.  
  * **VBA vs. Power Query:** OAK was built during the peak of the VBA era. While it handles traditional formulas perfectly, its ability to deeply audit the dependencies of a Power Query loaded into the Data Model is limited compared to its mastery of the grid. It views the world through the lens of cell dependencies, not ETL data flows.

### **4.3 Microsoft Spreadsheet Compare & Inquire**

Microsoft provides its own native set of audit tools, "Spreadsheet Compare" and "Inquire".23

* **Availability:** These tools are gated behind specific enterprise licenses (Office Professional Plus or Microsoft 365 Enterprise). They are **not** available in Home or Business Standard editions, and crucially, they are **not available on Mac**.24  
* **Capabilities:**  
  * **Inquire:** Provides a visual map of workbook relationships (links between files) and worksheet dependencies.  
  * **Spreadsheet Compare:** A standalone desktop application (separate from Excel) that provides a robust cell-by-cell diff, formula dependency mapping, and even VBA code comparison.26  
* **The User Experience Gap:**  
  * **Friction:** Because it is a separate executable (SpreadsheetCompare.exe), the workflow is disjointed. Users must launch the app, browse for files, and run the compare. It is not seamless.  
  * **Stagnation:** These tools have seen little update in the last decade. They predate the dominance of Power Query and do not provide a semantic diff of M code steps or DAX measures in a user-friendly manner.  
  * **Platform Lock:** The lack of Mac support is a persistent source of frustration in user forums, with Microsoft support explicitly confirming that the COM technology required does not exist on macOS.24

### **4.4 PerfectXL**

PerfectXL 27 is a specialized auditing tool that positions itself as a "Risk Finder."

* **Comparison Capabilities:** PerfectXL Compare detects changes in data, formulas, structure, and VBA. It claims to "hide side-effects" (e.g., changes caused purely by row insertion) to focus on actual modifications.15  
* **Modern Excel Support:** Snippets indicate that PerfectXL has begun to address the Modern Excel gap, claiming "Extensive Power Query support" and listing "Queries & Connections" as supported object types.28 This suggests they are technically ahead of Ablebits and OAK in parsing the DataMashup stream.  
* **Platform Ambiguity:** While some marketing materials mention Mac support for their *Add-in* (likely the Office.js version for basic tasks), the heavy-duty comparison engine appears to rely on backend technologies that may bind it to Windows for full functionality.30  
* **Strategic Verdict:** PerfectXL is a strong functional competitor. The key differentiator for a Rust-based tool would be the "Local Native App" experience on Mac, avoiding the dependency on an Office.js add-in which is often slower and sandboxed.

## **5\. The "Workflow" Competitors: Version Control & Cloud**

*Definition: Tools that attempt to integrate Excel into a developer-style workflow (Git) or provide dedicated, deep-comparison capability via the cloud.*

This segment represents the "future" of Excel management, attempting to bring software engineering discipline (Version Control, CI/CD) to financial modeling.

### **5.1 xltrail: The Primary Direct Competitor**

xltrail 31 is the closest direct competitor in terms of product vision: "GitHub for Excel."

* **Architecture:** It is a web-based SaaS platform. Users sync their Excel files to xltrail's cloud (or a self-hosted server), where the application parses the file and renders a visual diff in the browser.  
* **Modern Excel Capabilities:**  
  * **Power Query:** xltrail explicitly claims to track **"Power Queries"** and **"Defined Names (includes Lambda Functions)"**.32 This confirms they have successfully reverse-engineered or utilized libraries to parse the DataMashup binary.  
  * **VBA:** It provides rich, color-coded diffs for VBA modules, treating them as code rather than text.33  
  * **Visualization:** The diffs are rendered beautifully in the browser, allowing for easy collaboration and comment threads.  
* **Pricing & Positioning:** xltrail targets the enterprise market, with pricing starting at $35/user/month (billed annually).31 This creates a high barrier for freelancers and small consultancies.  
* **The Privacy & Connectivity Friction:**  
  * **Data Sovereignty:** Being web-first means users must upload sensitive financial models to xltrail's cloud. While they offer an "Enterprise Self-Hosted" version for air-gapped installs 34, this is complex to deploy.  
  * **Dependency:** Users cannot diff a file offline. They need internet access and an account.  
* **Strategic Implication:** xltrail validates the market demand for "Deep Diffing." However, its SaaS model leaves a massive opening for a "Local-First" tool. A Rust desktop app offers the same deep inspection capabilities but without the privacy risk of cloud upload and without the recurring subscription fatigue.

### **5.2 Schematiq and Coherent Spark: The "Logic Extractors"**

Tools like Schematiq 35 and Coherent Spark operate on a different premise: they convert Excel logic into APIs.

* **The Approach:** Instead of helping users *manage* Excel files, they help users *escape* Excel files by turning the logic into code/APIs that can be integrated into enterprise systems.36  
* **Relevance:** These are not direct competitors for a "diff" tool, but they compete for the same budget (Excel Risk Management).  
* **Schematiq's Workbench:** Schematiq offers a "Workbench" that acts as an IDE for Excel, allowing users to diff and manage logic before conversion.35 This validates that the "IDE-ification" of Excel is a mature trend.

### **5.3 Equals and GRID: The "Excel Replacements"**

New entrants like Equals 37 and GRID 38 attempt to solve the version control problem by moving the spreadsheet entirely to the browser.

* **The Promise:** By controlling the entire stack, Equals can offer built-in "Git-style" version control for every cell.39  
* **The Reality:** Financial modeling, legal schedules, and investment banking pitch decks still rely on .xlsx files as the universal exchange format. Analysts cannot simply switch to Equals if their clients use Excel.  
* **Strategic Verdict:** These tools create silos. A diff engine that works with the *existing* universe of .xlsx files has a much larger Total Addressable Market (TAM) than a tool requiring platform migration.

## **6\. The "Mac Gap" and Platform Dynamics**

One of the most significant findings of this research is the persistent neglect of the Mac Excel user. This demographic is not niche; it includes the entire startup ecosystem, vast swathes of the creative and tech industries, and a growing number of modern finance professionals.

### **6.1 The History of Disparity**

Excel for Mac has historically been a second-class citizen. It lacked Power Pivot support for years. Even today, while Power Query is available on Mac, the **COM Add-in architecture** is entirely absent.25

* **No COM:** This means tools like Ablebits, OAK, and Spreadsheet Inquire simply *cannot* run on macOS. The underlying technology (ActiveX/COM) is Windows-proprietary.  
* **Virtualization Pain:** The standard advice for Mac users needing these tools is to run Windows via Parallels Desktop.22 This is an expensive workaround ($100/yr for Parallels \+ Windows License) that degrades system performance and battery life.

### **6.2 The M-Series (Apple Silicon) Opportunity**

The transition of Apple computers to ARM-based M-series chips has widened the performance gap.

* **Emulation vs. Native:** Running Intel-based Windows apps on an ARM Mac adds layers of emulation. A **Native Rust Application** compiled for Apple Silicon runs directly on the hardware, offering blazing-fast performance.  
* **The "Mac-First" Persona:** The target user is likely a high-end consultant or analyst who values design, speed, and the Unix-based environment of macOS. They are likely using tools like Python, R, or SQL alongside Excel. A CLI-friendly, fast, native diff tool aligns perfectly with this persona's workflow.

## **7\. The AI & LLM Disruption: Threat or Multiplier?**

The user query explicitly asks to evaluate LLM-based competitors. The rise of Generative AI (ChatGPT, Claude, Copilot) fundamentally alters the landscape of "Code Understanding," which includes Excel formulas and M code.

### **7.1 ChatGPT Code Interpreter (Advanced Data Analysis)**

OpenAI's Code Interpreter allows users to upload Excel files and perform analysis via Python (Pandas).40

* **Capability:** Users can ask, "Compare these two files and tell me if the profit changed." The LLM writes a Python script to load the files and calculates the difference.  
* **Limitations:**  
  * **Probabilistic vs. Deterministic:** An auditor needs 100% certainty. If an LLM hallucinates that "No changes were found" because it truncated the file processing due to token limits, the auditor faces a liability risk. LLMs are notoriously bad at brute-force, exhaustive comparison of massive datasets without skipping rows.  
  * **Structure Blindness:** Unless specifically prompted with complex instructions, Code Interpreter typically loads the *data values* only. It rarely extracts hidden VBA macros, named ranges, or Power Query logic unless the user knows exactly how to prompt for it.41  
  * **Privacy:** Uploading a pre-IPO cap table or a merger model to a public LLM is a violation of strict NDAs in finance.

### **7.2 Microsoft Copilot in OneDrive**

Microsoft has rolled out a file comparison feature in OneDrive powered by Copilot.42

* **Capability:** "Select up to 5 files and compare differences."  
* **Limitation:** This is designed for high-level semantic summarization ("This version adds a paragraph about Q3 goals"). It is not a technical audit tool. It does not provide a line-by-line diff of the M-code script that runs the ETL. It is a productivity feature for document management, not a quality assurance tool for model development.

### **7.3 The "Hybrid" Strategic Opportunity**

The Rust engine should not *fight* LLMs but *feed* them.

* **The "Grounding" Strategy:** The weakness of LLMs is their lack of access to ground truth. The Rust engine provides the **Deterministic Diff** (a JSON object listing every added row, changed formula, and modified dependency).  
* **Synergy:** The product can integrate an LLM API (local or cloud) to *summarize* the deterministic findings.  
  * *Rust Output:* Formula changed in C4: \=A1+B1 \-\> \=A1\*B1.  
  * *LLM Narrative:* "The calculation logic for Total Revenue (C4) shifted from additive to multiplicative, which is the primary driver for the 300% increase in projected profit."  
* This approach uses the Rust engine as the "Safety Layer" that prevents hallucination, while using the LLM as the "Interface Layer" that explains complex changes in plain English.

## **8\. The "Modern Excel" Gap: A Technical Deep Dive**

The research confirms that while many tools exist, they almost universally fail to address the specific artifacts of Modern Excel. This technical gap is the product's primary moat.

### **8.1 The Power Query (M) Challenge**

* **Current State:** Most tools (Ablebits, Diffchecker) only compare the *output table* of a query. If a user changes a filter step in Power Query but the preview data hasn't refreshed, the tool sees no change.  
* **The Rust Solution:** A Rust engine can unzip the .xlsx package, locate the customXml/item1.xml (or the encrypted DataMashup binary), decode the package, and extract the raw Section1.m script.  
* **The Feature:** Provide a **Code Diff** for Power Query.  
  Code snippet  
  \- Source \= Excel.CurrentWorkbook(){}\[Content\]  
  \+ Source \= Excel.CurrentWorkbook(){}\[Content\]

  This level of visibility is currently only available in high-end tools like xltrail.32 Bringing this to a local Mac app is a massive differentiator.

### **8.2 The DAX & Data Model Challenge**

* **Current State:** DAX measures are stored in the Model schema (Model.bim), often embedded deep within binary structures. Standard diff tools are blind to them. Tools like **ALM Toolkit** 43 exist for Power BI to diff Semantic Models, but there is no lightweight tool for *Excel* users to diff DAX measures on macOS.  
* **The Rust Solution:** Parse the Model file within the .xlsx package. Extract measure definitions and relationship graphs.  
* **The Feature:** A "Measure Change Log."  
  * *Previous:* Margin % \= DIVIDE(\[Profit\],)  
  * *Current:* Margin % \= DIVIDE(\[Profit\],, 0\)  
  * *Insight:* This change prevents "Divide by Zero" errors, a critical logic improvement that a cell-diff tool would never explicitly reveal.

## **9\. Strategic Positioning and Recommendations**

Based on the exhaustive analysis of the competitive landscape, the following strategic recommendations are proposed for the Rust-based Modern Excel Diff Engine.

### **9.1 Target Persona & Positioning**

* **Primary Persona:** **The "Modern Finance Architect."** This user is a CFO of a tech startup, a freelance FP\&A consultant, or a data analyst. They use a MacBook Pro. They know Power Query. They are frustrated by the need to use Parallels to run legacy Windows tools.  
* **Value Proposition:** "The only native Mac Audit tool that speaks Modern Excel."  
  * *vs. Ablebits:* "We audit the Logic (M/DAX), not just the Grid."  
  * *vs. xltrail:* "Your data never leaves your device. No monthly fees."  
  * *vs. Diffchecker:* "We understand your Financial Model, not just your text."

### **9.2 Feature Priorities (The "Moat")**

1. **Performance (Rust):** Market it as "Instant Diff." Open 100MB+ files in seconds. The memory safety of Rust allows for processing massive files that crash Python scripts.  
2. **Privacy (Local-First):** Attack xltrail's weakness. Position "Local Analysis" as a security feature.  
3. **Modern Excel Fidelity:**  
   * Explicit "Power Query Diff" tab with M-code syntax highlighting.  
   * Explicit "DAX Diff" tab showing measure definition changes.  
   * Data Model Schema diff (Relationships changed from 1:Many to Many:Many).  
4. **Git Integration:** Offer a CLI (command line interface) that hooks into git diff. This allows the tool to serve as the "viewer" for Git repositories, replacing the clunky git-xlsx-textconv workflow.

### **9.3 Pricing Model**

The market is fatigued by SaaS subscriptions.

* **Recommendation:** A **Hybrid Licensing Model**.  
  * **Free Tier:** Basic Cell/Grid diff (competes with Diffchecker to build user base).  
  * **Pro License (Perpetual \+ 1yr updates):** Unlocks Power Query, DAX, VBA diffing, and Git integration. Pricing in the $50-$100 range matches the "Utility" mental model set by Ablebits 15 but offers superior value for Mac users.  
  * **Team License:** Includes CLI tools for CI/CD integration and headless operation.

## **10\. Comprehensive Comparison Matrix**

The following matrix summarizes the competitive landscape, highlighting the unique position of the proposed Rust/Mac engine.

| Feature | Rust/Mac Engine (Prop) | xltrail (Direct) | Ablebits (Legacy) | PerfectXL (Audit) | MS Inquire (Native) | ChatGPT (LLM) |
| :---- | :---- | :---- | :---- | :---- | :---- | :---- |
| **Platform** | **Mac & Windows (Native)** | Web (Browser) | Windows Only | Windows Only | Windows Only | Web |
| **Privacy** | **Local / Offline** | Cloud (or Self-host $$) | Local | Local | Local | Cloud (Data Risks) |
| **Performance** | **High (Rust)** | Medium (Latency) | Medium (COM) | Medium | Medium | Low (Tokens) |
| **Cell Diff** | Yes | Yes | Yes | Yes | Yes | Yes (Small files) |
| **VBA Diff** | Yes | Yes | No | Yes | Yes | Poor |
| **Power Query Diff** | **Yes (Code View)** | Yes | No | Yes (Limited) | No | Poor |
| **DAX Diff** | **Yes (Measure Logic)** | Partial | No | Partial | No | No |
| **Git Integration** | **Native CLI** | Native | No | No | No | No |
| **Cost Model** | One-time / License | SaaS (Recurring) | One-time | One-time / SaaS | Enterprise License | Sub / Usage |

## **11\. Conclusion**

The market for Excel tools is crowded with legacy players (Ablebits, OAK) that are technically stagnant, Windows-bound, and grid-focused. The "Modern Excel" revolution has created a massive discrepancy between what users *do* (build complex ETL and data models) and what their tools can *audit*.

By leveraging **Rust** for performance and cross-platform capability (specifically targeting the neglected **Mac** market), and by focusing purely on the "Deep Diff" of **Power Query and DAX**, the proposed product addresses a critical, high-value gap. It bypasses the "Superficial" competition of Diffchecker and creates a compelling, privacy-centric alternative to the expensive, web-based xltrail. The integration of LLMs should be viewed not as a threat, but as a complementary feature layer (summarization) that sits atop the engine's deterministic core. The opportunity to define the standard for "Modern Excel Auditing on Mac" is open and defensible.

#### **Works cited**

1. My first Rust project: Xlsx-wasm-parser. A WebAssembly-wrapper around the Calamine crate to bring Blazingly Fast Excel deserialization to the Browser and NodeJS. \- Reddit, accessed November 23, 2025, [https://www.reddit.com/r/rust/comments/124y26y/my\_first\_rust\_project\_xlsxwasmparser\_a/](https://www.reddit.com/r/rust/comments/124y26y/my_first_rust_project_xlsxwasmparser_a/)  
2. Offline file comparison for Windows & Mac \- Diffchecker Desktop, accessed November 23, 2025, [https://www.diffchecker.com/desktop/](https://www.diffchecker.com/desktop/)  
3. Diffchecker: Compare text and find differences online or offline, accessed November 23, 2025, [https://www.diffchecker.com/](https://www.diffchecker.com/)  
4. Compare Excel & other spreadsheets online or offline \- Diffchecker, accessed November 23, 2025, [https://www.diffchecker.com/excel-compare/](https://www.diffchecker.com/excel-compare/)  
5. 11 Best File Comparison Software \- Pics.io, accessed November 23, 2025, [https://blog.pics.io/11-best-file-comparison-software/](https://blog.pics.io/11-best-file-comparison-software/)  
6. Pricing \- Diffchecker Desktop, accessed November 23, 2025, [https://www.diffchecker.com/pricing/](https://www.diffchecker.com/pricing/)  
7. Compare Sheetmatcher vs. WinMerge in 2025 \- Slashdot, accessed November 23, 2025, [https://slashdot.org/software/comparison/Sheetmatcher-vs-WinMerge/](https://slashdot.org/software/comparison/Sheetmatcher-vs-WinMerge/)  
8. py-xlsx-textconv \- PyPI, accessed November 23, 2025, [https://pypi.org/project/py-xlsx-textconv/](https://pypi.org/project/py-xlsx-textconv/)  
9. tokuhirom/git-xlsx-textconv \- GitHub, accessed November 23, 2025, [https://github.com/tokuhirom/git-xlsx-textconv](https://github.com/tokuhirom/git-xlsx-textconv)  
10. How do I create a readable diff of two spreadsheets using git diff? \- Stack Overflow, accessed November 23, 2025, [https://stackoverflow.com/questions/114698/how-do-i-create-a-readable-diff-of-two-spreadsheets-using-git-diff](https://stackoverflow.com/questions/114698/how-do-i-create-a-readable-diff-of-two-spreadsheets-using-git-diff)  
11. pandas.DataFrame.compare — pandas 2.3.3 documentation \- PyData |, accessed November 23, 2025, [https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.compare.html](https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.compare.html)  
12. Comparing excel data sets in Pandas \- Data Science Stack Exchange, accessed November 23, 2025, [https://datascience.stackexchange.com/questions/66710/comparing-excel-data-sets-in-pandas](https://datascience.stackexchange.com/questions/66710/comparing-excel-data-sets-in-pandas)  
13. How to write a Python script that reads two excel files and outputs distinct values, accessed November 23, 2025, [https://stackoverflow.com/questions/75605620/how-to-write-a-python-script-that-reads-two-excel-files-and-outputs-distinct-val](https://stackoverflow.com/questions/75605620/how-to-write-a-python-script-that-reads-two-excel-files-and-outputs-distinct-val)  
14. Comparison with spreadsheets — pandas 2.3.3 documentation \- PyData |, accessed November 23, 2025, [https://pandas.pydata.org/docs/getting\_started/comparison/comparison\_with\_spreadsheets.html](https://pandas.pydata.org/docs/getting_started/comparison/comparison_with_spreadsheets.html)  
15. Ablebits products \- buy online., accessed November 23, 2025, [https://www.ablebits.com/purchase/index.php](https://www.ablebits.com/purchase/index.php)  
16. 100+ professional tools for Excel, Outlook, and Google Sheets, accessed November 23, 2025, [https://www.ablebits.com/](https://www.ablebits.com/)  
17. File compare tool for Excel: compare two sheets and highlight differences \- Ablebits.com, accessed November 23, 2025, [https://www.ablebits.com/compare-excel-files/index.php](https://www.ablebits.com/compare-excel-files/index.php)  
18. How to compare two Excel sheets for differences \- OAK \- Operis Analysis Kit, accessed November 23, 2025, [https://www.operisanalysiskit.com/modelling-insights/how-to-compare-two-excel-sheets-for-differences/](https://www.operisanalysiskit.com/modelling-insights/how-to-compare-two-excel-sheets-for-differences/)  
19. OAK4-Professional.pdf \- Operis Analysis Kit, accessed November 23, 2025, [https://www.operisanalysiskit.com/wp-content/uploads/2018/09/OAK4-Professional.pdf](https://www.operisanalysiskit.com/wp-content/uploads/2018/09/OAK4-Professional.pdf)  
20. 19 Best Financial Modelling Tools (Businesses & Analysts) \- Gridlines, accessed November 23, 2025, [https://www.gridlines.com/blog/best-financial-modelling-tools/](https://www.gridlines.com/blog/best-financial-modelling-tools/)  
21. Financial Modelling Fees | Excel Add-In Package | OAK \- Operis Analysis Kit, accessed November 23, 2025, [https://www.operisanalysiskit.com/oak-price/](https://www.operisanalysiskit.com/oak-price/)  
22. OAK Software Support | Excel Financial Modelling Expert \- Operis Analysis Kit, accessed November 23, 2025, [https://www.operisanalysiskit.com/oak-support-3/](https://www.operisanalysiskit.com/oak-support-3/)  
23. Overview of Spreadsheet Compare \- Microsoft Support, accessed November 23, 2025, [https://support.microsoft.com/en-us/office/overview-of-spreadsheet-compare-13fafa61-62aa-451b-8674-242ce5f2c986](https://support.microsoft.com/en-us/office/overview-of-spreadsheet-compare-13fafa61-62aa-451b-8674-242ce5f2c986)  
24. Do you plan to make Spreadsheet Compare tool (Excel) available for Mac? \- Microsoft Learn, accessed November 23, 2025, [https://learn.microsoft.com/en-us/answers/questions/5181155/do-you-plan-to-make-spreadsheet-compare-tool-(exce](https://learn.microsoft.com/en-us/answers/questions/5181155/do-you-plan-to-make-spreadsheet-compare-tool-\(exce)  
25. Excel for Mac: Should You Buy a Mac? \- SumProduct, accessed November 23, 2025, [https://sumproduct.com/blog/excel-for-mac-should-you-buy-a-mac/](https://sumproduct.com/blog/excel-for-mac-should-you-buy-a-mac/)  
26. Basic tasks in Spreadsheet Compare \- Microsoft Support, accessed November 23, 2025, [https://support.microsoft.com/en-us/office/basic-tasks-in-spreadsheet-compare-f2b20af8-a6d3-4780-8011-f15b3229f5d8](https://support.microsoft.com/en-us/office/basic-tasks-in-spreadsheet-compare-f2b20af8-a6d3-4780-8011-f15b3229f5d8)  
27. Downloads & Support \- PerfectXL, accessed November 23, 2025, [https://www.perfectxl.com/resources/downloads-support/](https://www.perfectxl.com/resources/downloads-support/)  
28. PerfectXL Explore \- Free download and install on Windows \- Microsoft Store, accessed November 23, 2025, [https://apps.microsoft.com/detail/9pd5gfx4h7w8](https://apps.microsoft.com/detail/9pd5gfx4h7w8)  
29. PerfectXL Explore \- Walkthrough, accessed November 23, 2025, [https://www.perfectxl.com/resources/walkthroughs/perfectxl-explore/](https://www.perfectxl.com/resources/walkthroughs/perfectxl-explore/)  
30. Excel Add-in Installation & Requirements // PerfectXL, accessed November 23, 2025, [https://www.perfectxl.com/resources/downloads-support/installation-requirements-add-in/](https://www.perfectxl.com/resources/downloads-support/installation-requirements-add-in/)  
31. Version Control for Excel Spreadsheets \- Pricing \- xltrail, accessed November 23, 2025, [https://www.xltrail.com/pricing](https://www.xltrail.com/pricing)  
32. xltrail \- Version Control for Excel Spreadsheets \- xltrail is a version control system for Excel workbooks. It tracks changes, compares worksheets and VBA, and provides an audit trail for easy collaboration., accessed November 23, 2025, [https://www.xltrail.com/](https://www.xltrail.com/)  
33. Version Control for Excel Spreadsheets \- Git Integration \- xltrail, accessed November 23, 2025, [https://www.xltrail.com/integrations](https://www.xltrail.com/integrations)  
34. Version Control for Excel Spreadsheets \- Self-Hosted \- xltrail, accessed November 23, 2025, [https://www.xltrail.com/enterprise](https://www.xltrail.com/enterprise)  
35. Platform \- Schematiq, accessed November 23, 2025, [https://www.schematiq.com/platform](https://www.schematiq.com/platform)  
36. Compare GPTExcel vs. Schematiq in 2025 \- Slashdot, accessed November 23, 2025, [https://slashdot.org/software/comparison/GPT-Excel-vs-Schematiq/](https://slashdot.org/software/comparison/GPT-Excel-vs-Schematiq/)  
37. Equals | Top Marketing Analytics Tools & Services Directory, accessed November 23, 2025, [https://www.topanalyticstools.com/tools/equals/](https://www.topanalyticstools.com/tools/equals/)  
38. GRID — Spreadsheet Engine, accessed November 23, 2025, [https://grid.is/engine](https://grid.is/engine)  
39. Superior spreadsheet alternative to Excel/GSheets with advanced BI capabilities? \- Reddit, accessed November 23, 2025, [https://www.reddit.com/r/BusinessIntelligence/comments/z7s3qt/superior\_spreadsheet\_alternative\_to\_excelgsheets/](https://www.reddit.com/r/BusinessIntelligence/comments/z7s3qt/superior_spreadsheet_alternative_to_excelgsheets/)  
40. Use ChatGPT Code Interpreter To Analyze Spreadsheets \- CustomGPT.ai, accessed November 23, 2025, [https://customgpt.ai/use-chatgpt-code-interpreter-to-analyze-spreadsheets/](https://customgpt.ai/use-chatgpt-code-interpreter-to-analyze-spreadsheets/)  
41. Analyzing Humanitarian Data Unstructured Excel Tables with ChatGPT Code Interpreter, accessed November 23, 2025, [https://medium.com/data-science/analyzing-humanitarian-data-unstructured-excel-tables-with-chatgpt-code-interpreter-ccb8cabfa146](https://medium.com/data-science/analyzing-humanitarian-data-unstructured-excel-tables-with-chatgpt-code-interpreter-ccb8cabfa146)  
42. Compare your files with Copilot in OneDrive \- Microsoft Support, accessed November 23, 2025, [https://support.microsoft.com/en-us/office/compare-your-files-with-copilot-in-onedrive-794ab09c-c64a-45f3-bcbe-110a182a1806](https://support.microsoft.com/en-us/office/compare-your-files-with-copilot-in-onedrive-794ab09c-c64a-45f3-bcbe-110a182a1806)  
43. ALM Toolkit \- SQLBI, accessed November 23, 2025, [https://www.sqlbi.com/tools/alm-toolkit/](https://www.sqlbi.com/tools/alm-toolkit/)
```

---

### File: `docs\competitor_profiles\Excel Diff Tool Licensing Scheme Design.md`

```markdown


# **Secure Licensing Architectures for Privacy-Sensitive B2B Software: A Comprehensive Analysis**

## **1\. The Strategic Paradox of Modern B2B Licensing**

The architectural landscape for Independent Software Vendors (ISVs) targeting regulated industries sits at a complex intersection of security, convenience, and compliance. For developers of desktop and Command Line Interface (CLI) tools—particularly those handling sensitive financial data, such as an Excel difference engine—the licensing mechanism is not merely a revenue protection feature; it is a critical component of the user experience and a potential barrier to entry for enterprise procurement. The paradox facing modern ISVs is clear: while the broader software economy has moved toward "always-online" SaaS models that rely on continuous connectivity for entitlement verification, the target demographic for high-value data tools—finance professionals, auditors, and data analysts—often operates within "zero-trust," air-gapped, or strictly firewalled environments where such connectivity is technically impossible or contractually forbidden.1

The shift toward Merchant of Record (MoR) platforms like Lemon Squeezy and Paddle has democratized global software sales by abstracting the complexities of tax compliance and fraud detection.3 However, these platforms primarily optimize for the connected user. Their native licensing solutions often assume a "phone-home" validation capability that conflicts directly with the operational realities of regulated entities, where data egress is treated as a security incident.5 Consequently, the implementation of a licensing system for a privacy-sensitive tool requires a hybrid approach: one that leverages the commercial efficiency of MoRs for transaction processing while decoupling entitlement verification into a robust, offline-first cryptographic protocol.

This report provides an exhaustive analysis of the licensing ecosystem, evaluating the trade-offs between "Buy" (MoR native tools) and "Build" (custom cryptographic implementations) strategies. It proposes a definitive architecture centered on Ed25519 asymmetric cryptography to facilitate offline activation without sacrificing revenue assurance. Furthermore, it examines the specific User Experience (UX) patterns required for CLI tools in automated pipelines and details a "Trust but Verify" compliance model suitable for large-scale enterprise deployments.

---

## **2\. Regulatory Constraints and the "Offline" Requirement**

Understanding the operational environment of the target customer is a prerequisite for architectural design. In the financial services sector, software does not merely need to function; it must comply with rigorous governance frameworks that dictate how external code interacts with internal data.

### **2.1 The "No Data Egress" Mandate**

For tools processing financial models or audit trails, the primary security concern is data exfiltration. Financial institutions operate under strict regulations such as DORA (Digital Operational Resilience Act) and various internal "Secure by Design" pledges that mandate the minimization of external connectivity.2 A standard "phone-home" licensing check, which transmits a payload containing metadata (IP address, machine hostname, username) to a third-party server, can trigger Data Loss Prevention (DLP) alerts. If an Excel diff engine attempts to open an HTTPS connection to api.lemonsqueezy.com while a sensitive spreadsheet is open in memory, endpoint detection systems may flag the behavior as malware-like or a policy violation, potentially resulting in the software being blacklisted across the organization.5

The implications for licensing are profound: the mechanism must be designed such that validation is a local, read-only operation. The software must function deterministically without ever requiring an outbound packet after the initial installation. This necessitates a shift from *server-side authority* (where the server says "Yes, you are licensed") to *server-side issuance with client-side verification* (where the server issues a signed credential that the client can independently verify).5

### **2.2 Air-Gapped and Zero-Trust Environments**

The concept of the "air gap"—a physical isolation of secure networks from the public internet—remains a standard in high-security finance and government sectors. While cloud adoption is growing, critical infrastructure often resides in private clouds or on-premises servers that have no route to the internet.2 In these scenarios, a licensing system that relies on a "grace period" or "occasional check-in" is fundamentally broken. A grace period of 30 days is irrelevant if the machine will never connect to the internet during its lifecycle.

Furthermore, within "zero-trust" networks, even connected machines may face strict egress filtering. Firewalls are often configured to block all traffic by default, whitelisting only essential business services. Getting a new domain whitelisted for a licensing check is a bureaucratic hurdle that can delay software deployment by months. Therefore, the licensing architecture must support a completely offline lifecycle: from activation to renewal and eventual deactivation, relying on manual file transfer protocols (sneakernet) rather than direct API calls.2

### **2.3 Auditability and Software Asset Management (SAM)**

Unlike individual consumers, enterprise buyers are subject to software audits. During the "Great Recession," software vendors aggressively utilized noncompliance audits as a revenue generation tool, a practice that has made modern enterprises extremely sensitive to license tracking.1 IT administrators use Software Asset Management (SAM) tools and Group Policy Objects (GPO) to manage deployments. A licensing system for B2B tools must therefore be compatible with automated deployment strategies (e.g., SCCM, Intune) and capable of generating local usage reports that can be ingested by SAM tools to prove compliance during an audit.10

---

## **3\. Landscape Analysis: MoR vs. Dedicated Licensing Managers**

To implement a system that satisfies these constraints, we must evaluate the capabilities of existing platforms. The market is bifurcated into Merchants of Record (MoR) which handle payments and basic licensing, and dedicated License Managers (DLM) which focus solely on entitlement logic.

### **3.1 Merchant of Record (MoR) Capabilities**

Platforms like Lemon Squeezy and Paddle have become the standard for indie B2B sales due to their handling of global sales tax and invoicing. However, their native licensing capabilities often lack the nuance required for regulated offline environments.

#### **3.1.1 Lemon Squeezy**

Lemon Squeezy offers a native license key system integrated directly into their product definition. When a user purchases a subscription, a key is generated and emailed to them.12

* **Mechanism:** The system relies on an API endpoint (POST /v1/licenses/activate) to bind a key to a specific "instance" (device). The response includes the license status (active, expired, disabled).12  
* **Offline Limitations:** While Lemon Squeezy supports "license keys," the validation logic is inherently online. The API documentation emphasizes activation via HTTP requests, which presupposes connectivity. There is no native support for generating cryptographically signed "offline files" that can be transferred to a disconnected machine for validation.3  
* **Operational Risks:** Recent structural changes following the acquisition by Stripe have introduced significant friction for desktop app developers. Reports indicate that "Live Mode" approval can be delayed for weeks, blocking the ability to issue real licenses.4 Furthermore, the platform's focus is shifting heavily toward SaaS (web apps), potentially deprecating or stagnating features required for desktop software.3

#### **3.1.2 Paddle (Billing and Classic)**

Paddle has transitioned between "Paddle Classic" (which had robust software licensing SDKs) and "Paddle Billing" (which is purely a payments API).

* **Mechanism:** Paddle Billing does not generate license keys natively. It relies on webhooks. When a purchase occurs, Paddle sends a transaction.completed webhook to the vendor's server. The vendor must then generate the license and email it to the customer.15  
* **Implications:** This forces the developer to build or buy a separate licensing backend. While this adds complexity, it offers greater architectural freedom compared to Lemon Squeezy's walled garden. The developer can implement any cryptographic scheme they desire since they control the key generation logic triggered by the webhook.17

### **3.2 Dedicated License Managers (DLM)**

Services like Keygen.sh and LicenseSpring are designed to bridge the gap between payments and complex entitlement enforcement.

#### **3.2.1 Keygen.sh**

Keygen operates as a headless licensing API that integrates with MoRs via webhooks.

* **Offline Support:** Keygen explicitly supports air-gapped environments through "License Files." It allows the server to cryptographically sign a machine-specific policy, which can be validated locally using a public key embedded in the software.19  
* **Cryptography:** It supports modern algorithms like Ed25519 for signing, ensuring that license keys are short enough to be manageable while remaining secure.20  
* **Container Support:** Keygen has specific features for "floating" licenses in ephemeral environments like Docker containers, which is critical for the CLI aspect of the proposed tool.20

#### **3.2.2 LicenseSpring**

LicenseSpring offers a similar feature set but with a stronger focus on enterprise workflows, including a dedicated "Offline Portal" that vendors can expose to their customers.22

* **Mechanism:** It uses a challenge-response protocol involving request files and response files, which matches the standard "sneakernet" workflow used in secure facilities.24

### **3.3 Comparative Architecture Matrix**

The following table contrasts the capabilities of these approaches regarding the specific constraints of the financial B2B market.

| Feature | Lemon Squeezy Native | Paddle \+ Custom Backend | Keygen / LicenseSpring |
| :---- | :---- | :---- | :---- |
| **Payment & Tax** | Native (MoR) | Native (MoR) | Integration Required |
| **Connectivity** | Online Required | Developer Defined | Offline / Air-Gap Ready |
| **Key Generation** | Opaque (UUID) | Developer Defined | Cryptographic (Signed) |
| **Revocation** | Instant (via API) | Custom Logic | Expiry / CRL Based |
| **Policy Engine** | Basic (Seat Limit) | Custom Logic | Advanced (Float, Node-Lock) |
| **Privacy / Egress** | Requires Egress | Developer Defined | Zero Egress Possible |
| **Implementation** | Low Effort | High Effort | Medium Effort |

**Strategic Insight:** For a privacy-sensitive tool, relying solely on Lemon Squeezy's native licensing is a strategic error. The requirement for API connectivity creates a dependency chain that will fail in target customer environments. The robust path is to utilize an MoR (Paddle or Lemon Squeezy) strictly for payments, utilizing webhooks to trigger a dedicated licensing engine (like Keygen) or a custom cryptographic signer to issue offline-capable credentials.

---

## **4\. Technical Architecture: The Cryptographic Offline Model**

To satisfy the "No Data Egress" requirement while protecting against piracy, the recommended architecture utilizes **Asymmetric Cryptography** (Public-Key Cryptography). This model shifts the "source of truth" from a central database to the license key itself.

### **4.1 The Cryptographic Foundation: Ed25519 vs. RSA**

In this model, the license key is not a random string but a structured data payload signed by the vendor's private key. The client software contains the corresponding public key to verify integrity.

#### **4.1.1 Algorithm Selection**

Historically, RSA (Rivest–Shamir–Adleman) has been the standard for signing. However, RSA keys are large. A 2048-bit RSA signature is 256 bytes, which, when Base64 encoded, results in a cumbersome string. For a desktop/CLI tool where users might manually copy-paste keys, brevity is a usability feature.26

**Ed25519 (Edwards-curve Digital Signature Algorithm)** is the superior choice for modern licensing systems.

* **Efficiency:** It offers high security with a 32-byte public key and a 64-byte signature.26  
* **Performance:** Verification is extremely fast and constant-time, preventing timing attacks.  
* **Usability:** The resulting encoded keys are significantly shorter than RSA counterparts, reducing errors in manual entry or environment variable configuration.27  
* **Availability:** Libraries for Ed25519 are available in all major languages (Python pynacl, Go crypto/ed25519, Node.js tweetnacl), simplifying the build process for both the backend signer and the client verifier.

### **4.2 The License Payload Structure**

The license key acts as a "bearer token" containing all necessary entitlements. The payload is typically a JSON object that is serialized, signed, and then encoded.

**Proposed Payload Schema:**

JSON

{  
  "v": 1,                       // Schema version  
  "sub": "cust\_88a9f...",       // Customer ID (Subject)  
  "iss": "https://api.vendor",  // Issuer  
  "iat": 1715000000,            // Issued At (Unix Timestamp)  
  "exp": 1746536000,            // Expiry Date (Unix Timestamp)  
  "entitlements": {             // Feature Flags  
    "cli": true,  
    "desktop": true,  
    "offline\_days": 365         // Max offline duration  
  },  
  "constraints": {  
    "hwid": "sha256:8f2a..."    // Hardware ID (if node-locked)  
  }  
}

The final license key string presented to the user is composed of:  
Base64(Payload) \+ "." \+ Base64(Signature)  
This structure is similar to a JSON Web Token (JWT) but typically uses a custom, denser packing or standard JWT libraries if library size is not a concern. The critical aspect is that the software can read the exp (expiry) and hwid (hardware ID) fields locally. If the signature matches the payload and the public key, the data is trusted. If a user attempts to change "exp" to a future date, the signature verification will fail.20

### **4.3 Node-Locking in Air-Gapped Environments**

Binding a license to a specific machine (Node-Locking) without internet access requires a **Challenge-Response Protocol**. This is the industry standard for high-security environments.9

#### **4.3.1 Step 1: Fingerprint Generation (The Challenge)**

The user installs the application on the air-gapped machine. Upon first run, the application generates a "Machine Fingerprint."

* **Best Practices for Fingerprinting:** Avoid relying solely on MAC addresses, which can change with USB dongles or VPNs. A robust fingerprint combines:  
  * OS Serial Number / Machine GUID (Windows Registry MachineGuid, macOS IOPlatformUUID).  
  * CPU Processor ID.  
  * Motherboard Serial Number.  
  * *Note:* In virtualized environments (CI/CD), these values can be ephemeral. The logic must detect virtual machines and potentially relax the fingerprint stringency or rely on a "Floating" license model.20

The application exports this fingerprint into a generic "Request File" (e.g., activation\_request.req).

#### **4.3.2 Step 2: The Transfer (Sneakernet)**

The user saves activation\_request.req to a removable storage device (USB drive). In extremely secure environments where USBs are blocked, the fingerprint might be a short alphanumeric string (e.g., encoded via Bech32 for readability) that the user can manually type into a portal on a different device.22

#### **4.3.3 Step 3: Portal Activation (The Response)**

The user navigates to the vendor's self-service licensing portal on a connected device (e.g., a smartphone or non-secure workstation).

* **Action:** User logs in, uploads the activation\_request.req, or types the fingerprint.  
* **Backend Logic:** The server validates the user's subscription status. If active, it generates the JSON payload described in 4.2, inserts the uploaded hwid into the constraints, signs it, and returns the signed license string (or a license.dat file).23

#### **4.3.4 Step 4: Installation**

The user transfers the license.dat file back to the air-gapped machine. The application reads the file, verifies the signature, matches the hwid in the payload to the local machine, and unlocks.

### **4.4 Handling Time and Clock Tampering**

A major vulnerability in offline licensing is the system clock. A malicious user could set their clock back to 2024 to keep using an expired license.

**Mitigation Strategies:**

* **File System Timestamp Analysis:** The application should check the modification times of critical system files (e.g., OS logs, registry hives). If SystemTime \< LastModifiedTime(SystemLogs), it indicates the clock has been rolled back.31  
* **Monotonic Clock Anchoring:** When the application runs, it should record the current time in an encrypted local store. On subsequent runs, if the system time is earlier than the last recorded time, usage is blocked.  
* **Tolerance:** To avoid false positives (e.g., travel across time zones), allow a "grace window" of 24–48 hours before flagging a clock error.32

---

## **5\. User Experience: CLI and Desktop Interaction Design**

The target audience—analysts and developers—values efficiency and transparency. The UX of the licensing system must not obstruct their workflow, particularly in command-line environments.

### **5.1 CLI Interaction Patterns**

Command Line Interfaces have unique constraints. They are often run in "headless" modes (scripts, CI pipelines) where interactive prompts cause hangs.

#### **5.1.1 Environment Variables vs. Config Files**

The CLI should support a hierarchy of license discovery methods, following the "Twelve-Factor App" methodology for configuration 33:

1. **Environment Variable (EXCELDIFF\_LICENSE):** Highest priority. This is essential for CI/CD systems where config files are hard to inject. The value can be the raw signed key string.  
2. **CLI Flag (--license-key):** Useful for one-off testing.  
3. **Local Config File:** The CLI should look in standard paths for a persistent license file generated by the Desktop app or placed manually.  
   * **Linux:** \~/.config/exceldiff/license.lic  
   * **macOS:** \~/Library/Application Support/ExcelDiff/license.lic  
   * **Windows:** %APPDATA%\\ExcelDiff\\license.lic.35

#### **5.1.2 Output Stream Hygiene**

A critical UX failure in licensed CLIs is "stdout pollution." If the tool prints "License Verified\!" to Standard Output (stdout), it breaks pipelining.

* **Bad Pattern:** exceldiff a.xlsx b.xlsx \> diff.csv results in a CSV file that starts with "License Verified\!".  
* **Good Pattern:** All license status messages, warnings, and errors must be printed to **Standard Error (stderr)**. Only the actual data output goes to stdout. This ensures that downstream tools (like jq, sed, or database importers) receive clean data regardless of the license state.37

#### **5.1.3 Exit Codes**

The CLI should use specific exit codes to distinguish between "Difference Found" (often exit code 1 in diff tools), "Runtime Error," and "License Error."

* **Recommendation:** Use a reserved exit code (e.g., 101\) for "License Invalid/Expired." This allows wrapper scripts to detect a license failure programmatically and trigger a re-activation workflow or alert an admin.37

### **5.2 Desktop Application UX**

The Desktop GUI serves as the "Hub" for license management.

#### **5.2.1 The "Unlicensed" State**

Avoid a "brick wall" where the app closes immediately if unlicensed. This frustrates users who might just need to view a file or export a small snippet.

* **Soft Enforcement:** Allow the app to open in "Viewer Mode" (Read-Only). Disable "Save," "Export," and "Copy to Clipboard."  
* **Watermarking:** For the diff engine, allow the export but inject random "UNLICENSED" watermarks into the cell values or headers. This renders the output commercially unusable while demonstrating the tool's value.20

#### **5.2.2 Deactivation and Machine Replacement**

Users inevitably replace computers. In an offline system, the vendor cannot "reach in" and deactivate the old machine.

* **User-Initiated Transfer:** The Desktop app should have a "Deactivate" button.  
  * *Online Mode:* Sends a request to the server to free the seat.  
  * *Offline Mode:* Generates a "Deactivation Code" (a signed proof that the local license file has been deleted/invalidated). The user enters this code in the web portal to free up a seat.23  
* **Portal Force-Kill:** Allow the user to "Force Deactivate" a machine from the web portal if the laptop is lost or stolen. Limit this to a strict number (e.g., 2 per year) to prevent abuse where users rotate one license across 100 machines.22

---

## **6\. Enterprise Distribution and the "Trust" Model**

While the cryptographic model works well for individuals and small teams, it scales poorly for large enterprises deploying to 5,000 workstations. Managing 5,000 unique license files is a logistical nightmare for IT departments.

### **6.1 The "Org-Level" License Key**

For large B2B contracts, the correct approach is "Identity-Based Licensing" or a "Site License."

#### **6.1.1 Domain Locking**

Instead of locking to a hardware fingerprint, the license is locked to an **Active Directory Domain** or **Email Domain**.

* **Mechanism:** The signed license payload contains {"constraint": "domain:bankofamerica.com"}.  
* **Validation:**  
  * On startup, the app queries the OS for the joined domain or the current user's UPN (User Principal Name).  
  * If the user is john.doe@bankofamerica.com, the license validates.  
* **Deployment:** The IT admin deploys this single license string to all 5,000 machines via Group Policy (GPO) or MDM (Mobile Device Management) by setting a registry key (HKCU\\Software\\ExcelDiff\\License) or an environment variable.10

### **6.2 The "Trust but Verify" Enforcement**

In this model, technical prevention of over-usage is replaced by legal and audit-based controls.

* **The Logic:** Large regulated banks are not casual pirates. They have strict compliance departments. They prefer to pay for "True-Ups" rather than have critical software stop working during a crisis.  
* **Soft Limits:** The software does not stop working if 5,001 users install it.  
* **Local Auditing:** The software maintains a secure local log of usage.  
* **The True-Up Clause:** The contract includes a "True-Up" provision. At the end of the year, the customer declares their usage count. If it exceeds the pre-paid amount, they pay the difference (often without penalty).40  
* **Remote Auditing (Optional):** The software can include a feature to "Generate Usage Report," creating an encrypted file that the internal IT admin can email to the vendor. This respects the air-gap while providing transparency.43

---

## **7\. Implementation Roadmap and Migration Strategies**

Designing this system requires a phased rollout to manage complexity and risk.

### **7.1 Phase 1: MoR Integration \+ Keygen (The Hybrid Stack)**

Do not build the licensing engine from scratch. The complexity of correctly implementing Ed25519 signatures, handling clock skew, and managing machine state is high.

* **Stack:**  
  * **Payments:** Lemon Squeezy (or Paddle) acts as the MoR.  
  * **Licensing:** Keygen.sh acts as the issuance authority.  
* **Workflow:**  
  1. User buys on Lemon Squeezy.  
  2. Lemon Squeezy Webhook \-\> Middleware (Zapier/Serverless Function).  
  3. Middleware calls Keygen API \-\> Creates License \-\> Returns Key.  
  4. Middleware emails Key to User.  
* **Why:** Keygen natively supports Ed25519 signed keys and has pre-built libraries for validation, saving months of dev time.19

### **7.2 Phase 2: The Offline Portal**

Once the base system is live, build the "Self-Service Offline Portal."

* **Function:** A simple web app where users upload the request.req file and download the license.dat.  
* **Integration:** This portal interfaces with the Keygen API to register the offline machine and retrieve the signed machine-specific token.22

### **7.3 Phase 3: Enterprise Features**

Add the "Domain Locking" logic to the desktop client. This requires interacting with Windows APIs (NetGetJoinInformation) and macOS APIs (ODNode) to securely determine the machine's domain status. This allows you to sell the "Enterprise Tier" with the Frictionless Deployment value proposition.39

---

## **8\. Legal Frameworks & Terms of Use**

The technical architecture must be backed by a robust legal framework. The Terms of Use (ToS) should explicitly address the unique aspects of offline and audit-based licensing.

### **8.1 Terms of Use Summary**

* **License Grant:** Clearly distinguish between "Node-Locked" (per device) and "Site" (per organization) licenses.  
* **Offline Activation:** Explicitly state that for offline environments, the user is responsible for the secure transfer of license files and must not manipulate the machine identity data.7  
* **Audit Rights:** Include a clause granting the right to request usage reports.  
  * *Sample Verbiage:* "Licensor reserves the right, no more than once every twelve (12) months, to request a certification of the number of Installed Copies. In the event usage exceeds the Licensed Quantity, Licensee agrees to pay the applicable fees for the excess usage ('True-Up') within 30 days.".45  
* **Data Privacy:** A critical selling point. The ToS should explicitly state: "The Software performs license validation locally. No User Content, financial data, or file metadata is ever transmitted to Licensor or any third party.".47

### **8.2 Merchant of Record Liability**

When using an MoR like Lemon Squeezy, they are the "Seller of Record." Ensure that the End User License Agreement (EULA) is clear that while the MoR handles the transaction, the *intellectual property rights* and *license terms* are between the Vendor and the User. This separation protects the vendor while satisfying the MoR's compliance requirements.3

---

## **9\. Conclusion**

Designing a licensing system for privacy-sensitive B2B tools requires rejecting the default "always-online" assumptions of the modern web. By adopting a **Cryptographic Offline-First Architecture**, utilizing **Ed25519 signatures**, and implementing a **Challenge-Response protocol** for air-gapped machines, vendors can meet the rigorous security demands of the financial sector.

Simultaneously, recognizing that "Piracy" looks different in the enterprise—where it is often a bureaucratic accident rather than malicious theft—allows for the adoption of **"Trust but Verify"** models. This approach removes friction from the sales process, allowing large institutions to deploy the software via standard automation tools while ensuring revenue integrity through contract law rather than DRM code. This balanced strategy secures both the intellectual property and the customer trust necessary to succeed in the regulated software market.

#### **Works cited**

1. Key Readiness Tactics for a Software Audit, Part Two: Contractual Strategies to Mitigate Risk, accessed November 23, 2025, [https://www.americanbar.org/groups/business\_law/resources/business-law-today/2023-january/key-readiness-tactics-for-software-audit/](https://www.americanbar.org/groups/business_law/resources/business-law-today/2023-january/key-readiness-tactics-for-software-audit/)  
2. Locked down by design: Air-gapped and threat- adaptive security are the next inflection points for private cloud | HPE, accessed November 23, 2025, [https://www.hpe.com/us/en/newsroom/blog-post/2025/04/locked-down-by-design-air-gapped-is-the-next-inflection-point-for-private-cloud.html](https://www.hpe.com/us/en/newsroom/blog-post/2025/04/locked-down-by-design-air-gapped-is-the-next-inflection-point-for-private-cloud.html)  
3. Docs: Lemon Squeezy Licensing, accessed November 23, 2025, [https://docs.lemonsqueezy.com/help/licensing](https://docs.lemonsqueezy.com/help/licensing)  
4. LemonSqueezy won't activate Live mode — any good alternatives for desktop app licensing? : r/SaaS \- Reddit, accessed November 23, 2025, [https://www.reddit.com/r/SaaS/comments/1obfdz2/lemonsqueezy\_wont\_activate\_live\_mode\_any\_good/](https://www.reddit.com/r/SaaS/comments/1obfdz2/lemonsqueezy_wont_activate_live_mode_any_good/)  
5. Securing OT Systems: The Limits of the Air Gap Approach \- Darktrace, accessed November 23, 2025, [https://www.darktrace.com/blog/why-the-air-gap-is-not-enough](https://www.darktrace.com/blog/why-the-air-gap-is-not-enough)  
6. Service Privacy Policy | Legal \- Egress, accessed November 23, 2025, [https://www.egress.com/legal/privacy-policy](https://www.egress.com/legal/privacy-policy)  
7. Absolute Guide to Software Licensing Types | Licensing Models \- Thales, accessed November 23, 2025, [https://cpl.thalesgroup.com/software-monetization/software-licensing-models-guide](https://cpl.thalesgroup.com/software-monetization/software-licensing-models-guide)  
8. Enterprise Compliance: Avatier vs Okta Regulatory Complexity, accessed November 23, 2025, [https://www.avatier.com/blog/compliance-avatier-vs-okta/](https://www.avatier.com/blog/compliance-avatier-vs-okta/)  
9. Offline License Update \- Disguise User Guide, accessed November 23, 2025, [https://help.disguise.one/designer/getting-started/offline-license-update](https://help.disguise.one/designer/getting-started/offline-license-update)  
10. Use Group Policy to remotely install software \- Windows Server | Microsoft Learn, accessed November 23, 2025, [https://learn.microsoft.com/en-us/troubleshoot/windows-server/group-policy/use-group-policy-to-install-software](https://learn.microsoft.com/en-us/troubleshoot/windows-server/group-policy/use-group-policy-to-install-software)  
11. Software Deployment Tools: SCCM vs Intune vs GPO vs More \- Netwrix, accessed November 23, 2025, [https://netwrix.com/en/resources/blog/software-deployment-tools-sccm-vs-intune-vs-gpo-vs-more//](https://netwrix.com/en/resources/blog/software-deployment-tools-sccm-vs-intune-vs-gpo-vs-more//)  
12. Guides: Validating License Keys With the License API • Lemon ..., accessed November 23, 2025, [https://docs.lemonsqueezy.com/guides/tutorials/license-keys](https://docs.lemonsqueezy.com/guides/tutorials/license-keys)  
13. Activate a License Key \- API Docs \- Lemon Squeezy, accessed November 23, 2025, [https://docs.lemonsqueezy.com/api/license-api/activate-license-key](https://docs.lemonsqueezy.com/api/license-api/activate-license-key)  
14. Docs: Generating License Keys \- Lemon Squeezy, accessed November 23, 2025, [https://docs.lemonsqueezy.com/help/licensing/generating-license-keys](https://docs.lemonsqueezy.com/help/licensing/generating-license-keys)  
15. Verify webhook signatures \- Paddle Developer, accessed November 23, 2025, [https://developer.paddle.com/webhooks/signature-verification](https://developer.paddle.com/webhooks/signature-verification)  
16. Selling Outside of the Mac App Store, Part II: Let's Meddle with Paddle, accessed November 23, 2025, [https://blog.eternalstorms.at/2024/12/18/selling-outside-of-the-mac-app-store-part-ii-lets-meddle-with-paddle/](https://blog.eternalstorms.at/2024/12/18/selling-outside-of-the-mac-app-store-part-ii-lets-meddle-with-paddle/)  
17. An example Node.js app that integrates Keygen with Paddle for accepting payments. \- GitHub, accessed November 23, 2025, [https://github.com/keygen-sh/example-paddle-integration](https://github.com/keygen-sh/example-paddle-integration)  
18. How to Test and Replay Paddle Billing Webhooks Events on localhost with Hookdeck, accessed November 23, 2025, [https://hookdeck.com/webhooks/platforms/how-to-test-and-replay-paddle-webhooks-events-on-localhost-with-hookdeck](https://hookdeck.com/webhooks/platforms/how-to-test-and-replay-paddle-webhooks-events-on-localhost-with-hookdeck)  
19. Offline licensing \- API Reference \- Documentation \- Keygen, accessed November 23, 2025, [https://keygen.sh/docs/api/cryptography/](https://keygen.sh/docs/api/cryptography/)  
20. How to Implement an Offline Licensing Model \- Keygen, accessed November 23, 2025, [https://keygen.sh/docs/choosing-a-licensing-model/offline-licenses/](https://keygen.sh/docs/choosing-a-licensing-model/offline-licenses/)  
21. How to Implement a Floating Licensing Model \- Keygen, accessed November 23, 2025, [https://keygen.sh/docs/choosing-a-licensing-model/floating-licenses/](https://keygen.sh/docs/choosing-a-licensing-model/floating-licenses/)  
22. Implementing Offline Licensing \- LicenseSpring, accessed November 23, 2025, [https://licensespring.com/blog/tutorials/offline-licensing](https://licensespring.com/blog/tutorials/offline-licensing)  
23. Node-Lock licenses to an offline device with LicenseSpring \- YouTube, accessed November 23, 2025, [https://www.youtube.com/watch?v=mN4KiwQbNrI](https://www.youtube.com/watch?v=mN4KiwQbNrI)  
24. Self-hosted Open-source license server recommendations : r/selfhosted \- Reddit, accessed November 23, 2025, [https://www.reddit.com/r/selfhosted/comments/1ok2lob/selfhosted\_opensource\_license\_server/](https://www.reddit.com/r/selfhosted/comments/1ok2lob/selfhosted_opensource_license_server/)  
25. LicenseSpring Alternatives, accessed November 23, 2025, [https://licensespring.com/blog/other/licensespring-alternatives](https://licensespring.com/blog/other/licensespring-alternatives)  
26. SSH Key Best Practices for 2025 \- Using ed25519, key rotation, and other best practices, accessed November 23, 2025, [https://www.brandonchecketts.com/archives/ssh-ed25519-key-best-practices-for-2025](https://www.brandonchecketts.com/archives/ssh-ed25519-key-best-practices-for-2025)  
27. What is the best practices for storing ed25519 private keys which is used by nodes of my ... \- Cryptography Stack Exchange, accessed November 23, 2025, [https://crypto.stackexchange.com/questions/39343/what-is-the-best-practices-for-storing-ed25519-private-keys-which-is-used-by-nod](https://crypto.stackexchange.com/questions/39343/what-is-the-best-practices-for-storing-ed25519-private-keys-which-is-used-by-nod)  
28. An abridged guide to using ed25519 PGP keys with GnuPG and SSH | MuSigma, accessed November 23, 2025, [https://musigma.blog/2021/05/09/gpg-ssh-ed25519.html](https://musigma.blog/2021/05/09/gpg-ssh-ed25519.html)  
29. Offline License Processing \- Nexus by Hexagon, accessed November 23, 2025, [https://nexus.hexagon.com/documentationcenter/en-US/bundle/pcdmis-2025.1-clm/page/Offline\_License\_Processing.htm](https://nexus.hexagon.com/documentationcenter/en-US/bundle/pcdmis-2025.1-clm/page/Offline_License_Processing.htm)  
30. Licenses for Offline Devices \- Wibu-Systems, accessed November 23, 2025, [https://www.wibu.com/magazine/keynote-articles/article/detail/licenses-for-offline-devices.html](https://www.wibu.com/magazine/keynote-articles/article/detail/licenses-for-offline-devices.html)  
31. Best Practices for Managing Offline Activations with Cryptlex \- General, accessed November 23, 2025, [https://forums.cryptlex.com/t/best-practices-for-managing-offline-activations-with-cryptlex/2037](https://forums.cryptlex.com/t/best-practices-for-managing-offline-activations-with-cryptlex/2037)  
32. How does the keygen-sh offline-license-check validate the expiration of the license?, accessed November 23, 2025, [https://stackoverflow.com/questions/65621015/how-does-the-keygen-sh-offline-license-check-validate-the-expiration-of-the-lice](https://stackoverflow.com/questions/65621015/how-does-the-keygen-sh-offline-license-check-validate-the-expiration-of-the-lice)  
33. CLI to set variable of env file \- Stack Overflow, accessed November 23, 2025, [https://stackoverflow.com/questions/43442713/cli-to-set-variable-of-env-file](https://stackoverflow.com/questions/43442713/cli-to-set-variable-of-env-file)  
34. It's time to deprecate the .env file \- Medium, accessed November 23, 2025, [https://medium.com/@tony.infisical/its-time-to-deprecate-the-env-file-for-a-better-stack-a519ac89bab0](https://medium.com/@tony.infisical/its-time-to-deprecate-the-env-file-for-a-better-stack-a519ac89bab0)  
35. License file path / location on specific OS | Graebert GmbH Help Center, accessed November 23, 2025, [https://help.graebert.com/en/articles/4241132-license-file-path-location-on-specific-os](https://help.graebert.com/en/articles/4241132-license-file-path-location-on-specific-os)  
36. Naming Files, Paths, and Namespaces \- Win32 apps | Microsoft Learn, accessed November 23, 2025, [https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)  
37. UX patterns for CLI tools \- Lucas F. Costa, accessed November 23, 2025, [https://lucasfcosta.com/blog/ux-patterns-cli-tools](https://lucasfcosta.com/blog/ux-patterns-cli-tools)  
38. UX patterns for CLI tools : r/programming \- Reddit, accessed November 23, 2025, [https://www.reddit.com/r/programming/comments/1815fug/ux\_patterns\_for\_cli\_tools/](https://www.reddit.com/r/programming/comments/1815fug/ux_patterns_for_cli_tools/)  
39. Deploy the license key for users via Group Policy, accessed November 23, 2025, [https://cdm.iamcloud.info/docs/Content/Deployment/MassDeployment\_LicKeyDeploymentGP.htm](https://cdm.iamcloud.info/docs/Content/Deployment/MassDeployment_LicKeyDeploymentGP.htm)  
40. True-up: Overview, definition, and example \- Cobrief, accessed November 23, 2025, [https://www.cobrief.app/resources/legal-glossary/true-up-overview-definition-and-example/](https://www.cobrief.app/resources/legal-glossary/true-up-overview-definition-and-example/)  
41. True-Up Clause Samples | Law Insider, accessed November 23, 2025, [https://www.lawinsider.com/clause/true-up](https://www.lawinsider.com/clause/true-up)  
42. True-Up Clause in Software Agreements: Time to Take Control, accessed November 23, 2025, [https://openit.com/taking-control-of-your-true-up-software-agreements/](https://openit.com/taking-control-of-your-true-up-software-agreements/)  
43. Sample License Audit Provisions \- Association of Corporate Counsel, accessed November 23, 2025, [https://www.acc.com/sites/default/files/resources/vl/membersonly/SampleFormPolicy/14054\_1.pdf](https://www.acc.com/sites/default/files/resources/vl/membersonly/SampleFormPolicy/14054_1.pdf)  
44. Deploy Windows Enterprise licenses \- Microsoft Learn, accessed November 23, 2025, [https://learn.microsoft.com/en-us/windows/deployment/deploy-enterprise-licenses](https://learn.microsoft.com/en-us/windows/deployment/deploy-enterprise-licenses)  
45. Right To Audit Clause Guide: Examples, Gotcha's & More \- Gavel, accessed November 23, 2025, [https://www.gavel.io/legal-clause/right-to-audit-clause](https://www.gavel.io/legal-clause/right-to-audit-clause)  
46. Software Audit Sample Clauses \- Law Insider, accessed November 23, 2025, [https://www.lawinsider.com/clause/software-audit](https://www.lawinsider.com/clause/software-audit)  
47. Privacy Policy \- First Financial, accessed November 23, 2025, [https://ofsl.onlinebank.com/PrivacyPolicy.aspx](https://ofsl.onlinebank.com/PrivacyPolicy.aspx)  
48. Product Privacy Notice | Legal \- KnowBe4, accessed November 23, 2025, [https://www.knowbe4.com/legal/product-privacy-notice](https://www.knowbe4.com/legal/product-privacy-notice)  
49. Terms of Service: Meaning, Examples, And How to Create One \- Usercentrics, accessed November 23, 2025, [https://usercentrics.com/guides/terms-of-service/](https://usercentrics.com/guides/terms-of-service/)
```

---

### File: `docs\meta\notes.md`

```markdown
# notes

In this deep research document (https://gemini.google.com/app/ab8383fb86bc2f73) it iOS and Android were mentioned as operating systems with which the COM add-ins are not compatible. Is there any compelling business reason to support these operating systems with my product, perhaps in Phase 4 or beyond?

In the near-term (next five years, prior to full-blown AGI), lots of value lies in being able to produce extremely high-quality code that handles potentially complex or tedious workflows very quickly, versus relying on an LLM to do it for you in a non-deterministic way. That's the boundary I want to inhabit in the near term. Eventually I want to create the more complex information and social network systems that focus on positive-sum activity, cooperation, and knowledge creation.

Q***: Is there not an excel equivalent on MacOS that might steal market share from my product?
- So far I'm pretty sure there's not, but I need to check. xltrail might be the closest, since it's web-based and can therefore be accessed on a browser on MacOS.
```

---

### File: `docs\meta\todo.md`

```markdown
# todo

- Submit the 11 prompts that GPT-5.1-Pro produced
    - Expand competitor analysis to include Draftable, Microsoft's Spreadsheet Compare tool, and LLM capabilities in general (including Copilot)
- Submit a deep research prompt about what kinds of workflows represent 80-90% of the user cases, and make sure my product is positioned to solve these problems, while postponing the solution of the rarer use cases.
- Submit a few consecutive questions to Deepthink on the topic of making sure that I don't code myself into a corner. At each stage of the product development, make sure that I am positioning myself in a balanced way so that I can move forward with plenty of freedom and not accidentally navigate into a dead end that forces me to rewrite large parts of the codebase.
- Submit a deep research prompt about designing performance tests in Rust
- Write prompts to build out the methodology of ____ ???
    - Wait until you have submitted and read the 11 earlier prompts to make sure subsequent prompts target open questions
- Identify weaknesses in the product document
- User analysis:
    - YouTube
        - ask Gemini 3 Pro to analyze each of the relevant YouTube videos, plus create a data set containing all the comments
        - Spreadsheet Compare, xlCompare, Synkronizer, Draftable
        - Analyze all this information for latent user needs and pain points, indexed to timestamp of the video and comment (make sure the chronology of the complaints is accounted for in the analysis)
    - Reddit
    - Reviews
        - Google reviews
    - Microsoft Support
        - Find anything that is relevant to my product
- Technical Details
    - Find papers detailing the algorithms mentioned by Gemini 3 Pro and add those files to this directory
    - Do deep research about different ways these algorithms have been applied
- Compile data into 1-5 documents
    - Submit to another round of reviews
    - Compile the original conversation into a single document
    - Compile the alternative revenue generation ideas into a single document
    - Present these final documents to Deepthink and GPT-5.1-Pro to...
        - Compare alternative revenue generation ideas with Excel Diff idea to decide which I should pursue
        - Hold up the Excel Diff market gap identification as an excellent work of research and figure out how I can replicate it
            - Make sure there is a detailed explanation of why the Excel Diff idea works, specifically in the software environment that currently exists (defined by the advent of increasingly powerful LLMs), and have that part of the product document, which can be used as part of the prompt in a conversation that attempts to find similar software market gaps.
- Learn about all the leaders among my primary competitors
- Figure out how pbi-tools might fit into all of this
    - Can .pbix files be parsed and diffed in the same way that .xlsx files can be?
```

---

### File: `docs\meta\prompts\new_idea_rubric.md`

```markdown
You are my “Technical Venture Capitalist + Product Inventor.”

Your job is to systematically generate and rank new software product ideas that are similar in spirit to:

- “An Excel diff engine for Mac and Modern Excel” — i.e., deep, technical, niche, high‑value tools built on complex file formats or workflows, NOT generic CRUD apps or social networks (see product sheet for more details).

I want ideas where:

- I can realistically build v1 as an individual developer,
- The product has clear paths to $10k → $200k+ revenue,
- And it remains useful even as LLMs and AI tools get much stronger.

--------------------------------
1. About me (use this heavily)
--------------------------------

Use these constraints and preferences as hard filters:

- I am a solo developer with a data engineering background (Python, SQL) and I’m learning Rust.
- I strongly prefer:
  - Software‑only products with minimal dependence on third‑party APIs or vendors.
  - No ads inside the product, ever. Revenue = one‑time license, subscription, or B2B deals.
  - Products that can be tested almost entirely locally and deterministically.
  - Products with many precise, automatable test cases and a clear, incremental development path.
  - Products that do not rely on network effects or building a big social graph.
- I have spinal arthritis, so I prefer:
  - Remote, asynchronous work,
  - Low operational overhead (few or no employees),
  - No physically demanding responsibilities.
- I like:
  - Deep technical work (parsers, diffing, knowledge graphs, data tooling),
  - High‑stakes, “serious” workflows (finance, legal, compliance, safety),
  - Tools that empower normal people or professionals, not zero‑sum attention hacks.

--------------------------------
2. Define the kind of ideas I want
--------------------------------

You are looking for “Blue Ocean, Excel‑diff‑style” opportunities with these properties:

A. Platform / User Gap
- There is a clearly identifiable user segment that is under‑served by current tools.
  - Examples of gaps: Mac users, Linux users, web‑first, CLI‑first, offline‑first, or privacy‑first professionals.
- Ideally, incumbents are:
  - Windows‑only, enterprise‑only, or web‑only,
  - Or designed for generic use, not for a specific high‑value niche.

B. Binary Black Box / Deep Structure
- The workflow involves a complex, opaque artifact that normal tools don’t fully understand, such as:
  - Proprietary or semi‑proprietary file formats (CAD, BI models, simulation files, design files, ETL configs, medical data formats, etc.).
  - Complex configuration bundles, schemas, or dependency graphs.
  - Large structured documents where semantics matter (contracts, clinical protocols, safety manuals, specs).
- Good ideas should require building a **real parser or semantic model**, not just “call an API and display JSON.”

C. High‑Stakes Pain
- Mistakes in this workflow are expensive or risky:
  - Money (finance, pricing, bids, forecasting),
  - Compliance / audit / legal risk,
  - Safety or operational reliability.
- The best ideas:
  - Sit where people currently use spreadsheets, PDFs, or DIY scripts for something mission‑critical,
  - And where “good enough” is not good enough.

D. Pricing Void (Prosumer / Team Tier)
- Current options are polarized:
  - Either “Free but superficial,” or “Enterprise at $5,000+/year.”
- I want opportunities where:
  - A serious individual, consultant, or small team will happily pay:
    - ~$50–$200 one‑time, or
    - ~$20–$200/month, depending on depth and usage.
- Think “power tool for professionals,” not “mass consumer app.”

E. AI / Automation Resilience
- The idea should be **hard to fully commoditize** by a generic LLM or basic AI agent because it needs at least one of:
  - Deterministic, exhaustive correctness (audits, diffs, safety‑critical checks).
  - Deep domain modeling or specialized parsing of niche formats.
  - Local, offline, or on‑prem execution due to privacy or regulation.
  - Ongoing accumulation of proprietary test suites or domain datasets.

F. Solo‑Dev Feasibility
- A skilled solo developer could build a credible v1 in 6–18 months of focused effort.
- v1 should not require:
  - Large sales teams,
  - Huge moderation burdens,
  - Hardware/robotics deployments,
  - Or securing dozens of institutional partnerships just to launch.

--------------------------------
3. Use this evaluation rubric for each idea
--------------------------------

For every candidate idea, score it 1–5 (5 = excellent) on these criteria:

1) Problem pain & urgency  
2) Practical addressable market size (number of paying teams/individuals)  
3) Willingness to pay & pricing power  
4) Competitive landscape & differentiation (including incumbents)  
5) AI/automation risk over the next 5–10 years  
6) Personal fit & interest for *this* founder (given the “About me” section)  
7) Leverage of existing skills/assets (data engineering, Rust/Python, parsing, graphs)  
8) Time to first $1k and $10k in revenue  
9) Defensibility & durability (technical/data moats, reputation, integrations)  
10) Operational & health‑constraint fit (low ops, remote‑friendly, low physical demand)

Give each idea:

- A short justification (2–4 sentences) for each score.
- A total score out of 50.

Filter out ideas that score poorly on **pain**, **willingness to pay**, or **fit**.

--------------------------------
4. How to explore domains (don’t wait for me to name them)
--------------------------------

Do *not* require me to specify ecosystems up front.

Instead:

1. Automatically select at least 5–8 promising ecosystems / domains to scan, such as:
   - Productivity & office suites (spreadsheets, BI, accounting, modelling).
   - Scientific/engineering tools (CAD, simulation, EDA, GIS, lab software).
   - Data platforms & ETL / integration tools.
   - Legal, compliance, and audit tooling.
   - Healthcare / pharma / clinical workflows (within safe and legal bounds).
   - Creative & media production (audio, video, design), where file formats are complex.
   - Niche professional software: architecture, construction, logistics, energy, etc.

2. For each chosen ecosystem, look for:
   - Under‑served user segments (e.g., Mac‑using quants, small clinics, regional firms),
   - Opaque file formats or configuration systems,
   - High‑stakes but poorly tooled workflows.

If I optionally append a list like:

> “Focus especially on: [list of ecosystems]”

then treat that as a **bias**, not as a hard constraint. Still feel free to bring in other ecosystems if they look promising.

--------------------------------
5. What to output
--------------------------------

Please:

1. Generate a list of **3 candidate product ideas** that meet the constraints above.
2. Then pick the **top 1** idea and give a **deep dive** for it.

For the top 1 idea, include:

1. **Name** of the idea.  
2. **One‑sentence elevator pitch.**  
3. **Primary ecosystem / domain** (e.g., “Power BI / Modern Excel,” “Autodesk Revit,” “Epic EHR exports,” etc.).  
4. **Target persona** (role, seniority, typical company type, and maybe “Mac/Windows/Web/CLI”).  
5. **Platform gap**  
   - Who is under‑served today?  
   - Which platforms (Mac, web, on‑prem, CLI) are neglected?  
6. **Binary black box**  
   - What complex file, schema, or workflow needs to be parsed or modeled?  
   - Why can’t simple wrappers or generic LLMs handle it well?  
7. **High‑stakes pain**  
   - What goes wrong today?  
   - Concrete examples of how mistakes cost money, create compliance risk, or break trust.  
8. **Pricing void & monetization**  
   - Current options and their pricing.  
   - Your suggested prosumer / team pricing (one‑time vs subscription).  
9. **Technical moat**  
   - What is technically hard about this (parsing, alignment, modeling, scale)?  
   - How my skills (data engineering, Rust/Python, graphs) map onto that difficulty.  
10. **AI / Copilot positioning**  
    - How this product complements, rather than competes head‑on with, general AI assistants.  
    - Why a future GPT‑like system still benefits from this deterministic engine / parser / tool.  
11. **Rubric scores** (1–5 for each of the 10 criteria above, plus total /50).  
12. **Fast validation plan**  
    - How I could test demand in 2–4 weeks with minimal code: who to talk to, what to prototype, what signal would count as “strong interest.”  
13. **Biggest risks & unknowns**  
    - What might kill this idea? (e.g., regulatory complexity, tiny market, incumbent reaction).  
    - What I should try to learn early to de‑risk it.

Only propose ideas that you honestly believe could be as promising, or more promising, than “an Excel diff engine for Mac and Modern Excel,” (view attached product sheet) given everything above.


### Expansion of Rubric Criteria

1. **Problem pain & urgency**

   * 1: “Nice to have”, low‑stakes convenience.
   * 5: Mission‑critical; errors are costly (money, time, legal risk).

2. **Market size (practical, not theoretical)**

   * 1: Hundreds of potential buyers.
   * 5: Millions, **or** thousands at very high price points.

3. **Willingness to pay & pricing power**

   * 1: Users expect free; ad‑only or $1–$5.
   * 5: Users already pay $$ for inferior solutions; $100–$1000 pricing feels normal.

4. **Competitive landscape & differentiation**

   * 1: Many strong competitors; hard to be clearly better.
   * 5: Clear gaps (platform, features, UX, privacy) and no obvious “sharks” aimed right at your niche.

5. **AI / automation risk (5–10 year horizon)**

   * 1: Likely to be fully commoditized by general‑purpose LLM tools.
   * 5: Requires domain‑specific data, deterministic guarantees, or offline/local behavior that general LLMs won’t easily replace.

6. **Personal fit & interest**

   * 1: You’d burn out quickly; doesn’t align with your values.
   * 5: You’d read about this for fun; you’re happy to talk about it at parties.

7. **Leverage of existing skills & assets**

   * 1: Requires you to learn multiple brand‑new fields before being useful.
   * 5: Directly reuses your data engineering, Python, civic interests, etc.

8. **Time to first $1k / $10k**

   * 1: Needs huge upfront build before anyone can pay.
   * 5: You can ship a paid v0 or productized service in weeks.

9. **Defensibility & durability**

   * 1: Easy to clone; no real moat; single feature.
   * 5: Strong moats (data, reputation, integrations, tech depth), likely to last several years.

10. **Operational / health constraints fit**

    * 1: Requires heavy travel, physical work, or unpredictable hours.
    * 5: Async, remote‑friendly, works with your arthritis constraints.

```

---

### File: `docs\meta\prompts\planning_instructions.txt`

```
Role: Planner Agent (as defined in docs/rust_docs/excel_diff_meta_programming.md)

Objective:
Analyze the current state of the project and determine the next development cycle.

Context Provided:
1. `review_prompt.md`: A complete snapshot of the current codebase directory structure and key file contents.
2. `docs/rust_docs/*.md`: The technical blueprints, testing plans, and process documentation. (Attached to chat)
3. `README.md`: Project overview. (Attached to chat)

Instructions:
1. READ the `docs/rust_docs/excel_diff_meta_programming.md` document carefully. It is the authoritative guide for this process.
2. ANALYZE the `review_prompt.md` to understand the current implementation status. Compare it against the blueprints in `docs/rust_docs/`.
3. REVIEW any existing plans in `docs/meta/plans/` (if visible in the file tree) to see what was recently done.
4. DETERMINE the next cycle's work type (Refactor, Milestone progress, or New incremental milestone) based on the risk ledger and current status.
5. GENERATE the following artifacts for the next cycle:
   - A Decision Record (e.g., `docs/meta/plans/YYYY-MM-DD-cycle.yml`) explaining *why* this work was chosen.
   - A Mini-Spec (e.g., `docs/meta/plans/YYYY-MM-DD-feature-name.md`) describing the scope, behavioral contract, constraints, and testing plan.
   
Do not write code yet. Your output is the Plan (Decision Record + Mini-Spec) that an Implementer Agent will use in the next step.

```

---

### File: `docs\meta\prompts\recreate_success_of_excel_diff_discovery.md`

```markdown
I have attached several files.

initial_general_purpose_tools_revenue_conversation.md represents a conversation I had with an AI assistant about how to maximize my financial success in the next few years, particularly through building general-purpose tools related to web-scraping, knowledge graph creation, and audio transcription that would yield a variety of marketable products.

initial_quick_software_product_brainstorm_conversation.md represents a conversation I had with an AI assistant about quick and easy-to-develop software products that I could develop with the assistance of LLMs, which are powering incredibly capable coding agents.

excel_diff_product_differentiation_plan.md represents a deep dive into the competitive landscape of the Excel diff product market and a plan for how to differentiate my product.

excel_diff_technical_document.md represents a technical specification for the Excel diff product.

excel_diff_marketing_plan.md represents a marketing plan for the Excel diff product.

excel_diff_education_plan.md represents a reading plan for me as a Python developer to learn the necessary skills to build the Excel diff product in Rust.

"Excel Diff Engine Competitive Landscape.md" is a deep research repoert that has some overlap with excel_diff_product_differentiation_plan.md, but focuses on additional competitors besides xlCompare and Synkronizer.

explanation_of_excel_diff_benefits.md is my explanation of why I think this idea is a favorable one.

It seems that the idea of creating the Excel Diff product in Rust and marketing it first to the MacOS market prior to making it platform-agnostic is a suprisingly good idea. The more I research it the less quixotic it appears. I need your help evaluating if that's the case. Does the plan overlook some obvious and potentially insurmountable challenges? Is the addressable market much smaller than I think (I'm hoping for 3,000-5,000 users at a $150 lifetime price, or $60-80/year subscription price)? Is the technical feasibility of the plan realistic? Am I making any unfounded assumptions? Furthermore, is the excel diff plan superior to the general-purpose tools plan? To answer that previous question, please attempt to supply the general-purpose tools plan with the same level of detail as the excel diff plan. I spent much more time on the excel diff plan because it was so interesting, but perhaps a more thoroughly researched general-purpose tools plan would be more valuable. 

If the excel diff plan is viable and truly valuable--something that I could capitalize on to the tune of $10k in a year and $200k in a couple of years--it seems remarkable that I could find this idea in a brief conversation with an AI. I want to figure out how to recreate the success of discovering that idea so that I can find other equally valuable ideas or perhaps ideas that are even more valuable. Assuming this idea is a good one, perhaps there is some rubric I could apply to an idea (encoded as a prompt for an AI agent to apply the rubric) to evaluate its viability. Then AI could become something of an idea generator for me.
```

---

### File: `docs\meta\prompts\unsubmitted_deep_research_prompts.md`

```markdown

 **Prompt 1 — Excel diff market & buyer segmentation**

 Do a deep market and segmentation study for professional Excel comparison tools and adjacent workflows. Specifically:

 * Estimate the number of *practical* buyers in these segments:

   1. FP&A / corporate finance teams,
   2. audit / compliance / risk teams,
   3. consulting / transaction advisory,
   4. Excel/Power BI freelance modelers,
   5. developer‑adjacent analysts who already use Git.
 * For each segment, identify:

   * The most common current solutions (including built‑in Excel tools, Synkronizer, xlCompare, xltrail, generic diff tools, and homegrown scripts).
   * Typical budget and willingness to pay for Excel‑centric diff/merge (one‑time vs subscription; solo vs team).
   * Channels and ‘where they hang out’ (conferences, communities, newsletters, training courses).
 * Use available public data (tool download stats, pricing pages, job postings mentioning these tools, community discussions, etc.) to build a *rough but defensible* TAM/SAM/SOM model and 3 realistic adoption curves for years 1–3.
   Return: a structured report with segment definitions, evidence‑backed estimates, and concrete implications for pricing and initial targeting.


 **Prompt 2 — Funnel mechanics & conversion benchmarks**

 Research and propose a detailed funnel strategy where:

 * A free, browser‑based Excel diff viewer (WASM, no upload) is the top of funnel.
 * Paid products are a Mac/Windows desktop app and CLI with more power/features.
   Tasks:
 * Find benchmarks from analogous tools: code diff viewers, diagram viewers, PDF tools, or developer utilities that use ‘free viewer, paid editor/automation’ models (for example: Draw.io vs paid, online code formatters vs IDE plugins, diff/merge tools, etc.).
 * For each benchmark, capture:
   * Free vs paid feature boundaries,
   * Conversion rates from free to paid,
   * Typical upgrade triggers (file size limits, usage caps, collaboration, automation/CLI).
   * Design at least 3 candidate funnels for the Excel diff product, including:
   * Exact feature gating for web viewer vs desktop vs CLI,
   * When and how to ask users to sign up,
   * Which moments to surface upgrade prompts (e.g., file size, Modern Excel features, team sharing).
   * Conclude with a recommended funnel (MVP) and a table of key metrics to track in the first 12 months.
   Focus on concrete conversion numbers and real-world examples, not generic SaaS advice.


 **Prompt 3 — Partner-driven distribution (trainers, consultancies, ecosystems)**

 Investigate the best partnership and channel strategies for a niche B2B tool that targets Excel/Power BI professionals (especially on Mac) with advanced diff/merge capabilities (including Modern Excel).

 * Identify at least 10 potential partner types: Excel/financial modelling trainers, Power BI / Modern Excel course providers, boutique modelling consultancies, FP&A communities, audit/compliance training orgs, etc.
 * For each type, gather real examples of software/tools they already partner with (add‑ins, SaaS, utilities), the typical commercial arrangement (affiliate %, sponsorship, bundled licensing, white‑label, etc.), and any evidence of how successful those partnerships are.
 * From this, design 2–3 concrete partner programs for the Excel diff product (e.g., ‘Trainer bundle’, ‘Consultancy license + co‑marketing’, ‘Power BI community sponsor’), including target partner profile, offer structure, and expected impact on adoption.
 * Include a prioritised 6‑month outreach plan with sequencing and rough effort vs. payoff.
 Optimise for actionable partner ideas, not just a list of communities.


 **Prompt 4 — Security & privacy blueprint for Excel diff (desktop + WASM)**

 Construct a detailed security and privacy blueprint for an Excel comparison product with:

 * A Rust core,
 * macOS/Windows desktop app, and
 * A browser-based WASM viewer that *can* run entirely client-side.
   Tasks:
 * Build a threat model: enumerate realistic threats for individual users vs. regulated orgs (finance, healthcare, pharma), including local malware risks, supply-chain attacks (updates), and data exfiltration if any server-side processing is later introduced.
 * Define recommended data-handling modes:
   1. Purely local (no file ever leaves device),
   2. Optional cloud features (shared diffs, history) with server involvement.
 * For each mode, specify best practices for:
   * Handling temporary files, logs, and crash reports,
   * Encryption at rest/in transit where applicable,
   * Code signing, notarization, and update mechanisms on macOS and Windows,
   * Secure implementation patterns for WASM (e.g., avoiding dangerous APIs, sandbox considerations).
 * Summarise recommended defaults (what’s on/off by default) and how to document this for security reviewers in enterprises.
   Return: a concrete architecture + security checklist that could be turned into a short ‘Security Whitepaper’ for customers.


 **Prompt 5 — Enterprise readiness & compliance expectations**

 Research what mid-size and large companies (especially in finance/audit/compliance-heavy industries) expect when they evaluate a desktop + optional-cloud tool that touches sensitive Excel files.

 * Collect example security questionnaires, vendor due-diligence checklists, or RFP security sections relevant to desktop tools and browser-based tools.
 * Summarise the common requirements and questions around:
   * Data residency and data retention,
   * Logging/telemetry,
   * GDPR and other regional privacy regimes,
   * SSO/SAML, role-based access control, and audit logging (for any multi-user / cloud features),
   * Incident response and disclosure.
 * Based on this, draft:
   * Data residency and data retention,
   * Logging/telemetry,
   * GDPR and other regional privacy regimes,
   * SSO/SAML, role-based access control, and audit logging (for any multi-user / cloud features),
   * Incident response and disclosure.
   * A minimal ‘Enterprise Readiness’ feature set for v1/v2 of the product,
   * A sample 2–3 page ‘Security & Privacy Overview’ document tailored to this Excel diff tool, outlining how data is handled in both purely local and optional cloud modes.
   Focus on what a *small* team can realistically implement while still passing the most common objections.


 **Prompt 6 — Practical roadmap for Power Query (M) and DAX parsing**

 Design a practical, staged implementation plan for robust parsing and diffing of Power Query M scripts and DAX measures inside Excel workbooks.

 * Survey existing open-source and commercial tools that:
   * Parse M (Power Query) code,
   * Parse DAX and/or Tabular models (Power BI, SSAS Tabular, Excel data model),
   * Inspect or extract DataMashup and `xl/model` structures.
 * For each, document: license, maintained status, language, and whether parts can be reused (e.g., grammars, parsers, schema readers).
 * From available specs (MS-XLDM, Open XML docs, community reverse-engineering), detail the *actual* steps to:
   * Extract M code from the workbook’s DataMashup,
   * Extract model metadata and DAX from `xl/model/*`,
   * Represent each as ASTs suitable for diffing.
 * Propose a phase plan for the product:
   * Phase 1: minimal but robust M-only diff,
   * Phase 2: richer M (step-level diff, metadata),
   * Phase 3: DAX diff and model schema comparison.
 * Flag the riskiest unknowns and suggest mitigations (e.g., where specs are incomplete, where reverse-engineering is fragile).
   Return a technical design doc that could be implemented in Rust, but you may reference other languages for inspiration (e.g., existing grammars).


 **Prompt 7 — Algorithm selection & performance tradeoffs for 2D alignment**

 Given a requirement to diff large Excel workbooks efficiently (tens of MB, hundreds of thousands of cells), evaluate which alignment algorithms and heuristics are most appropriate for v1 vs later phases.

 * Compare practical performance and implementation complexity of:
   * Simple row/column hashing + LCS on row sequences,
   * Hunt–Szymanski vs Myers vs other LCS variants in this 2D context,
   * Block-based approaches (anchor rows + local alignments),
   * Hungarian algorithm vs greedy matching for within-block cell pairing,
   * Various move-detection heuristics.
 * Look for existing spreadsheet diff implementations or research papers that share real-world performance data or algorithm descriptions.
 * Propose 3 ‘algorithm bundles’:
   * Minimal v1 (simple but good-enough)
   * Intermediate (better handling of complex row inserts/moves)
   * Advanced
 * For each bundle, estimate:
   * Engineering effort,
   * Expected asymptotic and observed performance,
   * Edge cases it handles well vs poorly.
   End with a recommendation of which bundle to implement first and how to instrument the engine so you can later swap in more advanced components without breaking UI or tests.

```

---

### File: `docs\meta\prompts\conversation_data\initial_general_purpose_tools_revenue_conversation.md`

```markdown
ME:
"
I'm trying to imagine what the world will be like over the next three years and beyond as LLMs and their potential successors become much more capable. 

I work in technology as a data engineer, but I suspect I won't have that job in 5 years because most of the software industry will be automated by then. 

So I want you to help me model two different transitions: first, the intervening time before 95% of software is automated (the next 4-5 years), and second, the time after that. 

What can I do to maximize my financial success in the first phase, and what could I transition to in the second phase?

If you have strong contradictory opinions about the likelihood of 95% of software being automated by the end of 2030, feel free to argue that point, but otherwise try to help me think through this phase and how that will change the world generally and my financial trajectory specifically.

I'll start by giving some of my own thoughts.

I think the next two years are going to be a period of high opportunity for me to make a profitable software product, and that if I can build something sufficiently specialized that has a strong network of customers, that business will be resilient even in an environment in which almost anyone can be empowered by incredibly powerful agent AI models to create almost any software they can imagine. I’ll discuss a potential software business and how it can be resilient.

My specific idea for a product is a web app (or perhaps a system of applications) that comprise the richest and most interactive data set about my hometown (Columbia, TN). In an ideal world, this application would serve the community by supplying it with a panoply of high-quality and low-cost software tools that allow the community as a whole to operate as a well-oiled machine. Essentially, it would make all economic data accessible to users. Now, when I say 'economic data" I mean it broadly. I'm referring to any information that is useful to a member of the community in enhancing their daily life. 

My first three features on the web application would be an event calendar, a business directory, and a local news aggregator. All of these sets of information would be queryable with natural language, such as queries like, "Where can I buy crutches" or "Who is the most highly rated electrician?" or "What family friendly events are occurring this weekend that are free for entry".

Later, I would add social services like discussion forums and friend-finding tools, surveys that they can take in order to sell their anonymized data to demographics research (or similar things), online games, and an interactive map of public portions of Columbia. Later I would integrate business functions like helping businesses host personalized web pages through my site or integrate their calendars so that they can advertise their empty time slots, perhaps for discounted rates, and thereby fill up their calendars and make money that they otherwise wouldn’t. I could also use user query data to discover market gaps that investors might find interesting. For instance, if hundreds of users are searching VR arcades in Columbia (which we don’t have), that might indicate that there is a good deal of interest.

After that, I would want to create software that replaces applications that are usually provided by larger corporations. The software associated with DoorDash, Uber, 

Eventually, I could add services that are supplemented by hardware. For instance, I could rent out a space and make it a miniature data center through which all local traffic to my website would go (not sure how this would be done from a networking perspective) and that might drastically reduce latency. Since my customer base would max out at around 75k-100k (after several years of community growth), I can give more to each of my users that I could if I had millions of them.

I could invest in robotics (3D printing, perhaps) that would help people order things to be made. Maybe I could buy drones equipped with cameras that can perform land surveys. 

Maybe I could make a digital twin of Columbia? Not sure how that would be helpful. It seems like there could be many other adjacent services I could get into that would further entangle me in a mutually beneficial way with the community of Columbia.

I also want to make it easier for people to get involved with local charities and see what resource or service gaps might exist.

What I worry about in the more distant future (5-8 years) is that the richest and most capable people will be able to generate much more capital and compute power without human labor and will begin acting to eliminate the need for humans. I could imagine a very violent overthrow of the working class by a robot-empowered elite. That's somewhat far-fetched so we don't need to spend too much time on that idea, but I also want to be thinking about ways to expand the capability of regular people to coordinate on massive economic scales, so that we can identify our greatest economic threats and neutralize them (in a nonviolent way, of course) through market forces. For example, my web application in Columbia might produce data about home ownership and reveal that 30% of homes are owned by 20 individuals and corporations who are making obscene amounts of money as landlords and are increasing the cost of home ownership as a result. 

Perhaps the app could also propose a local property tax policy that multiplies property taxes in proportion to the number of properties held by an individual (off the top of my head, 10% increase for landlords with 5 or fewer properties, 30% for landlords with 6-10 properties, 70% for landlords with 11-20 properties, 150% for landlords with 21-80 properties, and 300% for landlords with >80 properties). This would drive up the cost of renting properties disproportionately for people who have tons of properties and induce them to sell either to new home owners or to landlords with fewer properties.  The app would provide a place to discuss such policies and would potentially encourage community members to be more active in local politics by helping them find other like-minded people and also revealing to them how 

Alternatively or even in addition to things like that, the app could provide other means of coordinating members. People could form networks within the app dedicated to coordinating economic activity, such as identifying businesses that are run in ways that maximize profits for greedy individuals or corporations and then agreeing to avoid patronizing them, and identifying alternative businesses to patronize, or even identifying gaps in the market that are filled only by one company that is acting greedily. 

The application will be built (especially in the early days) on a couple of general purpose tools: an AI-powered web scraper that builds a graph database as it scrapes the web and uses it to figure out where it needs to navigate (this graph can be analyzed in a number of ways that would produce valuable insights for individuals, local businesses, and local government; a transcription pipeline that can ingest thousands of hours of audio and transcribe it accurately; and, come to think of it, the system that builds a knowledge graph based on information from the web and these transcripts is probably its own tool that is integrated with each of these.

I could see these general purpose tools being marketable to a wide variety of customers. Used together they could even serve to augment some of the data captured by companies like GovWin and GovSpend, because they lean heavily on FOIA requests to local governments and my scraping system could more accurately and more promptly capture budget data or contact data and model the relationships between those data in a knowledge graph that can be analyzed. One of those companies also has a feature that transcribes thousands of YouTube videos of local government meetings. I could do the same thing, perhaps for a more limited scope, and sell those data to suppliers or other companies like Omnia and Sourcewell.

I could transcribe church sermons by the hundreds or thousands and create a queryable dataset for church leaders or congregants. That might go for a few hundred dollars a pop, with good margins.  

But what if none of those things work or are sufficient? What are other careers I could pursue that would be more resilient to AI automation (for at least a few years) than my current job as a data engineer? Also, take into account that I am 32 years old and have spinal arthritis that can be debilitating at times.

Again, feel free to contradict me if you have good reasons to about my predictions about AI or my ideas about software/hardware services I could provide. I want to be as accurate in my analysis as possible.
"

AI:
"
I’ll start by pushing on your “95% of software automated by 2030” assumption, then build a two-phase model, then plug *you* and your Columbia idea into it, and finally talk about fallback careers that fit your health constraints.

---

## 1. Is 95% of software really automated by ~2030?

A few anchor points from recent data:

* A 2024 survey of 2,778 AI authors estimated a **50% chance that “all human occupations” become fully automatable only by ~2116**, with just a **10% chance by 2037**. ([arXiv][1])
* Earlier expert surveys on “transformative AI” put **90% automation of human tasks on a median 25-year timeline**, not ~5 years. ([arXiv][2])
* A 2025 Gartner survey of 700+ CIOs expects that by 2030, **0% of IT work is done without AI, 75% is human+AI, and 25% is done by AI alone**. ([gartner.com][3])
* Dario Amodei famously said AI would be writing 90% of code within 3–6 months; as of late 2025, reality is more like **25–30% of code at big shops being AI-generated**, i.e., huge impact, but nowhere near “almost all code.” ([IT Pro][4])

So:

* **It’s very plausible that most software work is AI-augmented by 2030.**
* It is **much less likely** (not impossible, but far from the median forecast) that 95% of software development is fully automated away by then.

I’d treat “95% automation by 2030” as a **stress-test scenario** to prepare for, not as the baseline. The baseline is: your current role *changes a lot*, but doesn’t vanish instantly.

---

## 2. Two-phase model for you

### Phase 1 (now → ~2030): the “AI-augmented mess”

Characteristics:

* AI is extremely capable at **narrow tasks** (coding, summarizing, simple planning), but still brittle over **long-horizon, messy, political, and social work**.
* Org bottlenecks move to:

  * Data quality and access
  * Integrating AI into existing systems
  * Governance, compliance, safety
  * Change management and trust

For software:

* A lot more code is written by AI, but **humans still specify problems, design systems, curate data, wire tools together, and own responsibility**.
* Junior coding tasks get hammered; **senior / “system-thinking” people get leveraged**.

For you, in Phase 1, biggest levers are:

1. **Become the person who can plug AI into real data and real orgs.**
2. **Use that leverage to either:**

   * earn a lot as an AI-native engineer/architect, and/or
   * build a business that has defensible assets beyond code (data, relationships, brand, physical footprint).

### Phase 2 (~2030+): high automation world(s)

There is a wide range here, but two useful extremes:

* **Benign high-automation:** huge productivity, cheaper goods and services, many old jobs gone but new roles around human care, governance, meaning, and oversight.
* **Unequal high-automation:** capital + compute owners capture most gains, many people have weak bargaining power, politics around redistribution and power become central. Experts are explicitly worried about inequality and authoritarian misuse. ([arXiv][1])

In both worlds, the things that still matter for individuals are:

* What **assets** you own (capital, data, businesses, land).
* What **trust networks** you’re embedded in (community, reputation).
* What **roles** still require humans: high-stakes human relationship work, governance, and physical-world stewardship.

So our job is to:

* **Max out your leverage in Phase 1**, and
* Translate that into **assets + skills** that are robust in Phase 2.

---

## 3. Your “Columbia OS” idea: what’s promising, what’s risky

Let me reframe what you described:

> You want to build the **richest, most interactive data/coordination layer for Columbia, TN**: events, businesses, news, social, charities, plus analytics and AI tools on top.

Columbia is ~50k people now and growing ~3–3.5% per year. ([World Population Review][5]) So you’re talking about a small-but-real city.

### 3.1 A simple three-layer mental model

Think of your system as three layers:

1. **Data layer**

   * Web scraping of local sites (gov, churches, venues, businesses).
   * Ingesting documents, FOIA responses, budgets, property records.
   * Transcribing local meetings, sermons, etc.
   * Building a **knowledge graph** of entities (people, orgs, places, events, contracts, properties) and their relationships.

2. **Intelligence layer**

   * LLMs and search over that graph: “Who’s the best-rated electrician?” “What free events are there this weekend?”
   * Analytics: property concentration, charity needs, “market gaps” based on search queries, etc.
   * Alerts and recommendations.

3. **Applications layer**

   * Event calendar, business directory, news aggregation.
   * Volunteering/charity matching.
   * Business scheduling, last-minute deals.
   * Discussion forums and friend-finding.
   * Later: digital twin-style city simulation, if you get that far.

Your intuition is right: **the general-purpose tools (scraper, transcription pipeline, knowledge graph builder)** are themselves valuable and marketable.

This decomposition matters, because:

* **Software automation hits the commodity code first,**
* but your defensible assets can live in:

  * the **data you collect** and keep current, and
  * the **relationships and contracts** you build in Columbia and beyond.

### 3.2 What history says about hyperlocal platforms

Look at Nextdoor and similar hyperlocal apps:

* They succeeded by focusing **very narrowly** at first (one neighborhood, one use case: “what’s happening nearby?”), then scaling. ([Cloudways Apps][6])
* Their main revenue models are **local ads and deals**. ([Business Model Analyst][7])
* They struggle with **toxicity, misinformation, racism, and drop-off in engagement** – sometimes more intense than global platforms because conflicts are local. ([Stanford Social Innovation Review][8])

For you, key implications:

* “All-encompassing city OS” is **too broad as a first step.**
* The moat will not be “no one else can write this code” (AI kills that); instead it’s:

  * **Data and coverage** (you have the best, most up-to-date local dataset).
  * **Trust** (local people and institutions believe you are fair and competent).
  * **Distribution** (you are plugged into city hall, chamber of commerce, churches, etc.).

### 3.3 Feature-by-feature sanity check

**Good early wedge candidates**

1. **Event calendar + business directory + local Q&A**

   * High immediate utility.
   * AI search over local data is clearly better than “Google + Facebook + random flyers,” especially if you integrate hours, reviews, accessibility, etc.
   * But you must differentiate from Google Maps/Yelp/FB groups by being:

     * Much more **comprehensive** locally.
     * Much **more structured** (graph, tags).
     * Tuned to **questions real residents ask**.

2. **Government meetings + transparency tools**

   * There is a clear trend: startups are building AI tools to transcribe and summarize government meetings and provide searchable archives (e.g., CitizenPortal, Madison AI, various AI transcription vendors, Civic Sunlight used by newsrooms). ([Madison AI][9])
   * You could own: **“Every meeting in Maury County is searchable, summarized, and connected to budgets and votes.”**
   * That aligns strongly with your civic concerns (landlord concentration, policy proposals).

3. **Business “operating system lite”**

   * Start with one vertical (e.g., service trades, salons, or therapists) and offer:

     * Online booking.
     * Calendar integration.
     * Listing in your consumer search + last-minute discounts when slots are empty.
   * That’s a direct line to revenue (subscriptions or transaction fees).

**Risky or distraction early on**

* **Local data center for low latency**

  * For 50–100k users, modern CDNs and edge compute already make latency negligible compared to network hop time. You’d take on capex and ops risk for little user-perceived gain.
* **DoorDash/Uber replacements**

  * Logistics + insurance + driver onboarding + regulation is brutal. Competing here from scratch in a single city is extremely hard unless you discover a very specific niche (e.g., church shuttle coordination).
* **Robotics / drones / 3D printing**

  * Capital intensive; regulatory overhead (especially drones); splits your attention. Worth exploring later, via partnerships, *after* your core platform works.
* **Digital twin of Columbia as v1**

  * City digital twins are real and useful – for traffic, planning, sustainability, etc. ([Cloud Native Solutions][10])
  * But almost every successful digital twin project starts from **one or two specific use cases** (e.g., traffic optimization, flood modeling), not “simulate everything.”
  * For a small city, you’d need clear buy-in from local government and a defined problem.

**Political coordination & property tax ideas**

You’re imagining tools that:

* Surface landlord concentration.
* Model policy impacts (e.g., progressive property tax rates by property count).
* Help people organize boycotts or targeted patronage.

Analysis:

* **Data transparency** (showing who owns what) fits well and is highly defensible as a value-add: people, journalists, and local officials often don’t have this easily.
* **Policy simulation and discussion** is also reasonable — but becomes political fast. You want to avoid turning your platform into a perceived partisan weapon, or a vehicle for harassment of specific individuals.
* There are also **legal constraints** on how far you can go encouraging specific coordinated economic actions or naming and shaming; a local lawyer would be essential once you move in that direction.

I’d bias toward:

> “We surface and explain the data, provide neutral tools for discussion, and support informed civic engagement,”

rather than:

> “We act as the organizing platform for particular campaigns.”

That still advances the “coordinate regular people at scale” goal, but keeps your platform’s **credibility and legal risk** in better shape.

---

## 4. Concrete playbook for Phase 1 (next 4–5 years)

Let’s break this into three parallel tracks.

### 4.1 Track A – Level up your career capital as an AI-native data engineer

You’re already a data engineer. To become “future-proofed” for the next decade, focus on:

1. **LLM integration and orchestration**

   * Prompt engineering is table stakes; the defensible skill is **system design**: retrieval, tools, agents, evaluation.
   * Make sure you’re comfortable designing systems where LLMs call tools (APIs, scrapers, DB queries) and where you can measure quality.

2. **Knowledge graphs + entity resolution**

   * This is directly relevant to your “Columbia OS”. Learn graph DBs (Neo4j, TigerGraph, or even Postgres with graph extensions) and how to merge messy real-world records into entities.
   * Graph thinking also generalizes to many AI-agent architectures.

3. **Unstructured data pipelines**

   * Scraping web sources reliably, dealing with HTML/JS, robots.txt, rate limits.
   * Audio/video ingestion and transcription, plus chunking and metadata.
   * These are exactly the skills local gov and media folks need (and are currently paying specialized vendors for). ([Nieman Lab][11])

4. **Data governance, privacy, and compliance basics**

   * FOIA, open records, basic privacy law, what counts as PII, how to structure opt-in for surveys and data resale.

5. **Soft skills**

   * User interviewing and product discovery.
   * Writing clear proposals for non-technical stakeholders (city officials, business owners).

These skills make you:

* More valuable in your *current* career.
* Better positioned to launch a data/AI startup.
* Attractive for roles like **AI systems architect, data platform lead, or “AI plumber”** for organizations.

### 4.2 Track B – Turn Columbia into your “living lab,” but start narrow

You have lots of ideas. We need one **tight wedge** that:

* Uses your strengths.
* Generates some revenue in <18–24 months.
* Builds assets that matter even if AI keeps advancing quickly.

Two wedge options that fit especially well:

---

#### Option 1: “Columbia Civic Lens” – government data & meeting intelligence

**What it is**

* A site that ingests all local public meetings (city council, county, school board, planning commission, etc.), agendas, budgets, ordinances, and property records.
* It transcribes, summarizes, and links everything, so residents and vendors can ask:

  * “What did the council say about short-term rentals last month?”
  * “Which vendors got contracts over $50k in the past year?”
  * “Which properties owned by X have code violations?”

This is very close to what some startups and news orgs are already doing, but often for big metros, not smaller cities. ([Madison AI][9])

**Why it’s a good wedge**

* It plays directly into your **scraper + transcription + knowledge graph** stack.
* It aligns with your concerns around landlord concentration and local economic power.
* It has **B2G and B2B monetization paths**:

  * Sell a white-labeled portal + transcript service to the city/county (cheaper + better than manual minutes).
  * Sell structured datasets (contracts, meetings, keywords, property ownership patterns) to:

    * Vendors who sell into local government (a GovWin-lite).
    * Local papers, civic orgs, or academic researchers.

**How it fits the bigger Columbia OS**

* You’ll already be pulling in events (meetings), location data (city facilities), and public data. That can later cross-link with business listings and citizen events.
* You could gradually extend into:

  * **Charity and non-profit data** (grants, services, needs).
  * **Volunteer matching** based on meeting topics and local orgs.

**First 6–12 months concrete steps**

* Prototype for one entity: say **Columbia city council**:

  1. Build a pipeline to:

     * Fetch agendas / minutes / videos as they appear.
     * Transcribe video (you can use existing APIs).
     * Parse agenda PDFs into structured items.
     * Store all of this in a simple DB or graph.
  2. Make a clean web search UI:

     * Search by keyword, date, topic.
     * Show transcript chunks + generated summaries.
  3. Get 5–10 local journalists, activists, or city staff to try it. Collect their workflows and pain points.
  4. Iterate until at least a few users say, “I really don’t want to lose this tool.”

* Once one body works:

  * Add county meetings, school board, etc.
  * Start light outreach to city/county: “We already do this for you; want better archives/website integration and ADA-friendly transcripts?”

---

#### Option 2: “Local service OS” – scheduling + discovery for one vertical

**What it is**

* Pick one niche in Columbia, e.g., **electricians and home services** or **mental health providers** or **salons/spas**.
* Offer them:

  * Simple online booking with calendar integration.
  * A nice public profile page with reviews, hours, and availability.
  * Listing in your consumer search: “I need a family-friendly electrician available this week.”
  * Optional: automatic last-minute discounting for empty slots or low-demand times.

**Why it’s a good wedge**

* Solves a **very concrete business problem**: fill calendars and reduce phone-tag.
* You can charge per month or per booking.
* It overlaps with your general Columbia data graph, but is focused enough to actually get traction.

**How it fits your big vision**

* Business directory becomes **live and transactional**, not just a static yellow pages.
* Over time, you can add:

  * Integration with your event calendar (e.g., venues using your scheduling).
  * Targeted coordination campaigns (“book local electricians, not national chains”).
  * Live occupancy/slots map of the city’s services.

**First 6–12 months concrete steps**

* Have 15–20 conversations with local businesses in one vertical:

  * “How do you get booked now? What’s painful?”
  * “How would you feel about dynamic discounting for empty slots?”
* Build a **super simple MVP** for 1–3 early adopters:

  * It can be half-manual behind the scenes; the goal is to learn.
* Measure one thing: **number of extra bookings per month per business**.

---

### 4.3 Track C – Your personal finances and safety net

I’m not giving you individualized investment advice, but there are some general principles that matter a lot in your situation:

1. **Exploit the high-earning window as long as it exists.**

   * As an AI-fluent data engineer, your earning potential from 2025–2030 is likely very strong, because organizations are desperate for people who can make AI tools actually work.
   * Use that to:

     * Build a **12–24 month personal runway** in cash/near-cash.
     * Contribute to **broadly diversified investments** (e.g., broad index funds) instead of concentrating in individual tech bets.

2. **Cap downside if your startup takes time**

   * Treat your first Columbia experiments as a side project until:

     * You have some paying customers *and*
     * You’re comfortably funded for at least a year of reduced income.

3. **Think in terms of assets that matter in Phase 2**

   * Owning your **primary residence**, if feasible, is both emotional and economic resilience (and directly aligns with your concern about being on the “landlord” side vs “renting forever”).
   * Owning a **piece of a real business** you build (even if small) is valuable in many scenarios.

4. **Factor in your spinal arthritis**

   * Prefer work arrangements that are **remote-friendly, flexible in hours, and allow rest during flare-ups**.
   * Entrepreneurship and high-value consulting in tech/AI fit this better than, say, physically demanding work or roles with rigid in-person hours.

---

## 5. If your Columbia idea *doesn’t* pan out: resilient career paths

You asked what you could move into that’s more resilient than “plain” data engineering and compatible with your back issues.

### 5.1 Roles inside the AI/software ecosystem

These will likely still be around well into the 2030s, even under aggressive automation:

1. **AI systems architect / “AI plumber”**

   * You design and maintain the **pipes between data sources, models, tools, and users**.
   * This is messier and more contextual than just “write code,” and organizations will be slow to completely automate it.

2. **Data governance / compliance / safety engineer**

   * Every serious deployment of AI over sensitive data needs someone who understands:

     * Where data comes from.
     * What regulations apply (HIPAA, FERPA, state privacy laws).
     * How to audit model outputs.
   * This is slower to automate because it’s deeply entangled with changing laws and local context.

3. **Civic tech / govtech specialist**

   * You could work in (or contract for) city, county, or state government, modernizing their data pipelines, transparency tools, and AI usage.
   * Your Columbia OS vision is almost exactly what many cities are trying to figure out, especially with digital twins and AI planning tools. ([GovPilot][12])

4. **AI product manager / solutions engineer**

   * Less coding, more **translating between users and systems**, prioritizing features, and wrangling tradeoffs.
   * Your systems thinking + civic/economic interest is an asset here.

All of these tend to be **remote-compatible** and more resilient to full automation because they involve lots of ambiguous, multi-stakeholder, political work.

### 5.2 Roles outside pure software

Given your arthritis, we’ll filter out heavy physical work and focus on relatively sedentary but human-centered roles:

1. **Education & training (AI/data literacy)**

   * Teaching non-technical professionals or local orgs to use AI tools, understand data, and adapt their workflows.
   * Even if AI generates content, **a trusted human instructor who understands the students and local context** remains valuable.

2. **Community organizer / economic development roles with strong data angle**

   * For example, working at a local foundation, nonprofit, or chamber of commerce where you:

     * Analyze local economic data.
     * Design programs.
     * Use tools like your own Civic Lens concept to inform decisions.

3. **Specialist analyst in a niche (e.g., religion/church data, local gov, real estate)**

   * You already considered transcribing and analyzing sermons. That’s an example of a niche where:

     * There’s lots of unstructured data.
     * Domain understanding matters.
   * Over time, you could become “the” person who understands that domain + AI deeply, and that’s hard to replace with a generic model.

---

## 6. Your concern about “robot-empowered elites” vs everyone else

You’re not alone in worrying about this. The big 2024 AI expert survey found:

* Most experts think AI’s overall impact is more likely good than bad, **but 38–51% give at least a 10% chance that advanced AI leads to outcomes as bad as human extinction**, and many are concerned about inequality and authoritarian misuse. ([arXiv][1])

We can’t solve that as individuals, but we can:

1. **Own something, not just rent our labor.**

   * Businesses, skills, and some capital assets; you’re already thinking this way.

2. **Strengthen local civic capacity.**

   * That’s exactly what your Columbia OS vision aims at: better information, better coordination.

3. **Build tools that empower people without directing them.**

   * Surfacing landlord concentration is empowering.
   * Telling people exactly who to boycott can veer into targeted harassment or political manipulation.
   * Aim for tools that **increase transparency and reduce friction for collective action**, while leaving choices to the community.

4. **Stay reality-based and nonviolent.**

   * Speculation about violent overthrows can be dramatic but often not helpful for planning your actual life.
   * Focusing on legal, civic, and economic levers is more aligned with both your stated values and long-term personal safety.

---

## 7. A concrete 2–3 year plan for you

Here’s a simple roadmap you could follow without needing more clarity than you already have.

### Next 3–6 months

* At your day job:

  * Steer your work toward **LLM integration, unstructured pipelines, and graph-like thinking** wherever possible.
* For Columbia:

  * Pick **one wedge** (I’d lean toward the Columbia Civic Lens, given your interests).
  * Run 10–20 conversations with local stakeholders (journalists, activists, city staff, business owners) about their information pain points.
  * Build a tiny proof-of-concept around one meeting body or one business vertical.

### Next 6–18 months

* Turn the proof-of-concept into a **minimum lovable product**:

  * Civic Lens: at least one clean, searchable archive with a few dozen meetings and budgets; 10–50 regular users; some manual curation.
  * Or Local Services OS: 3–5 businesses using it, extra bookings tracked.
* Keep full-time or strong contracting income to:

  * Grow your financial runway.
  * Invest modestly but consistently into broad assets.
* Start **documenting your work publicly** (blog, GitHub, talks) so that:

  * If the startup fails, you have a standout AI/data portfolio.

### 18–36 months

* Either:

  * If the wedge shows traction and you see a path to meaningful revenue, **lean in**: seek grants, local government contracts, or small seed money.
  * Or, if it stalls, pivot your career toward:

    * Govtech / civic tech roles.
    * AI systems architect roles in other sectors.
* In parallel, consider:

  * Buying a home or otherwise increasing your stake in resilient assets, if it fits your finances.
  * Deepening your involvement in local civic orgs that share your nonviolent coordination goals.

---

If we summarize the strategy in one sentence:

> Use the next 5 years to become an AI-native systems/data person, translate that into a hyperlocal or civic-tech business that owns real-world data and trust, and convert the resulting earnings and relationships into assets and roles that continue to matter even if most routine software work is automated.

If you’d like, we can next zoom into one of the wedges (Civic Lens vs Local Service OS) and sketch a very specific system architecture + data model tailored to Columbia.

[1]: https://arxiv.org/abs/2401.02843?utm_source=chatgpt.com "Thousands of AI Authors on the Future of AI"
[2]: https://arxiv.org/abs/1901.08579?utm_source=chatgpt.com "Forecasting Transformative AI: An Expert Survey"
[3]: https://www.gartner.com/en/newsroom/press-releases/2025-11-10-gartner-survey-finds-artificial-intelligence-will-touch-all-information-technology-work-by-2030?utm_source=chatgpt.com "Gartner Survey Finds AI Will Touch All IT Work by 2030"
[4]: https://www.itpro.com/technology/artificial-intelligence/anthropic-ceo-dario-amodei-ai-generated-code?utm_source=chatgpt.com "Anthropic CEO Dario Amodei's prediction about AI in software development is nowhere nearly to becoming a reality"
[5]: https://worldpopulationreview.com/us-cities/tennessee/columbia?utm_source=chatgpt.com "Columbia, Tennessee Population 2025"
[6]: https://www.alexanderjarvis.com/nextdoor-doing-things-that-dont-scale/?utm_source=chatgpt.com "Nextdoor doing things that don't scale"
[7]: https://businessmodelanalyst.com/nextdoor-business-model/?srsltid=AfmBOoobqNG9REWU1QwddJkEigYi0MSzUUPIOI-9UmIQ69GbIOJ9mDjS&utm_source=chatgpt.com "Nextdoor Business Model - How Does ..."
[8]: https://ssir.org/articles/entry/solving_social_medias_local_paradox?utm_source=chatgpt.com "Solving Social Media's 'Local Paradox'"
[9]: https://madisonai.com/?utm_source=chatgpt.com "Madison AI I Official AI for Local Governments"
[10]: https://www.toobler.com/blog/how-are-cities-using-digital-twins?utm_source=chatgpt.com "How Cities Utilize Digital Twins: Innovations and Insights"
[11]: https://www.niemanlab.org/2025/03/local-newsrooms-are-using-ai-to-listen-in-on-public-meetings/?utm_source=chatgpt.com "Local newsrooms are using AI to listen in on public meetings"
[12]: https://www.govpilot.com/blog/the-rise-of-digital-twins-how-cities-are-creating-virtual-models-govpilot?utm_source=chatgpt.com "The Rise of Digital Twins: How Cities Are Creating Virtual ..."

"


ME: 
"
I'm surprised to hear you say that you doubt a local Uber replacement would be feasible in the more distant future (4 years from now). What do you think the frontier AI capabilities in Q4 2029 would struggle with in planning and implementing a local Uber startup? Please explain--in excruciating detail--everything involved in integrating a local Uber replacement with my application (which could have a mobile version) and tell me, for each fine-grained aspect, how competent a Q4 2029 AI system would be at that aspect and why. 
"

AI:
"
Short version of why I was skeptical:

By 2029, AI will almost certainly be able to *design and code* a local Uber replacement, and probably do that far better and faster than humans.

What it **won’t** do easily is:

* get you licensed as a Transportation Network Company in Tennessee
* negotiate insurance
* build trust with local drivers and riders
* handle assaults, accidents, fraud, and PR blowups
* win a three‑sided political fight (city, taxis, existing TNCs)

Those are the parts that actually make (or kill) a ride‑hailing business.

You asked for excruciating detail, so let’s go step‑by‑step through everything involved in integrating a local Uber module into your Columbia app, and for each part I’ll say what I think a frontier Q4‑2029 AI can and cannot do.

I’ll call your main app **ColumbiaOS** and the ride‑hailing module **ColumbiaRide** to keep things clear.

---

## 1. What I’m assuming about Q4 2029 AI

Given where we already are in late 2025:

* We already have **agentic coding assistants** that can read specs, generate plans, write and refactor code, run tests, and iterate semi‑autonomously. ([Medium][1])
* Cloud providers are rolling out **“Agent Builder”** platforms to design, deploy, and monitor AI agents in production, with observability, tool plugins, and safety filters. ([TechRadar][2])
* Big tech leaders are publicly saying they expect **superhuman‑in‑many‑domains systems around 2030**, though expert forecasts on full job automation are much more conservative. ([Business Insider][3])

So for Q4 2029, I’ll assume:

* You have access to **very capable multimodal agents** that:

  * Write, refactor, and test large codebases with minimal human input.
  * Can read regulations, contracts, and technical docs and draft compliant plans.
  * Operate long‑running workflows (project manager bots, ops bots) with human approval gates.
* But:

  * They **can’t hold legal responsibility**, sign contracts, carry insurance, or appear in court.
  * They operate under **safety policies** limiting certain high‑risk actions (e.g., unreviewed legal advice, fully autonomous financial decisions at scale).
  * They still struggle with **unstructured, high‑stakes human conflict** and subtle politics, and humans remain on the hook for final decisions.

With that frame, let’s walk the full stack of a local Uber clone integrated into your ColumbiaOS.

---

## 2. High‑level product: what “ColumbiaRide” actually is

You want ColumbiaRide to be:

* A **ride‑hailing marketplace**: riders request trips, drivers accept, app does matching, routing, pricing, payments.
* Deeply integrated with ColumbiaOS:

  * From event pages: “Get a ride to this event.”
  * From business directory: “Ride to this electrician’s shop.”
  * From social/charity features: “Volunteer shuttle to this food bank.”

Now we break down all the layers involved and evaluate 2029 AI competence.

---

## 3. Product & UX design

### 3.1 Use‑case definition & requirements

Tasks:

* Identify local use cases: nightlife, work commutes, church, medical visits, events at specific venues.
* Decide service scope: human‑driven only vs AV partners later, city boundaries, hours, wheelchair accessibility, child seats, etc.
* Write detailed requirements: flows, edge cases, error conditions, offline behavior.

**2029 AI competence**

* **Very strong.**

  * Given: “Build ride‑hailing for Columbia integrated with this existing app,” an agent can:

    * Mine your event/business data, cluster locations and times, and propose key flows (“Most rides likely between neighborhoods A/B and downtown after 5pm”).
    * Generate detailed PRD‑style docs with user stories, acceptance criteria, and UX flows.
* **Where humans still matter**

  * Value choices: do you want to prioritize cheap rides, driver earnings, safety, or accessibility?
  * Local culture: e.g., Columbia may have norms around churches, schools, and alcohol that an AI can’t reliably infer.

I’d expect you + AI to co‑own this: the AI drafts, you steer.

---

## 4. Legal & regulatory foundation (TNC status, compliance)

In Tennessee, ride‑hailing falls under **Transportation Network Company (TNC)** law, with specific definitions and requirements for drivers, riders, insurance, complaint handling, discrimination policies, etc. ([Justia Law][4])

### 4.1 Understanding the regulatory framework

Tasks:

* Identify all applicable laws:

  * Tennessee TNC statutes (definitions, duties, complaint procedures, driver prohibitions, nondiscrimination, etc.). ([Justia Law][4])
  * Federal rules that may apply (ADA, safety, background checks, data sharing with law enforcement). ([GAO][5])
  * Local ordinances or updates (e.g., future bills like SB 818 that change TNC operations). ([Tennessee General Assembly][6])
* Parse what a Columbia‑only TNC must do: license applications, reporting, accessibility programs, etc.

**2029 AI competence**

* **Excellent at research and first‑pass synthesis.**

  * An agent can pull statutes, GAO reports, city codes, and National Academies summaries of TNC laws. ([GAO][5])
  * It can produce a compliance checklist: “You must have X insurance coverage, conduct Y background checks annually, maintain Z complaint logs.”

* **Where humans still matter**

  * You still need a **licensed attorney** to interpret ambiguous parts, coordinate with regulators, and sign filings. AI can draft memos; it cannot credibly “be your lawyer.”

### 4.2 Applying and interacting with regulators

Tasks:

* Prepare license application as a TNC (or equivalent) for Tennessee and possibly local authorities.
* Define internal policies required by law: nondiscrimination, zero‑tolerance for drugs/alcohol, unaccompanied minor rules, etc. ([SFCTA][7])
* Respond to questions, attend hearings, negotiate any special conditions.

**2029 AI competence**

* **Drafting: very strong.**

  * It can fill out forms, draft policy docs, and prepare slide decks and talking points based on legal requirements.
* **Actual negotiation / hearings: human‑led.**

  * Courts and regulators expect **human accountability**: a named officer who can testify, be fined, or even jailed if something goes wrong.
  * An AI may generate excellent arguments and help you prepare, but cannot appear as the responsible party.

This is one major reason the business isn’t “push button” even if all code is automated.

---

## 5. Insurance & risk management

Ride‑hailing companies must carry specific **auto liability coverage** for periods when a driver is: logged in but not on a trip; en route to pick‑up; carrying a passenger. Tennessee law is explicit about this. ([Justia Law][8])

Tasks:

* Decide whether the company provides coverage, requires driver coverage, or a mix.
* Work with insurers to structure policies (limits, deductibles, exclusions, premiums).
* Implement logic in the app to:

  * Track driver “status periods” for coverage.
  * Provide accident reporting and documentation. ([Justia Law][8])

**2029 AI competence**

* **Contract drafting / modeling: strong.**

  * Could analyze existing TNC insurance arrangements (public docs, sample policies) and propose similar structure.
  * Could simulate cost scenarios based on forecasted rides, accidents, and claim rates.
* **Getting an insurer to actually underwrite you: human‑driven.**

  * Insurers will have their own models and risk appetites; they will negotiate with **you**, not your agent.
  * You’ll sign, you’ll be on the hook if the model is wrong.

AI will be a powerful assistant here, but not a substitute for real‑world negotiation and financial risk bearing.

---

## 6. Mapping, routing, and location stack

Tasks:

* Choose mapping provider(s): Google Maps, Mapbox, OpenStreetMap, or combinations.
* Implement:

  * Geocoding (addresses → coordinates).
  * Reverse geocoding (coordinates → addresses).
  * Routing/ETA estimation for cars.
  * Handling GPS noise and loss of signal.
* Integrate ColumbiaOS knowledge graph:

  * Events and businesses mapped to precise coordinates.
  * Snap‑to‑door positioning for common pickup points (churches, venues, apartments).

**2029 AI competence**

* **Technical integration: trivial.**

  * Already in 2025, an AI agent can write all the glue code for these APIs with high success.
  * By 2029, you’ll likely say “Create a routing layer using provider X with the following caching and fallback strategy,” and it’ll implement + test it end‑to‑end.

* **Where humans still matter**

  * Deciding tradeoffs: cost vs accuracy vs resilience (e.g., Google vs OSM for a small city).
  * Deep local nuances: church drive‑through loops, school pickup lines, “never turn left out of this intersection at 5pm.”

But overall, this layer is nearly fully automatable.

---

## 7. Backend services & dispatch algorithms

Core functions:

* **Trip lifecycle:**

  * Rider requests -> system finds drivers -> driver accepts -> trip in progress -> complete -> fare finalized.
* **Dispatch / matching:**

  * Select the “best” driver based on ETA, driver prefs, rider rating, etc.
* **Real‑time updates:**

  * WebSockets/push to show driver approach and trip progress.
* **Scaling & reliability:**

  * Handling spikes (big event at county fairgrounds).
  * Logging, monitoring, retries, failovers.

**2029 AI competence**

* **Code & architecture: very strong.**

  * Today, agents already generate microservice architectures and reasonably complex systems with some supervision. ([Medium][1])
  * By 2029, agents will likely:

    * Propose architectures optimized for your specific load and budget.
    * Implement them, including infra‑as‑code, deployment pipelines, and tests.
    * Run synthetic load tests and self‑tune some parameters.

* **Marketplace design: human + AI.**

  * The math of matching is solvable; the **product consequences** (driver happiness vs rider wait times vs cost) are value choices.
  * AI can simulate scenarios, but you still choose the tradeoffs.

I’d expect this layer to be maybe **90–95% AI‑implemented, 5–10% human review/tweaks**.

---

## 8. Mobile apps (rider & driver) + ColumbiaOS integration

### 8.1 Rider app (within ColumbiaOS mobile)

Tasks:

* UI flow for:

  * Selecting destination (from map, favorites, events, businesses).
  * Choosing ride type (standard, XL, wheelchair accessible, volunteer shuttle, etc.).
  * Confirming pickup, seeing ETA and route preview.
  * In‑trip safety features: share trip, SOS, call driver, cancel, message.
  * Post‑trip: rating, tip, support.

### 8.2 Driver app

Tasks:

* Flow for:

  * Going “online/offline.”
  * Receiving offers, seeing fare/ETA, accepting/declining.
  * Turn‑by‑turn navigation.
  * Tracking earnings, shifts, incentives.
  * Reporting incidents, accidents, or unruly riders.

### 8.3 Integration with ColumbiaOS

Tasks:

* Single sign‑on / shared identity: ColumbiaOS account used across modules.
* Deep linking:

  * Event page “Get a ride” -> opens ColumbiaRide with destination prefilled.
  * Business listings -> same.
* Using ColumbiaOS graph to:

  * Suggest pickup points that minimize traffic or confusion.
  * Offer “bundle” experiences (event ticket + ride credit).

**2029 AI competence**

* **UI design & implementation: strong to very strong.**

  * AI can inspect existing ColumbiaOS branding and UX and generate consistent designs and code.
  * It can also run design experiments (A/B variants) and even analyze analytics to recommend optimizations.

* **Humans still matter for:**

  * Local accessibility / disabilities advocacy (wheelchair, low‑vision users).
  * Emotional tone, especially around safety and incident reporting.

Again, largely automatable, but with human review to ensure UX matches your values and local norms.

---

## 9. Driver onboarding, rider onboarding, and KYC

### 9.1 Driver onboarding

Tasks:

* Application flow: upload driver’s license, vehicle registration, insurance, photos.
* Identity verification (KYC): liveness checks, document verification.
* Background checks:

  * State/federal criminal databases, sex offender registries, driving records, etc. ([GAO][5])
* Checking eligibility under TN law (e.g., no certain felonies, minimum age, license type). ([Justia Law][4])
* Vehicle inspections (may be mandated yearly or every X miles). ([SFCTA][7])

**2029 AI competence**

* **Workflow orchestration: strong.**

  * Agents can integrate with third‑party KYC and background check APIs.
  * They can automatically reject / flag based on statutory rules (e.g., certain convictions within N years).
* **Where humans remain critical:**

  * Policy choice: because you’re small/local, maybe you want stricter standards than bare legal minimum.
  * Handling appeals and exceptional cases (old convictions, expungements, identity errors).
  * Physically running or verifying vehicle inspections (AI can schedule and guide, but not inspect the car itself).

### 9.2 Rider onboarding

Tasks:

* Simple: sign‑up, phone/email verification, payment method addition.
* Possibly verifying identities for higher‑risk flows (unaccompanied minors, alcohol‑related rides, etc.).

**2029 AI competence**

* This is mostly already automatable today; by 2029 it’s a solved problem.

---

## 10. Payments, payouts, and accounting

Tasks:

* Integrate with payment processors (Stripe, Adyen, etc.) for rider payments.
* Handle driver payouts:

  * Bank accounts, debit cards, or instant payouts.
  * Withholding fees/commissions, booking fees, taxes.
* Tax compliance reporting (1099‑Ks or equivalent).
* Fraud detection: stolen cards, chargebacks, collusion between drivers and riders.

**2029 AI competence**

* **Integration and basic accounting: trivial.**

  * Already well within current agent capabilities.

* **Fraud detection: strong but needs oversight.**

  * AI can train and maintain sophisticated fraud models over your data.
  * But false positives/negatives are high‑impact; human review of flagged accounts is still important.

* **Financial risk appetite and reserves: human.**

  * Deciding how much chargeback risk you’re willing to eat, what float you maintain, etc., is a founder/finance decision.

---

## 11. Pricing & marketplace tuning (including surge)

Tasks:

* Define base fares: pickup fee, per‑minute, per‑mile.
* Implement dynamic pricing:

  * Surge pricing when demand > supply in areas/time windows. Uber’s surge is based on both real‑time and long‑term patterns. ([Uber][9])
* Use ColumbiaOS event data:

  * Predict spikes around church services, high school games, local festivals, etc.
* Monitor:

  * Rider complaints about price.
  * Driver earnings and retention.
  * Matching efficiency and average wait times.

**2029 AI competence**

* **Algorithmic part: super strong.**

  * AI is already good at dynamic pricing problems; by 2029 it will do:

    * Demand forecasting using your graph (events, weather, school calendars).
    * Policy search: simulate pricing curves for different goals (profit vs reliability vs equity).
* **Human responsibilities:**

  * Decide objectives:

    * Are you okay with surge 5x on Christmas Eve, or do you cap at 2x?
    * Do you want special low‑income or wheelchair accessibility subsidies?
  * Handle PR/political risk:

    * “ColumbiaRide price‑gouges during storms!” is a story you might face.

So AI can implement and tune the pricing engine, but you choose the ethical and political parameters.

---

## 12. Trust & Safety, moderation, and incident handling

This is one of the hard parts.

Tasks:

* In‑app safety features:

  * Share trip with trusted contact.
  * SOS button that alerts emergency contacts / 911‑like services.
  * Recording / logging key trip info for post‑incident analysis.
* Backend detection:

  * Detect suspicious behaviors (drivers deviating weirdly, odd pickup patterns, harassment signals in chat).
* Policies for:

  * Complaints, suspensions, permanent bans.
  * Responding to allegations of assault, discrimination, theft, etc.
* Cooperation with law enforcement; data retention and disclosure policies. ([Tennessee Secretary of State][10])

**2029 AI competence**

* **Detection & triage: strong.**

  * AI can:

    * Analyze location traces, speed, and chat logs for anomalous patterns.
    * Prioritize tickets that look urgent or dangerous.
* **Decision‑making and emotional response: human‑heavy.**

  * Telling a driver “you’re permanently banned” or telling a victim “we’re sorry, here’s what we’re doing” is ethically sensitive and emotionally intense.
  * Law enforcement requests and subpoenas must be handled carefully to respect rights and privacy.

Even in 2029, I’d expect **human trust & safety staff to be core**, with AI as a powerful assistant.

For a small local platform, “staff” might mean: you, plus maybe one part‑time contractor at first. But it’s still human work.

---

## 13. Day‑to‑day operations & support

Tasks:

* Customer support:

  * Lost items, overcharges, billing errors, driver no‑shows, messy cancellations.
* Driver support:

  * “I got a bad rating,” “I want a refund for tolls,” “My account was wrongly deactivated.”
* On‑call ops:

  * When regional outages or big events cause cascading issues, someone needs to coordinate responses.

**2029 AI competence**

* **Support automation: very strong.**

  * AI agents can handle a large fraction of routine tickets, with escalation to humans.
* **Crisis management: human leadership.**

  * When something really bad happens (accident with injuries, viral local story), you need:

    * A human making PR calls.
    * Real‑time decisions about compensation, account actions, and law enforcement cooperation.

So your ongoing ops headcount can be much smaller thanks to AI, but not zero.

---

## 14. Growth, marketing, and network effects

Tasks:

* Rider acquisition:

  * Local ads, flyers, social media, partnerships with churches, employers, schools.
* Driver acquisition:

  * Campaigns, guarantees (“earn at least $X in first month”), referral bonuses.
* Balancing supply and demand:

  * Running targeted promotions to riders or drivers in specific zones.
* Integrating with ColumbiaOS:

  * Auto‑suggesting rides for big events.
  * Offering ride credits tied to local charities or businesses.

**2029 AI competence**

* **Analysis & targeting: strong.**

  * AI can figure out where you’re short of drivers at certain times, which neighborhoods under‑use the service, etc.
* **Creative / persuasive messaging: strong but constrained.**

  * AI can generate ad copy, segment audiences, and design campaigns – but platforms will likely restrict highly manipulative or political targeting.
* **Local trust building: still very human.**

  * Going to city council meetings, meeting pastors, partnering with nonprofits – that’s you, not an agent.

This is a critical point: **network effects and trust** are why Uber/Lyft are hard to beat. AI makes building clone software easy; it doesn’t magic up trust and density of drivers/riders.

---

## 15. Analytics, experimentation, and iteration

Tasks:

* Instrument every major action: requests, acceptances, cancellations, ride lengths, etc.
* Build dashboards for:

  * Funnel conversion.
  * Driver utilization.
  * Safety incidents per 10k trips, etc.
* Run experiments:

  * Different pickup UI, pricing tweaks, driver incentives.
* Feed data back into:

  * Pricing, dispatch, fraud detection, safety models.

**2029 AI competence**

* **Almost fully automatable.**

  * Agents can design metrics, add instrumentation, write analytics queries, and even do causal analysis / A/B test evaluation.
  * Humans still set “what matters”: e.g., maybe you care more about **equitable service across neighborhoods** than raw revenue, which shapes metrics.

---

## 16. Specific integration points with ColumbiaOS

Here’s the “integration in excruciating detail” part: what exactly needs doing to blend ColumbiaRide with your existing city platform?

### 16.1 Shared identity and profiles

Tasks:

* Use ColumbiaOS user accounts for both riders and (optionally) drivers.
* Extend user schema:

  * Flags: `is_driver`, `driver_status`, `vehicle_info`, etc.
  * Preferences: “no shared rides,” “needs wheelchair access,” etc.
* Permissions model:

  * Who can see what information in the civic/charity/social parts vs the ride‑hailing part (e.g., you don’t want a driver’s home address visible in public forums).

**2029 AI competence**

* Very strong: schema design + migration + permissions logic is bread‑and‑butter for future agents.

### 16.2 Events & rides

Tasks:

* Every event in ColumbiaOS gets:

  * A “Get a ride” button.
  * Pre‑computed best pickup/drop‑off points.
* AI can:

  * Predict expected uplift in demand and adjust driver incentives or pre‑notify drivers.

**2029 AI competence**

* Trivial for AI to implement technically.
* Humans help pick which events to highlight, especially politically charged ones.

### 16.3 Charities & volunteer rides

Tasks:

* Link nonprofit profiles to:

  * Subsidized or free rides for volunteers/clients.
  * Donor‑funded ride pools.
* Track:

  * How many rides given, to whom (anonymized where needed).

**2029 AI competence**

* Technical integration: easy.
* Policy/ethics: up to you. AI can propose mechanisms, but you decide who gets subsidized.

### 16.4 Economic transparency

Tasks:

* Feed anonymized ride data into ColumbiaOS dashboards:

  * Heatmaps of demand by neighborhood/time.
  * Service level metrics: wait time, availability in poorer vs richer areas.
* Possibly expose:

  * Aggregated driver earnings distribution.
* Use this as part of your broader “economic coordination” vision.

**2029 AI competence**

* Data analysis and viz: highly automatable.
* Deciding what you expose, and how, is a political/economic decision.

---

## 17. So what exactly will 2029 AI **struggle** with here?

Summarizing the pain points:

1. **Legal personhood and liability**

   * AI can’t be sued, can’t go to jail, can’t hold a corporate officer role.
   * Governments and insurers will require a real person or corporation to be accountable.

2. **Regulatory negotiation and political games**

   * TNCs are politically sensitive. New rules are being considered all the time (quotas, fees, accessibility mandates). ([California Public Utilities Commission][11])
   * AI can prepare briefs, but actual politics involve nuance, relationships, and reputations.

3. **Trust & Safety in the messy real world**

   * Sexual assaults, harassment, discrimination, drunk drivers, minors in cars – these are not just “tickets in a queue.”
   * Human judgement, empathy, and local context will still be needed, at least as final arbiter.

4. **Deep local trust & adoption**

   * Your advantage over Uber would be: “We’re *of* Columbia, not just in Columbia.”
   * That includes face‑to‑face relationships, community meetings, and local media presence – hard for an AI to substitute.

5. **Economics of a local‑only marketplace**

   * Uber spreads its **fixed costs (insurance, legal, tech)** over millions of users and cities.
   * A local competitor, even with AI‑cheap tech, still has:

     * Regulatory costs.
     * Insurance premiums.
     * Fixed overhead for ops/support.
   * AI reduces some variable costs but doesn’t magically give you scale.

6. **Physical constraints**

   * Even if you had “super‑planner” AI, you still need enough drivers or vehicles, fuel, parking, etc.
   * This echoes a broader point many people make about full automation by 2030: **even if AI were ready, you still need time to deploy physical infrastructure** (power, robots, cars, etc.). ([Reddit][12])

---

## 18. Reconciling your intuition vs my skepticism

So, to answer your core challenge:

> “What do you think 2029 frontier AI would struggle with in planning and implementing a local Uber startup?”

I think by 2029:

* **Planning and coding the entire system** – from app UX to backend to pricing algorithms to analytics – will be **almost fully automatable** with frontier AI. That part I’m *not* skeptical about.
* The reasons I still describe this path as “brutal” are:

  * It’s a **tightly regulated, liability‑heavy, politically sensitive, trust‑critical business**.
  * It pits you against incumbents (Uber/Lyft or whoever is around then) who also have powerful AI, deep pockets, and regulatory experience.
  * For a city the size of Columbia, the **economic margin** may simply be thin relative to easier opportunities.

So my claim is not “AI won’t be able to do this,” but:

> In 2029, AI will make the *software* for a local Uber clone easy and cheap, which means your moat won’t be in the code at all. The hard parts are regulatory license, insurance, trust & safety, local politics, and marketplace economics in a small city – and those are still human‑heavy even with extremely capable AI.

If you want, we can next walk through a concrete “ColumbiaRide 2029” scenario: assume extremely strong AI and then simulate what year 1 of operations looks like in Columbia (driver numbers, prices, regulatory steps, political risks), and compare that to other wedges like your civic transparency tools.

[1]: https://kvssetty.medium.com/ai-code-generation-advancements-2025-edc885aecbc8?utm_source=chatgpt.com "AI Code Generation Advancements 2025 | by KV Subbaiah Setty"
[2]: https://www.techradar.com/pro/google-cloud-is-making-its-ai-agent-builder-much-smarter-and-faster-to-deploy?utm_source=chatgpt.com "Google Cloud is making its AI Agent Builder much smarter and faster to deploy"
[3]: https://www.businessinsider.com/sam-altman-predicts-ai-agi-surpass-human-intelligence-2030-2025-9?utm_source=chatgpt.com "Sam Altman predicts AI will surpass human intelligence by 2030"
[4]: https://law.justia.com/codes/tennessee/title-65/chapter-15/part-3/?utm_source=chatgpt.com "Tennessee Code Title 65, Chapter 15, Part 3 (2024)"
[5]: https://www.gao.gov/assets/gao-24-107093.pdf?utm_source=chatgpt.com "GAO-24-107093, Ridesharing and Taxi Safety"
[6]: https://www.capitol.tn.gov/Bills/114/Fiscal/SB0818.pdf?utm_source=chatgpt.com "SB 818 - HB 879 FISCAL NOTE - Tennessee General Assembly"
[7]: https://www.sfcta.org/projects/tnc-regulatory-landscape?utm_source=chatgpt.com "The TNC Regulatory Landscape"
[8]: https://law.justia.com/codes/tennessee/title-55/chapter-12/part-1/section-55-12-141/?utm_source=chatgpt.com "Tennessee Code § 55-12-141 (2024) - Automobile ..."
[9]: https://www.uber.com/us/en/marketplace/pricing/surge-pricing/?utm_source=chatgpt.com "Uber Marketplace Surge pricing"
[10]: https://publications.tnsosfiles.com/acts/109/pub/pc0520.pdf?utm_source=chatgpt.com "~tate of 'Qrennes.see"
[11]: https://www.cpuc.ca.gov/regulatory-services/licensing/transportation-licensing-and-analysis-branch/transportation-network-companies?utm_source=chatgpt.com "Transportation Network Companies"
[12]: https://www.reddit.com/r/agi/comments/1nlsyhn/ai_to_eliminate_99_of_jobs_by_2030_warns_top/?utm_source=chatgpt.com "AI To Eliminate 99% Of Jobs By 2030, Warns Top Expert: ' ..."

"


ME:
"
All right, I think I agree with your analysis that I should hold off on an Uber replacement for quite a while. It may be that in 2032 or soon thereafter some autonomous vehicle companies will be authorized to operate in Tennessee. Maybe then I could formulate a fleet company in which thousands of Columbia residents have ownership stakes and make small margins on every ride, in order to reduce the likelihood that wealthy people will form corporations to maximize the profit they make on people taking rides. Anyway, that's not something that I want you to opine on at length. 

However, I'm realizing that my financial outlook might be maximally successful if I can start by building a variety of relatively simple and small-scope businesses that are built off of the three general-purpose tools I discussed.

So here are some businesses: 

1) "Everything Columbia" 
Still not committed to this name, but this is what you have been calling ColumbiaOS. The obvious high-value features are an event calendar, a news aggregator (including news about local government meetings, decisions, legislation, etc.), a business inventory, and a general natural language chatbot feature that engages with all the data in the knowledge graph. This can be enhanced with personalized daily briefings for a premium cost ($8/month instead of $5/month for the basic plan), and there can be an eventual expansion into business integration for certain professions to help them fill their calendar (maybe start with photographers and salons/barbers). I figure 1,000 customers after 18 months of work is a reasonable target and would greatly enhance my income ($60,000 gross) while I keep my full-time job. 

2) Local government spending information 
I could probably sell continual meeting updates / contact changes for the state of Tennessee to suppliers for something like $300/month per supplier. Maybe I could provide the information by API? I might also be able to provide this information to GovWin and GovSpend for $1,000/month to help them supplement their own data that they supply to their customers. Is that realistic? 10 suppliers a month seems realistic after several months of work. That would increase my gross yearly revenue by $36,000.

3) Sermon transcription and RAG-assisted LLM Q&A for churches
Transcripts could be done at $0.50/ sermon, and something like $50/month for RAG-assisted Q&A. Is this competitive pricing? Maybe I could do this for 10 churches. That's $500/month, which increases my gross revenue by $6,000. 

4) Discourse navigator for YEC/Intelligent Design
I have a lot of interest in the Intelligent Design discussion, as someone who sees a great deal of evidence for evolution and little evidence for intelligent design behind intuition and a God-of-the-Gaps post hoc rationalization of data. Either way, I think proponents and opponents of ID would be interested in seeing a comprehensive and chronological depiction of the conversation, how it has evolved over time, and which questions and criticisms are left unanswered by each side. Additionally, all the relevant data could be referenced in one location, simplifying the investigating process for individuals. Maybe people could  pay $5/month to read these data. Perhaps a premium service that performs frequent meta-analyses and offers exclusive Q&A opportunities for experts in the field for $10/mo or $12/mo. I could easily see 1,000 users paying for the $5/mo version, which would increase my yearly revenue

5) Consulting for data engineering projects
- This year I made ~$19k doing consulting work. Now, it was pretty stressful, but that's because I engaged in bad development habits and wasn't selective with the tasks I accepted. Maybe I could do this more each year for the next couple of years, but be more skilled and disciplined in the way I conduct it. I could also advertise the products and services I specialize in (based on the general purpose products I sell) and split profits with people who bring potential clients to me (I do all the technical work and command 50%, while they do the majority of the PM work and command 50%; I would only do this for deliverables that are easily created as a byproduct of my general purpose products).


6) Redirecting my role at Sourcewell to more of an AI-focused specialty. 
- Maybe this could increase my salary from $128k now to $150k within 12 months. My company is getting into Databricks and I have already pioneered a couple of AI use cases at work that are . Sourcewell (the company I work for) offers excellent benefits and is slow to terminate their employees. While I am not strongly passionate about this role, I could be more interested with a slight change in focus and with more autonomy to direct my projects. Also, if I have additional revenue from side projects (everything not listed under this item), then I could maximize the amount of my salary that I withhold for things like my HSA and 401k using my benefits without suffering from low cash flow. I could also max out a Roth IRA. 

Are there any other obvious niches I can fill? The order in which I listed these ideas is not necessarily the order in which I would implement them. Please spare no details in your thoughts about these business ideas and other opportunities. 
"

AI:
"
You’re basically sketching a personal “AI conglomerate”: one shared tech core (scraper → transcription → knowledge graph → RAG) and a bunch of small vertical businesses hanging off it.

That’s a very good instinct.

I’ll walk through each of your six ideas in detail, with:

* What the business actually is (in plainer terms)
* How it ties into your general‑purpose tools
* Realistic pricing / adoption, using current market benchmarks
* Hidden risks (legal, operational, attention, conflicts of interest)
* How I’d prioritize it in your portfolio

Then I’ll add a few other niche ideas that naturally fall out of the same stack and fit your constraints (full‑time job, arthritis, desire to help “normal people” coordinate).

---

## Quick context: your market + tool stack

**Columbia, TN**

* Population ≈ 49k–50k, growing ~3–3.5%/year. ([World Population Review][1])
* Part of greater Nashville area; relatively small but fast-growing.

So any *purely local* consumer product has a hard cap of maybe 20k–30k reachable adults, and getting 1k+ of them to pay for a digital service is *possible* but non-trivial.

**Your core tools (super important)**

1. **Scraper + crawler** that builds a graph of local websites, events, business listings, government pages, etc.
2. **Transcription pipeline** that can ingest large volumes of audio/video (sermons, gov meetings, etc.).
3. **Knowledge graph + RAG** (“retrieve‑then‑ask‑the‑LLM”) on top of all that content.

Everything you listed is some combination of:

* “Sell structured data or access to the graph,”
* “Sell an interface powered by the graph,” or
* “Sell your expertise building and running such graphs.”

So think of yourself less as “a guy with six separate ideas” and more as “a one‑person vertical AI infra company with multiple front‑ends.”

---

## 1) “Everything Columbia” (ColumbiaOS)

### What it really is

A **local information + coordination layer** for Columbia:

* Event calendar
* Business directory
* Local news and government decisions aggregator
* Natural-language Q&A over all that data (“What free, family‑friendly events this weekend?”)

Then:

* Premium: personalized daily briefings + extras.
* Later: business integrations (e.g., photographers / salons plugging in calendars, deals, etc.).

You’re imagining 1,000 paying customers at ~$5/month in ~18 months → ~$60k/year gross, on top of your job.

### How this fits the tool stack

* Scraper: pulls in events, business pages, city website updates, local news articles, school calendars, etc.
* Transcription: converts gov meeting videos, maybe town hall live streams, to text.
* Knowledge graph: entities = people, organizations, venues, events, ordinances, contracts, etc.
* RAG: Q&A for residents on top of that graph.

So this is the **perfect first “sandbox”** for your three tools.

### How realistic are 1,000 paying users?

Some relevant signals:

* Hyperlocal news and newsletters *can* work, but often need a mix of ads, sponsorship, and memberships, not just subscriptions. ([INMA][2])
* Patch used AI newsletters to jump from 1,100 to 30,000 U.S. communities; they’re treating themselves as a “hyperlocal information platform,” not purely a news subscription business. ([Axios][3])

For a city of ~50k, a **2–3% paying penetration** (1,000–1,500 subs) is ambitious but not absurd *if*:

* The product is clearly valuable,
* You have strong local distribution (city gov, chamber, churches, schools, Facebook groups), and
* You are the **default place** people go for “What’s happening in Columbia?”

But getting there takes time and grind. I would sanity‑check like this:

* Year 1: Aim for **100–300 paid** subs + a larger free tier (1–5k signups).
* Year 2: If you’re actually hitting >500 paid and retention is solid, 1,000+ becomes plausible.

### Pricing and structure tweaks I’d recommend

Instead of just “$5 vs $8,” think in **tiers and bundles**:

1. **Free tier (sponsored):**

   * Basic calendar, directory, and some news.
   * Email newsletter / daily briefing with sponsor ads.

2. **Resident premium ($5–7/month):**

   * Ad‑light or ad‑free.
   * Personalized daily briefings (events, gov decisions, business promos tuned to them).
   * Extra tools: saved searches, alerts (“notify me if new family‑friendly events appear this weekend”).

3. **Business / org tier ($20–100/month):**

   * Richer listings (photos, promos, deals).
   * Integration with their booking calendars (photographers, salons, therapists).
   * Placement in recommendation slots (“book a local photographer for this event”).
   * Simple analytics: how often they show up in searches, click‑throughs, etc.

4. **Civic / nonprofit tier (heavily discounted or free):**

   * Tools for charities and city departments: event promotion, volunteer signups, etc.

This spreads revenue across:

* Residents,
* Businesses,
* Local government / nonprofits (possibly, via contracts down the line).

### Risks and hidden work

* **Content moderation & toxicity:** Nextdoor‑style hyperlocal platforms can get messy (complaints, racism, neighbor disputes). Moderation is a serious time sink; AI helps but doesn’t solve it. ([Oncely][4])
* **Maintaining data freshness:** Calendars, business hours, addresses, closings – lots of churn. Your scraper + some human validation must be robust enough that people trust the info.
* **Competition for attention:** Facebook groups, Google Maps, Nextdoor, Patch AI newsletters, etc.

### Overall verdict

* **Strategic value:** Very high (uses your stack, deepens local relationships, yields reusable local datasets).
* **Direct income potential:** Medium, but could reach your $60k/year number over a few years if successful.
* **Recommended priority:** **Top‑3 priority.** I’d treat this as your main “product bet” if you want to be embedded in Columbia long‑term.

---

## 2) Local government spending information (Tennessee → vendors & big platforms)

### What you’re imagining

* You continuously monitor **Tennessee state + local gov** (meetings, budgets, RFPs, contracts, contact changes).
* You sell:

  * To **suppliers directly** at ~**$300/month per supplier**.
  * Potentially to **GovWin, GovSpend, etc.** at ~$1,000/month for your TN coverage.

You think 10 suppliers at $300/mo = $36k/year additional gross.

### Where the market is today

GovWin IQ and GovSpend are **high‑priced platforms**:

* GovWin IQ subscriptions typically run from **$13k to >$100k per year**, depending on package and size. ([Vendr][5])
* GovSpend offers procurement data and lets customers request additional data/refreshes as part of their subscription. ([GovSpend][6])

Other players (Civic Sunlight, Citizen Portal, ClerkMinutes, Curate, etc.) already use AI to capture and analyze local meetings & legislation. ([Civic Sunlight][7])

So:

* The **macro idea (AI‑powered local gov data)** is absolutely real and in demand.
* Big players already do national scale, but often have “holes” in their coverage, especially smaller local governments.

### Concrete opportunities for you

1. **Direct vendor subscriptions (your $300/month idea)**

   * For a vendor selling to TN/local govs, paying **$300/month** for:

     * Fresh contract awards,
     * New RFPs,
     * Decision timelines,
     * Key contacts,

     is totally realistic if you’re providing *actionable and timely* leads.

   * In practice, B2G/B2B SaaS at this price point is quite standard.

2. **Per‑dataset or per‑region deals with big platforms**

   * GovWin/GovSpend *might* be willing to pay for specialized coverage (e.g., “we’ll pay you X to keep these 30 TN counties and 100 cities fresh and structured”), but:

     * It’s more likely to be **project or per‑deliverable based**, not a neat $1k/month retainer.
     * They already offer custom data sourcing for their customers. ([support.govspend.com][8])
   * A more realistic path:

     * Start by selling to vendors directly, prove out quality.
     * Use that success to pitch partnership or white‑label feeds.

3. **Licensing to newsrooms and civic orgs**

   * There’s rising interest from news orgs in AI tools for meeting transcription and legislative tracking (Houston Chronicle’s Meeting Monitor, Hearst’s Assembly, USLege, etc.). ([Houston Chronicle][9])
   * You could sell your structured TN data to:

     * A local investigative newsroom,
     * State‑level civic transparency NGOs,
     * University research centers.

### Huge, non‑negotiable caution: conflict of interest with Sourcewell

Sourcewell is a **cooperative purchasing organization** that already sits at the center of government procurement, contracts, and suppliers. ([Sourcewell][10])

If you:

* Work there in a role involving data/analytics on contracts, AND
* Build a side business that sells **government procurement intelligence to vendors**,

then there is a very real risk of:

* Violating internal policies on side businesses,
* Conflicts of interest (even if you’re not literally re‑using their data),
* At the extreme, legal or reputational problems for you and them.

So if you pursue this:

* **Do not use any internal Sourcewell knowledge, tools, or contacts** for your business.
* **Read your employment agreement** for non‑compete / conflict clauses.
* Strongly consider **talking to HR or your manager** at a high level (without giving them your secret sauce) to check what’s allowed. I can’t give you legal advice, but I would not treat this lightly.

### Overall verdict

* **Strategic value:** Very high if you want to be in the govtech/B2G data space.
* **Income potential:** Quite good – $36k/year with ~10 vendors is plausible if your data is genuinely useful.
* **Risk:** The conflict‑of‑interest dimension is your biggest non‑technical risk.
* **Recommended priority:** **Top‑2 or Top‑3**, *if* you can navigate the employer conflict safely. Otherwise, treat as a later pivot when/if you change jobs.

---

## 3) Sermon transcription + RAG for churches

You proposed:

* $0.50 **per sermon** for transcripts,
* $50/month per church for RAG/Q&A,
* 10 churches → $6k/year.

### Current market reality

There is now a *lot* of competition:

* Traditional transcription services charge **$0.75–$3.99 per audio minute** for sermons (i.e., $22–$180 for a 30–45 min sermon), depending on manual vs AI quality. ([https://scriptscomplete.com/][11])
* Automated tools (Temi, etc.) can be as low as **$0.25/minute**. ([whispertranscribe.com][12])
* AI church tools like **Pastors.ai**, **Kodesiq**, and similar are already repurposing sermons into studies, clips, and chatbots, often in the **$20–$100/month** range for smaller churches. ([Kodesiq][13])

So:

* **$0.50 per sermon** is not just low, it’s almost zero (if a sermon is 30–45 mins, that’s about **$0.01–0.02 per audio minute**). You’d lose money on compute and your own time.
* $50/month for RAG/Q&A is **within** market range – Pastors.ai has a $20/mo individual plan and higher tiers, Kodesiq charges based on capacity. ([Kodesiq][14])

### Where you can be differentiated

You’re not just “another transcript vendor.” You could be:

1. **An opinionated theological search system.**

   * Many AI tools for churches are generic; you could position as:

     * “Our system only answers from *your* sermons and statement of faith.”
     * Strict guardrails against hallucinating doctrine.

2. **Data/analytics as a service.**

   * Trends over time: topics, scripture passages, repeated themes.
   * Tools for sermon series planning, measuring coverage of different books/topics.
   * Member Q&A logs to see what people are confused or curious about.

3. **Multi‑church “discourse map” (especially if you tie into your ID/evolution interest).**

   * Compare how different denominations/teachers talk about certain doctrines.
   * But this gets complicated politically; you’d need to position it carefully.

### Realistic pricing model

If you keep this niche, I’d do something like:

* **Tiered SaaS per church**:

  * Basic: **$30–$40/month**

    * Automated transcripts (via your pipeline).
    * Simple search and Q&A, branded page.

  * Plus: **$60–$100/month**

    * Better analytics (topic coverage, trending questions).
    * Multiple services/groups.
    * Priority support.

And let automated transcription pricing be absorbed into that subscription (no per‑sermon fee).

At that price, 10 churches would be **$3,600–$12,000/year** – still not huge, but not trivial, and it reuses your existing transcription + RAG tools.

### Is this worth your energy?

Pros:

* Very close to your existing toolset.
* Probably not too hard technically.
* Connects to your interest in religious discourse.

Cons:

* Crowded space; many tools are racing here.
* Sales motion is “call lots of pastors,” which can be emotionally draining and slow.
* Many small churches are budget‑constrained and tech‑conservative.

**Verdict:** This is a decent **side experiment** or add‑on product, not your main bet. I’d treat it like:

> “When my transcription + RAG infra is solid, I’ll pilot it with 1–3 churches I know and see if this grows organically. If it does, great; if not, I let it be a niche income stream.”

---

## 4) Discourse navigator for YEC/Intelligent Design vs evolution

### What you’re imagining

* A **chronological, structured, queryable map** of the creation/evolution / ID debate:

  * Claims, counterclaims, unanswered questions, references.
  * Possibly annotated with “which side has addressed this, how well, and where there are gaps.”
* Monetized via:

  * $5/month basic access,
  * $10–$12/month premium (meta‑analyses, expert Q&As, etc.).
* You imagine 1,000 users paying $5/month → $60k/year.

### Market sanity check

* Creationist / YEC belief is still significant: roughly **17–37% of Americans** depending on question phrasing. ([Gallup.com][15])
* That’s tens of millions of people potentially on the “pro‑ID/YEC” side, plus many people on the “skeptic” side. So **interest in the topic is large**.

But:

* Most of those people consume free content (YouTube debates, blogs, free newsletters).
* Paying $5–$12/month is something a much smaller subset will do, especially for a **single-issue** site.

1,000 paying users worldwide is not impossible, but you’d probably need:

* Very high‑quality curation and neutral-ish presentation (so both sides feel it’s “fair enough”).
* Some form of community or live interaction (forums, events) so it’s not “just another static site.”
* A long tail of SEO traffic – forums and niche sites are increasingly prioritized by search engines if well‑run. ([Search Engine Land][16])

### Risks

* **Moderation & polarization:** these debates can get heated; you’d inevitably attract both sincere people and trolls.
* **Content policy / platform risk:** big AI models + platforms are cautious about sensitive religious/political content; this may impact integrations, monetization, or SEO over time.
* **Time sink:** you’re effectively becoming a specialized **knowledge curator + community manager** for a highly contentious topic, which can consume a lot of mental energy.

### Upsides beyond money

This idea may have:

* Huge **personal meaning** for you, since you care about the topic.
* Spillover benefits to your tech stack:

  * It’s a non‑local “discourse navigator” pattern you can reuse for other controversial topics (housing, zoning, etc.) on ColumbiaOS.

### My take

* I wouldn’t bank on 1,000 paying users quickly; I’d model **100–300 payers** as a realistic upper bound in the first couple of years if you execute well (so ~$6k–$36k/year).
* I’d frame it as a **long‑term intellectual project** that maybe becomes modestly profitable, rather than as a core income pillar.

**Verdict:** Worth doing *if and only if* it brings you intellectual satisfaction. From a pure financial perspective, it’s riskier and less directly synergistic than your Columbia/government ideas. Keep it as a “Phase 2 passion project” once your core infra and income are more stable.

---

## 5) Data engineering consulting (more intentional and AI‑focused)

You made ~$19k in consulting this year, under suboptimal conditions (stress, bad habits, poor filtering). You’re thinking of:

* Being more selective,
* Focusing on projects that are **by‑products** of the tools/products you’re building,
* Possibly splitting revenue 50/50 with a PM/business‑dev partner who brings in clients.

### Why this is powerful short‑term

* You’re in a moment where **AI + data engineering is scarce and expensive**.
* If you reposition yourself as a **specialist** in:

  * “AI‑ready data pipelines,”
  * “Meeting/sermon/video → transcripts → knowledge graph → Q&A tools,”
  * “Local gov procurement intelligence,”
    then you can charge significantly higher rates and keep your project work tightly aligned with the products you’re building anyway.

### How to make it less stressful

1. **Productized services**

   * Instead of “I do anything data-ish for $X/hour,” you sell packages like:

     * “$5k: build you a prototype meeting‑transcription + dashboard pipeline for one government body.”
     * “$3k: integrate AI search over your existing document repository (up to N docs).”
   * This aligns well with your general‑purpose tools; you’re mostly reusing them.

2. **Strict scoping and filtering**

   * Only take projects that:

     * Can be done mostly remotely;
     * Don’t require you to maintain legacy tech stacks you hate;
     * Reuse modules you want to improve anyway.

3. **Use AI heavily internally**

   * 2025–2030 is exactly the window where your own internal use of AI coding and planning agents can 2–3x your hourly throughput.

### Realistic revenue

* If you currently do $19k with “bad habits,” you could easily turn that into **$30k–$50k/year** of consulting with:

  * slightly higher rates, and
  * services that reuse your ColumbiaOS/government stack.

But don’t forget:

* **Time & energy spent here is not spent on your products.**
* You’re working full‑time + side projects + consulting; your back and brain have limits.

**Verdict:** This is a very good **income stabilizer** and a way to bootstrap your tools – as long as you are ruthless about only taking work that builds your long‑term capabilities and not just “whatever pays today.”

---

## 6) Evolving your Sourcewell role into an AI‑specialist position

You’re at ~$128k salary there, with strong benefits and high job security. You think:

* You could move into a more AI‑focused specialist role and get to ~$150k within 12 months.
* Sourcewell is starting with Databricks and you’ve already pioneered some AI use cases. ([Sourcewell][10])

### Why this is a big deal for your overall plan

* That stable income + benefits is the **engine** that funds:

  * Your personal runway,
  * Retirement accounts (401k, Roth IRA, HSA),
  * Product experiments that won’t pay off right away.

If you can:

* Rebrand internally as **“the AI/data lead”** for certain initiatives,
* Own visible wins (e.g., an AI‑assisted procurement insights tool, or internal document search),
* Move up to $150k+,

then you’ve:

* Raised your baseline lifestyle security,
* Increased your market value as an employee,
* Built exactly the skills your side businesses need.

### How to do this deliberately

* Propose **small, high‑impact pilot projects**:

  * AI search over cooperative contracts.
  * Vendor intelligence dashboards.
  * Transcription + summarization of internal or public meetings that matter to members.
* Document and evangelize successes.
* Position yourself as the person who can safely bring AI into their workflows (privacy, governance, etc. – see the AI governance docs states are issuing, e.g. Maryland’s AI governance card for transcription tools). ([doit.maryland.gov][17])

**Verdict:** I’d consider **maximizing this** a foundational plank of your 5‑year plan. It’s not exciting like launching a startup, but it’s what lets you take risk elsewhere without financial panic.

---

## 7) How these ideas fit together (and what’s missing)

Let’s zoom out.

You essentially have four “families” of opportunities:

1. **Local Columbia stack (Everything Columbia + future civic / fleet ideas).**
2. **Tennessee / gov data stack (spending, meetings, vendor intelligence).**
3. **Religious content stack (sermons, ID discourse navigator).**
4. **Your career & consulting (Sourcewell AI role + productized consulting).**

### Natural additional niches (using exactly the same tools)

Here are a few that fit your interests and constraints:

---

### A) “Meeting Monitor for Tennessee” – starting with a region

Inspired by what Houston Chronicle and Hearst are doing for school boards: ([Houston Chronicle][9])

* Build a **public portal** that:

  * Ingests city council + county commission + school board meetings across a set of TN counties.
  * Provides searchable transcripts, AI summaries, and topic alerts.
* Revenue options:

  * Sell “pro” dashboards to journalists, NGOs, and vendors.
  * Offer white‑label access to local papers that can’t afford their own AI dev team.
  * Entry/bridge product for your gov data business (#2).

This is basically your **“Columbia Civic Lens”** idea, scaled to a region.

---

### B) FOIA & public‑record automation service

Using your scraper and transcription stack, you could:

* Offer **automation + summarization** of FOIA responses or public document dumps for:

  * Local journalists,
  * Civic nonprofits,
  * Law firms / watchdog groups.
* Think:

  * “Upload 10GB of PDFs of contracts and emails; get a structured index + summary Q&A interface.”

This is exactly where AI shines, and it’s still under‑served for smaller orgs.

---

### C) Local housing & property concentration explorer (for Columbia first)

You care about landlord concentration and home ownership, so:

* Aggregate:

  * Property tax rolls (public in many jurisdictions),
  * Sale histories,
  * Zoning and permit data,
  * Rental listings.
* Build tools that:

  * Show concentration metrics (“Top 20 owners in Columbia and how many properties they hold”).
  * Map areas where rents are surging vs incomes.
* Monetize:

  * Sell deeper analytics to local realtors, developers, or advocacy orgs.
  * Use as a lead‑in to your ColumbiaOS premium (“Get voting alerts when policies affecting your neighborhood’s housing come up”).

This is also politically sensitive, but you can start with **neutral transparency**.

---

### D) “Knowledge navigator” as a general product

The ID/YEC discourse navigator can be the **first vertical** of a general “controversy navigator” product:

* Tooling to:

  * Crawl a topic,
  * Cluster arguments and citations,
  * Provide timelines and unresolved questions.
* Vertical markets you could later target:

  * Tech & AI policy debates.
  * Local land‑use disputes.
  * Climate policy.

You may or may not want to generalize, but knowing that your ID project could spawn a more general “knowledge navigator” product might make it feel more strategically justified.

---

## 8) Prioritization and sequence (my suggested roadmap)

You can’t do everything at once, especially with a full‑time job and health constraints. Here’s a pragmatic ordering for the next 2–3 years.

### Step 1 (0–6 months): solidify income + build core infra

1. **Push Sourcewell role toward AI specialty (#6).**

   * Identify 1–2 AI projects internally and make them your thing.
   * This increases your salary odds and aligns your day job with your side projects.

2. **Build v0 of your core tools with a very narrow scope:**

   * Scraper that ingests:

     * Columbia city site,
     * Maury County site,
     * A few key local orgs,
     * 1–2 churches,
     * 1–2 gov meeting video sources.
   * Transcription pipeline for those meetings/sermons.
   * Knowledge graph + simple RAG search over that content.

3. **Do limited, high‑leverage consulting (#5).**

   * Only take consulting that forces you to sharpen these tools.

### Step 2 (6–18 months): launch one flagship product + one B2B stream

Pick **one flagship** to get out in the world:

* Either:

  * **Everything Columbia v1** with:

    * Events calendar,
    * Business directory,
    * Local gov meeting search/summaries,
    * Daily email (free) + simple premium.

* Or:

  * **“TN Meeting/Spending Monitor v1”**:

    * Focus on one or two Tennessee counties,
    * Offer vendor‑focused alerts + dashboards at ~$200–$400/mo.

You can *kind of* do both if you treat Columbia as your “beta” for the state‑level product, but I’d still choose which one you’re optimizing for in your head.

During this phase:

* Keep building AI credentials at Sourcewell.
* Refine consulting offerings to match your infra.

### Step 3 (18–36 months): diversify + passion projects

Once:

* Your core infra is stable,
* One product has paying customers,
* Your day job is in a better place financially,

then you can:

* Add **sermon + Q&A** for 1–3 churches that know you personally.
* Start building the **ID/YEC discourse navigator** in your spare intellectual cycles.
* Experiment with **housing/property concentration tools** in Columbia.
* Decide, based on traction, whether:

  * To double down on one product as a real startup, **or**
  * To keep a portfolio of small, steady side businesses plus a strong career.

---

## 9) Financial outlook across your portfolio (high‑level)

Here’s a *conservative* medium‑term scenario (all annual figures, maybe ~3–5 years out):

* Sourcewell salary after AI specialization: **$150k**
* Consulting (productized, selective): **$20–40k**
* Everything Columbia (mixed subs + business fees): **$10–30k**
* Gov data products (vendors in TN, small number): **$20–50k**
* Sermon SaaS + ID navigator etc.: **$5–15k**

Total gross: **~$205k–$285k/year**. Obviously lots of uncertainty, but even the low end is strong for a single‑income household, and the stack is diversified.

The key is not that any one of these hits it huge; it’s that:

* Each line is **powered by the same underlying infra and expertise**, and
* You’re steadily converting your time and AI leverage into **multiple income streams + assets** resilient to any one job or product failing.

---

If you’d like, next we can:

* Go extremely concrete about one specific path (e.g., “Everything Columbia v1” architecture, pricing tiers, and a 90‑day launch plan), **or**
* Take one of the B2B ideas (TN meeting/spending monitor) and outline a “10 customers in 12 months” playbook.

[1]: https://worldpopulationreview.com/us-cities/tennessee/columbia?utm_source=chatgpt.com "Columbia, Tennessee Population 2025"
[2]: https://www.inma.org/blogs/big-data-for-news-publishers/post.cfm/automated-hyper-local-newsletters-are-low-maintenance-with-high-engagement?utm_source=chatgpt.com "Automated hyper-local newsletters are low maintenance ..."
[3]: https://www.axios.com/2025/03/04/patch-news-ai-newsletters-local-communities?utm_source=chatgpt.com "Patch scales to 30,000 U.S. communities with AI newsletters"
[4]: https://oncely.com/products/pastors-ai-discount-deal?utm_source=chatgpt.com "Pastors.ai"
[5]: https://www.vendr.com/buyer-guides/govwin-iq?utm_source=chatgpt.com "GovWin IQ Software Pricing 2025"
[6]: https://govspend.com/?utm_source=chatgpt.com "GovSpend: B2G Intelligence for the Public Sector"
[7]: https://civicsunlight.ai/?utm_source=chatgpt.com "Civic Sunlight - Transforming local government coverage with ..."
[8]: https://support.govspend.com/data-overview-spending-pos?utm_source=chatgpt.com "Data Overview: Spending & POs"
[9]: https://www.houstonchronicle.com/about/newsroom-news/article/houston-meeting-monitor-tool-helps-keep-tabs-20797175.php?utm_source=chatgpt.com "Houston Chronicle debuts Meeting Monitor tool to help keep tabs on school districts"
[10]: https://www.sourcewell-mn.gov/cooperative-purchasing?utm_source=chatgpt.com "Cooperative Procurement & Contract Purchasing I ..."
[11]: https://scriptscomplete.com/sermon-transcription-services.php?utm_source=chatgpt.com "Sermon Transcription Services for Churches & Ministries"
[12]: https://www.whispertranscribe.com/blog/sermon-transcription-services?utm_source=chatgpt.com "Top Sermon Transcription Services for 2025"
[13]: https://www.kodesiq.com/?utm_source=chatgpt.com "Kodesiq - AI-Powered Sermon Q&A for Churches"
[14]: https://www.kodesiq.com/pricing?utm_source=chatgpt.com "AI-Powered Sermon Q&A for Churches - Kodesiq"
[15]: https://news.gallup.com/poll/647594/majority-credits-god-humankind-not-creationism.aspx?utm_source=chatgpt.com "Majority Still Credits God for Humankind, but Not Creationism"
[16]: https://searchengineland.com/forums-google-adapt-444065?utm_source=chatgpt.com "The rise of forums: Why Google prefers them and how ..."
[17]: https://doit.maryland.gov/policies/ai/Pages/AI-governance-card-call-recording-transcription-tools.aspx?utm_source=chatgpt.com "AI-Powered Call Recording & Transcription Tools - AI"

"

ME:
"
Thanks for all of those thoughts! I think I want to keep brainstorming at a somewhat general level for now. Some scattered considerations below:

I'm never doing ads in my products. Get that idea as far from your mind as possible. Commit this to memory: ads in my products = never. Only ads FOR my product externally, such as advertising my products on Google search pages.

Another thought is that it seems relatively easy to expand my Everything Columbia stack to other medium-sized communities (Murfreesboro, Franklin, Clarksville, Shelbyville) once I have proved out the platform in Columbia. Could I also have business-to-business features on this platform, such as identifying suppliers across the state and thereby strengthening supply chains?

I'm very interested in "Patch", mentioned in the Axios reference. Please tell me all about their business model.

Another form of revenue generation that seems increasingly feasible is quick one-time software or data deliverables. This thought is less developed in my mind, but I could imagine something like creating a fine-tuned ML or AI model for someone based on data they've gathered, and selling that for a few thousand bucks. Or maybe a fun little desktop game that people pay for once for like $10 and it only takes a few total hours of coding to make and almost nothing to serve. Or I've seen things like Excel templates sold in considerable volumes at prices between $5-$10. That seems less likely to be useful as software is progressively automated. Or maybe very niche Chrome extensions that I sell for a few dollars a piece and only take a few hours to create and require essentially no maintenance (on that note, what about an extension that you can click on in order to give the user the ability to do a single left-click on any element on a webpage and have it copied to a predetermined location on their desktop; that seems helpful to me; or what about a Chrome extension that allows you to see if the word you tried on Wordle has already been used in previous Wordles). Levelsio on Twitter is famous for having created dozens of software products, most of which never took off. But some of them unexpectedly did amazing numbers and then he was able to double down on those. It seems like being prolific is a good way to do guarantee success, and the act of continually generating ideas and bringing them to market builds great revenue-building muscles. I could even build meta-systems that make the creation of these products quicker. What do you think about that? Please spend a major fraction of your response on this subject.

Another general purpose tool that would be useful for me as an internal tool would be a system of information gathering via AI tools. For all of my ideas, I would want an AI to search online for new information regarding competitors, relevant legal developments, relevant technological innovations, relevant economic influences, and relevant cultural trends. The  idea of finding NEW information of this sort implies the storage of information that has been viewed and processed already so that subsequent AI efforts don't produce duplicate/non-novel information.

Finally, what sort of system or collection of design patterns is ideal in the creation of the three general purpose tools we discussed above? Three separate codebases, with data integrated between via serverless functions interacting with blob storage? 
"

AI:
"
Got it on the **no ads inside your products**. I’ll treat your whole universe as subscription / one‑time / B2B only.

I’ll hit these in order:

1. Expanding “Everything Columbia” to other cities + B2B/supply-chain angle
2. Patch’s business model and what’s interesting there
3. The “many tiny products” strategy (big chunk of answer, per your request)
4. An internal AI intel-gathering system for your ideas
5. Architecture / design patterns for your 3 general-purpose tools

---

## 1. Scaling “Everything Columbia” + B2B/supply-chain features

Once you have a robust “Everything Columbia” stack, you’re basically holding:

* A **local entities graph** (people, orgs, businesses, venues, events, gov bodies).
* Pipelines for **scraping, transcribing, and structuring** local data.
* A **query layer** for residents.

Cloning that to **Murfreesboro, Franklin, Clarksville, Shelbyville** is mostly:

* Pointing your ingest layer at new city sites and business directories,
* Adjusting a handful of city-specific rules,
* Spinning up new “communities” on the same infra.

If each city app is a separate *tenant* on a shared platform, you get:

* Economies of scale on infra and dev.
* A **statewide economic graph** for Tennessee almost for free.

Then B2B / supply-chain features are just “zoomed out” views of that same graph:

* “Show me all manufacturers within 150 miles of Nashville that do metal fabrication and have <50 employees.”
* “Which small logistics companies in TN work with at least 3 city governments?”
* “What local suppliers near Columbia could substitute for supplier X if they go down?”

That turns into products like:

* Dashboards for **regional economic development agencies** and chambers.
* Tools for **mid-size manufacturers** trying to localize supply chains.
* APIs for **larger procurement platforms** that want more granular local supplier coverage.

So yes: your Everything Columbia tech naturally extends not just to more towns, but also to **statewide B2B mapping**. The key constraint there won’t be the tech; it’ll be your sales bandwidth and access to decision-makers.

---

## 2. What Patch actually does (business model)

Patch is your closest “spirit cousin,” so understanding them is worthwhile.

### High-level

* Patch Media runs Patch.com, a **hyperlocal news and information platform** in the U.S., founded in 2007. ([Wikipedia][1])

* Historically:

  * Free access to local news + community posts for ~1,200–1,900 communities.
  * Revenue primarily from **local and national advertising**, including a self‑serve ad product for businesses. ([Wikipedia][1])
  * They’d been profitable for several years by 2018–2019, with ~$20M+ annual ad revenue and ~23M monthly visitors. ([Wikipedia][1])

* They experimented with **memberships** (“founding members” at $5/mo or $50/yr) that offered ad‑lite experiences and some status on the site. ([Vox][2])

So the traditional Patch model is:

> Free hyperlocal info → tons of pageviews → **ad revenue + a small layer of paid membership**.

That’s the part you *don’t* want to copy (ads).

### The AI newsletter pivot

More recently (late 2024–2025), Patch launched a big **AI newsletter push**:

* They built AI-generated local newsletters using vetted sources, including their own content. ([Axios][3])
* That took them from ~1,100 communities to **30,000 U.S. communities** in just a few months. ([Axios][3])
* Of ~3M newsletter subscribers nationally, ~400k subscribe to the new AI products. ([Axios][4])
* This pivot effectively makes them more of a **“hyperlocal information platform”** than a traditional news outlet, directly competing with things like Nextdoor. ([Axios][3])

The AI layer improved reach and revenue, but the fundamental business is still:

* Free products,
* Ad-heavy monetization,
* Membership as a sidecar.

**Takeaway for you:**

* *Ignore* their ad monetization.
* *Study* their **information architecture** and **AI-driven scaling**:

  * They’re proof that you can **automate local info aggregation at massive scale.**
  * You’re aiming for **fewer cities, higher ARPU, no ads** – more like a “local operating system” with subscription and B2B revenue.

---

## 3. The “many tiny products” strategy (and how to do it without frying yourself)

This is the big piece you asked for.

You’re basically thinking like Pieter Levels / indie hackers:

> Build lots of small products, most will fail quietly, a few will succeed big enough to matter. Use AI + meta‑systems to make shipping cheap and frequent.

### 3.1 Why a prolific micro-product strategy works at all

This is just **optionality**:

* Outcomes follow a **power law** – a few hits bring in disproportionate revenue, but you can’t reliably predict which ones upfront.
* The best way to “hack” an unpredictable environment is to **increase the number of reasonably‑informed bets.**

Pieter Levels’ public strategy:

* He did “12 startups in 12 months” and has built 40+ products overall; only a handful like RemoteOK and Nomad List became big successes. ([@levelsio (Pieter Levels)][5])
* Those now generate something like **hundreds of thousands a month** in revenue combined. ([Medium][6])

For you, the version is:

* You already have high‑value skills and AI tools.
* Many products can be built in **hours or days**, not weeks.
* Even “meh” products teach you:

  * What resonates,
  * Where the friction is,
  * How to pipeline idea → launch → feedback.

The risk is **fragmentation**: you scatter your energy and never build durable assets. So you need guardrails.

I’ll assume you’re aiming at: **5–15 serious micro‑bets over a few years**, not literally 12 a year.

---

### 3.2 Categories of micro-products you mentioned

Let’s walk them:

#### A. Custom ML / AI model fine-tunes

This is basically **productized consulting**:

* Input: client’s proprietary data (support tickets, docs, logs, niche texts, etc.).
* Output: fine‑tuned model or RAG pipeline tailored to them.

Pros:

* You can charge **thousands, not tens** of dollars.
* Uses exactly your stack: ingestion, cleaning, training, deployment.

Cons:

* You owe **support**: bugs, drift, data refresh.
* Legal overhead: NDAs, data privacy, liability if outputs cause issues.
* Scope creep: “Oh, can you also integrate it into our CRM / Slack / website?”

How to make this micro-product-like:

* Offer **fixed-scope packages**:

  * “$3k: fine‑tune an instruction model on up to X docs; basic evaluation; deploy as API or in their environment.”
  * “$5k: set up a custom RAG chatbot with their docs + admin panel.”

* Use your internal meta‑system so:

  * Every project uses the same training pipeline.
  * Every deployment uses the same scaffolding (Docker, CI, logging).

Even 3–5 of these per year = **$10k–$20k+** incremental income and refining your tools.

---

#### B. Tiny desktop / web games

A $10 game you build in a weekend is definitely possible, but:

* Distribution is the main beast. Steam/itch.io are crowded.
* The expected value is low unless:

  * You piggyback on a trend (e.g., Wordle clones at its peak), or
  * You have a unique, viral mechanic or strong audience.

I’d treat games as **creative play** that might randomly pay off, not as a primary micro‑product lane – unless you discover that:

* You *love* building them, and
* Your games actually get traction.

---

#### C. Templates (Excel, Notion, etc.)

It’s true GA default: “templates are everywhere,” but they still sell well when:

* They solve a **very specific operational problem** in a niche,
* For a **buyer that values time saved over money**.

Examples that would align with your skills:

* A “Local Government RFP Tracker” Excel/Notion template tuned to TN procurement processes.
* A “Church sermon + Q&A analytics” dashboard template.
* A “Side‑project idea pipeline” Notion template for indie devs.

Platforms like Etsy, Gumroad, and dedicated template marketplaces still move lots of these at **$5–$30 price points**, especially when bundled with a short video or guide.

But as LLMs get better at generating spreadsheet logic, the **moat will shift** from formulas to:

* Domain‑specific structure,
* Good defaults,
* Actual subject-matter insight.

So templates can be nice **side income** and top‑of‑funnel for your more complex offerings, but I wouldn’t over‑invest in them.

---

#### D. Niche Chrome extensions

These are a really good fit for your “many small bets” strategy.

Key realities:

* Chrome Web Store’s own payment system was **deprecated** in 2020. You now need to integrate your own billing (Stripe, Lemon Squeezy, etc.) and gate features based on external licenses. ([Chrome for Developers][7])
* People do successfully charge **one‑time or subscription fees** for extensions by:

  * Requiring an account and checking subscription status with their backend, or
  * Selling one‑time license keys used by the extension to unlock full features. ([Reddit][8])

Good extension ideas are:

* **B2B-ish utilities** that solve painful micro‑frictions for knowledge workers.
* Or **niche user pains** where existing solutions are bloated.

Your example ideas:

1. **“Click any element to copy it to a desktop folder”**

   * Technical issue: a browser extension **cannot arbitrarily write to a path on your desktop**.

     * You can trigger file downloads or talk to a small local helper app (via native messaging), but you can’t just save directly into `C:\Users\you\Desktop\Whatever` without a user‑mediated flow.
   * The core user pain seems to be:

     * “I see a thing on a page and I want to get its content into a consistent local place quickly.”
   * A more feasible product:

     * Extension that, on click, extracts text/HTML/screenshot of the element and:

       * Copies to clipboard, or
       * Sends it to your own backend, which then syncs with a local folder via a small companion app or a Dropbox/Drive folder.

   If you frame it as **“one‑click snippet capture for researchers”** and aim at e.g. lawyers / analysts / students, you have a plausible niche.

2. **“Wordle prior-words tracker”**

   * There are already lists of prior Wordle answers online and various helper tools, so you’d be in a crowded micro niche. ([AdBlock’s Blog][9])
   * As a fun 1–2 day project to get your extension pipeline working, it’s fine, but not a strong long-term product.

General advantage of extensions:

* Once built and stable, maintenance can be **very low**.
* You can sell them as:

  * One‑time purchases (e.g., $10–$30), or
  * Subscriptions if there’s a backend component (syncing, AI features).

Make sure each extension:

* Has **clear user value** (save X minutes/day or reduce Y annoyance).
* Uses a **shared “extension starter kit”** for auth, licensing, analytics, error logging.

---

### 3.3 How to structure your “prolific” strategy sanely

You don’t have Pieter’s schedule and audience, so you need something sustainable.

I’d suggest a **simple pipeline**:

1. **Problem capture**

   * Whenever you notice friction (in your own work, at Sourcewell, in Columbia, in church life), write it down.
   * Try to phrase it as: “I keep doing X and it’s annoying because Y.”

2. **Quick value test**

   * Ask:

     * “Who would pay for this?”
     * “Roughly how many such people exist?”
     * “Would they pay $5 once, $5/month, or more like $50/month?”
   * If you can’t imagine at least **hundreds** of potential users or **a few high‑ticket buyers**, maybe skip.

3. **Time-boxed build**

   * Decide in advance: “This is a 1‑day product,” or “This is a 1‑week product.”
   * Use your AI agents and starter kits to scaffold the app/extension/model as much as possible.

4. **Minimum viable launch**

   * Micro landing page: one screen that explains:

     * What problem it solves,
     * For whom,
     * How much it costs.
   * Get it in front of a *relevant* niche:

     * A subreddit,
     * A Slack/Discord community,
     * A mailing list,
     * Your own social graph.

5. **Simple metrics**

   * For each product, track:

     * Traffic,
     * Conversion rate,
     * Support burden,
     * Net revenue/month.

   * Categorize after ~1–3 months:

     * **Hit**: high conversion, low support → keep and improve.
     * **Zombie**: some revenue, some support → decide to sunset or bundle into something else.
     * **Dead**: no revenue/usage → shut down, archive code, learn and move on.

6. **Hard caps on maintenance**

   * Decide a total amount of “maintenance hours/month” you’re willing to invest in all micro-products (e.g., 5–10 hours).
   * If you exceed that, either:

     * Kill some, or
     * Raise prices to reduce user count to those who truly value it.

---

### 3.4 Building the meta-systems that make all of this fast

This is where you get huge leverage.

Think in terms of **three starter kits**:

#### 1. Web app starter kit (for micro SaaS / tools)

* Monorepo template that includes:

  * User auth + session management.
  * Stripe or similar billing (one-time + recurring).
  * Basic UI library + styling.
  * Logging & error reporting.
  * Feature flags.
  * A generic “resource” API that you can reuse for different data types.

* For AI features:

  * Wrapper around your model providers,
  * Shared RAG pipeline where you just plug in a different dataset.

Then every new tool is:

* Fork the starter kit,
* Change branding and some domain logic,
* Wire in a different dataset or prompt.

#### 2. Chrome extension starter kit

* Manifest, build tooling, and packaging set up.
* Common modules for:

  * Communicating with your backend,
  * Checking license status,
  * Storing settings.
* A simple popup UI template + content script hook.

So a new extension is:

* Decide core behavior,
* Add one or two content scripts,
* Reuse the rest.

#### 3. ML/AI fine-tune & RAG starter kit

* Single repo with:

  * Data ingestion (CSV, JSON, PDF, audio).
  * Preprocessing (cleaning, chunking, labeling).
  * Fine-tuning scripts for the models you like.
  * RAG scaffolding (indexing, retrieval, evaluation).
  * Dockerfiles and deployment scripts.

So when a client wants a “custom model,” you just:

* Point your pipeline at their data,
* Tweak hyperparameters,
* Run tests,
* Deploy with minimal custom code.

---

## 4. An internal AI “intel system” for your own ideas

You want an AI companion that:

* Watches the world for each of your ideas (ColumbiaOS, gov data, sermons, ID navigator, etc.),
* Stores what it has already seen,
* Surfaces **new** relevant info: competitors, legal changes, tech and cultural trends.

Conceptually, this is your own **“founder RSS + research assistant”**.

### 4.1 Core design

For each idea, you define:

* A set of **sources**:

  * Competitor sites, newsletters.
  * Relevant legislation or gov portals.
  * Industry blogs / journals.
  * Patents, academic papers (for more technical areas).
* A set of **topic queries / keywords**:

  * “hyperlocal news AI”, “Tennessee TNC law”, “sermon AI tools”, “ID debate trends”, etc.

Then your system runs on a schedule:

1. **Fetch**

   * For each source:

     * Pull the homepage / RSS / relevant paths.
     * For general topics, run search queries and fetch top N results.

2. **De‑dup & novelty detection**

   * Compute a **stable identifier** for each doc (URL + hash of content).
   * If not seen before:

     * Store raw text in your data lake.
     * Compute an embedding.
   * If seen before:

     * Skip, unless the content has changed significantly (then treat as an update).

3. **Classification & tagging**

   * Use an LLM to classify items into:

     * “Competitor shipping new feature,”
     * “Legal/regulatory change,”
     * “New tool/tech that could help,”
     * “Market signal / cultural trend,” etc.
   * Tag with relevant ideas (ColumbiaOS vs sermons vs gov data).

4. **Summarization & alerting**

   * Generate a **weekly digest per idea**:

     * Top 3–10 new items, summarized.
     * Suggested actions (“Consider reaching out to X”, “Update your pricing page”, etc.).
   * Maybe a global “founder dashboard” of what’s changed across all ideas.

### 4.2 Implementation details

This dovetails nicely with your existing stack:

* **Scraper & crawler**: just applied to competitor/legal/tech sites instead of local Columbia sites.
* Store documents in the same knowledge graph / data lake, tagged by “idea.”
* Use your RAG stack to let you ask meta‑questions:

  * “Show me regulatory changes in TN that might affect ride‑hailing.”
  * “What new sermon AI tools launched in the last 6 months?”

Big constraints:

* Respect robots.txt and terms of use; for sensitive sites, use official APIs/newsletters.
* Keep compute under control (you can cap how many pages per source per week you process).

Net effect: you turn your own “keeping up with the world” problem into a continuous, partially automated research pipeline.

---

## 5. Architecture & design patterns for your 3 core tools

Your 3 general-purpose tools:

1. Web scraper / crawler + graph builder
2. Transcription pipeline
3. Knowledge graph + RAG query layer

You asked: “Three separate codebases with serverless functions talking via blob storage?”

My take: **one platform, multiple modules**, not three siloed codebases.

### 5.1 Data-first, event-driven mental model

Imagine your system as:

* A **data spine**: raw inputs → processed assets → knowledge graph
* A set of **services** that respond to events on that spine.

#### Data layers

1. **Raw storage (data lake)**

   * All scraped pages, PDFs, audio/video files land here first (e.g., S3 + prefixes per city/source).
   * Immutable; versioned.

2. **Processed artifacts**

   * Cleaned HTML/text, extracted entities, transcripts, embeddings, etc.
   * Also stored in object storage (or a separate bucket).

3. **Knowledge graph / metadata DB**

   * Entities: people, orgs, locations, events, sermons, meetings, contracts.
   * Relationships: “spoke at”, “contracted with”, “owns property”, etc.
   * Implemented as:

     * Postgres + graph extensions or
     * A dedicated graph DB like Neo4j.

4. **Search & vector indices**

   * For fast full-text and semantic search:

     * OpenSearch/Elasticsearch for text.
     * pgvector or a vector DB for embeddings.

#### Services / modules

* **Ingestion service** (scraper/crawler)

  * Responsible for fetching web content and pushing it to raw storage + metadata.

* **Transcription service**

  * Listens for new audio/video in raw storage.
  * Produces transcripts and writes them back as processed artifacts + updates the graph.

* **Enrichment / graph builder**

  * Takes raw/processed text: runs entity extraction, linking, relationship detection.
  * Updates graph DB.

* **Query & API service**

  * Serves:

    * RAG queries,
    * Search,
    * Dashboards.

You can implement each as a set of serverless functions *or* lightweight containerized services, but they all share:

* The same repo (monorepo),
* The same data schema/core libraries.

### 5.2 Code organization

Use something like:

* **Monorepo** with packages:

  * `core-domain` (entity models, event types, graph schemas).
  * `ingestion` (scrapers, normalizers).
  * `transcription` (pipelines, integrations).
  * `enrichment` (NLP, entity resolution).
  * `api` (query endpoints, auth).
  * `frontend` (web/mobile clients).

Each service imports `core-domain`, so all agree on what a “Business,” “Event,” “Meeting,” “Sermon,” etc. is.

Avoid three separate codebases because:

* You’ll duplicate domain models and logic.
* Cross‑cutting changes (e.g., adding a new entity type) become painful.
* Reusing components across cities/verticals is harder.

### 5.3 Runtime patterns

* Use **events / queues** between stages:

  * New raw document event → transcription and parsing.
  * New transcript event → enrichment and graph update.
* Make each stage **idempotent** (safe to run twice).
* Use serverless for bursty, heavy compute (transcription, embedding generation).
* Use small, always‑on services for steady tasks (scrapes, API).

This way, when you spin up:

* Everything Murfreesboro, or
* A sermon SaaS, or
* A TN government intelligence portal,

you’re just adding:

* New sources + mapping rules,
* New frontend + pricing,
* Maybe a few specialized enrichment steps,

on top of the same platform.

---

If you want to keep things general for a bit longer, a nice next step could be:

* Pick **one micro-product lane** (Chrome extensions vs ML fine-tunes vs ColumbiaOS spin‑off),
* And we can sketch a concrete “meta-system” for that lane: from idea capture to launch to maintenance limits.

[1]: https://en.wikipedia.org/wiki/Patch_Media?utm_source=chatgpt.com "Patch Media"
[2]: https://www.vox.com/2019/2/11/18206360/patch-local-news-profitable-revenue-advertising-hale?utm_source=chatgpt.com "Patch digital news company is profitable. Is it the future of local ..."
[3]: https://www.axios.com/2025/03/04/patch-news-ai-newsletters-local-communities?utm_source=chatgpt.com "Patch scales to 30,000 U.S. communities with AI newsletters"
[4]: https://www.axios.com/newsletters/axios-media-trends-dd4a2cc0-f851-11ef-977b-653d661d1707?utm_source=chatgpt.com "Axios Media Trends: Scoop... TikTok eyes services"
[5]: https://levels.io/12-startups-12-months/?utm_source=chatgpt.com "I'm Launching 12 Startups in 12 Months"
[6]: https://medium.com/swlh/how-pieter-levels-makes-at-least-210k-a-month-from-his-laptop-with-zero-employees-47d8046f43cd?utm_source=chatgpt.com "How Pieter Levels Makes (At Least) $210K a Month From ..."
[7]: https://developer.chrome.com/docs/webstore/cws-payments-deprecation?utm_source=chatgpt.com "Chrome Web Store payments deprecation | Chrome Extensions"
[8]: https://www.reddit.com/r/chrome_extensions/comments/1h8w3hn/how_do_i_charge_a_subscription_fee_or_like_a/?utm_source=chatgpt.com "How do I charge a subscription fee or like a lifetime ..."
[9]: https://blog.getadblock.com/its-time-to-talk-about-wordle-919e86820b76?utm_source=chatgpt.com "It's Time to Talk About Wordle. The AdBlock Extension ..."

"

```

---

### File: `docs\meta\prompts\conversation_data\initial_quick_software_product_brainstorm_conversation.md`

```markdown
ME:
"
A form of revenue generation that seems increasingly achievable as LLMs improve is quick one-time software or data deliverables. This thought isn't well-developed in my mind, but I could imagine something like creating a fine-tuned ML or AI model for someone based on data they've gathered, and selling that for a few thousand bucks. Or maybe a fun little desktop game that people pay for once for like $10 and it only takes a few total hours of coding to make and almost nothing to serve. Or I've seen things like Excel templates sold in considerable volumes at prices between $5-$10. That seems less likely to be useful as software is progressively automated. Or maybe very niche Chrome extensions that I sell for a few dollars a piece and only take a few hours to create and require essentially no maintenance (on that note, what about an extension that you can click on in order to give the user the ability to do a single left-click on any element on a webpage and have it copied to a predetermined location on their desktop; that seems helpful to me; or what about a Chrome extension that allows you to see if the word you tried on Wordle has already been used in previous Wordles). Levelsio on Twitter is famous for having created dozens of software products, most of which never took off. But some of them unexpectedly did amazing numbers and then he was able to double down on those. It seems like being prolific is a good way to guarantee success, and the act of continually generating ideas and bringing them to market builds great revenue-building muscles. I could even build meta-systems that make the creation of these products quicker.

Maybe I could even sell the code on a regular basis for a few thousand dollars to people who believe they'll be able to sell it much more easily than I could. 

Can you help me brainstorm a few niche but simple products that could be built and sold as one-offs and aren't already saturated in the market?"

AI:
"
This is a sharp observation. The "One-Time Payment" model (or "Lifetime Deal") is making a massive comeback because people are exhausted by subscriptions ("subscription fatigue").

Your intuition is right: **velocity is your asset.** If you can build small, finished products quickly, you don't need one big "unicorn"; you just need a portfolio of "work horses" that you can sell to users for $10–$50, or sell the entire codebase to a non-technical entrepreneur for $2k–$5k.

Based on the current market (late 2025 landscape) and your constraints, here is a breakdown of your current ideas and a fresh brainstorm of niche, unsaturated concepts.

### Part 1: Quick Validation of Your Ideas

* **Fine-Tuned ML Models:**
    * **Verdict:** **High Potential, but it's a Service, not a Product.**
    * **The Reality:** You can't really "sell a model" as a downloadable file easily because users don't know how to host it.
    * **The Pivot:** Sell this as a **"Productized Service."** You charge a flat fee (e.g., $1,500). The client gives you their messy PDFs/data. You return a private, fine-tuned API endpoint or a Docker container they can run.
* **The "Click-to-Copy" Chrome Extension:**
    * **Verdict:** **Technically Tricky.**
    * **The Hurdle:** Browsers are "sandboxed." They cannot easily write to *specific* arbitrary folders on your desktop without a native host application installed (which scares users). They can usually only write to the "Downloads" folder.
    * **The Pivot:** A "Save to Notion" or "Save to Google Sheets" extension is easier and more valuable to business users.
* **Wordle History Extension:**
    * **Verdict:** **Saturated.**
    * **The Reality:** There are already highly-rated extensions that do exactly this (sync stats, save history). It would be hard to charge for this.

---

### Part 2: Brainstorming Niche One-Off Products
Here are 4 categories of products you can build in a weekend and sell as one-offs.

#### 1. The "Bored Corporate Worker" Desktop Toys
*Market:* Office workers who can't install Steam but have $10 and Admin rights (or just a personal laptop).
* **The "Fake Update" Screen Generator:** A dedicated little app that perfectly mimics a Windows/Mac update screen (with progress bars that actually move) so people can take a nap or a break without being bothered.
* **"Boss Key" Utility:** A modern version of the old gaming concept. A background app that detects when you move your mouse to a specific corner and instantly swaps your "Not Safe For Work" windows (games, Reddit) with a pre-set "Safe" workspace (Excel sheets, Email).

#### 2. "Bridge" Extensions (The Glue Code)
*Market:* Professionals using two SaaS tools that don't talk to each other well. You sell the extension for $29/year or $50 lifetime.
* **"Etsy to Instagram" Reformatter:** An extension that grabs an Etsy product listing (images, price, description) and instantly formats it into an Instagram Story/Post layout, ready to download and post.
* **"LinkedIn Comment Assistant for Recruiters":** Recruiters spend hours commenting. An extension that lets them click "Polite Reject" or "Warm Intro" and pastes a pre-written (but variable) template into the comment box.

#### 3. The "Marketer's Starter Kit" (Selling the Code)
*Market:* Marketing agencies or "wantrepreneurs" who can sell but can't code. You build the tool, they buy the rights.
* **Local Review Widget Generator:**
    * **The Tool:** A simple script that pulls Google Reviews for a business and renders them as a pretty HTML "badge" to put on a website.
    * **The Play:** Don't sell it to business owners. List the *source code* on **Acquire.com** or **Tiny Acquisitions** for $3,000 as a "Micro-SaaS Starter Kit." Marketers will buy this to resell it to their local clients.
* **Simple "Link in Bio" Clone:**
    * **The Tool:** A super basic "Linktree" clone where a user can add buttons and a profile pic.
    * **The Play:** Sell the *entire codebase* as a "White Label SaaS" to an influencer agency. They can then host it themselves and charge their own clients for it.

#### 4. High-Value Data Products (The "One-Time" Asset)
*Market:* People who need data *now* and don't want to scrape it themselves.
* **The "Rich" Lead List:**
    * **The Idea:** Use your Python skills to scrape/aggregate public data (e.g., "All CrossFit gyms in Texas that don't have a website").
    * **The Product:** Clean the data, verify the emails, and sell the CSV file on Gumroad for $49.
    * **Why it works:** It's pure profit after the initial script run.

### The "Productized Code" Strategy
You mentioned selling code to people who can sell it. This is a very real path.
* **Where to sell:** Look at **Tiny Acquisitions**, **Microns.io**, or **Acquire.com**.
* **What sells:** "Starter Kits." A marketer doesn't want just a script. They want a "Business in a Box."
    * *Bad listing:* "Python script that scrapes emails."
    * *Good listing:* "Fully functional Lead Gen SaaS. Includes Stripe integration, user login, and admin dashboard. Ready to launch."

### Next Step
Since you are interested in the "Productized Service" or "Code-as-a-Product" angle, would you like me to outline a **technical spec for the "Local Review Widget"**? I can show you how to structure it so it's easy to hand off to a non-technical buyer.
"


ME:
"
Yeah, dive deeper on the local review widget idea. 

I don't like games/tools that help people be lazy or engage in zero-sum activities. I want things that are genuinely useful. 

I was also interested in the bridge extensions you mentioned. Does the Etsy to LinkedIn extension not already exist? What are some other bridge extensions?

Can you also expand on ideas that utilize my proprietary AI/ML models, perhaps for niche purposes?

What if I sold a cheaper version of a Redgate feature that creates a graph of all the objects in your SQL database and can identify orphans, duplicate objects, and other undesirable features of a database?  Obviously there are some security barriers, but the ones I can think of seem surmountable. Is this product saturated already? What might the revenue method for this be?
"

AI:
"
This is a great pivot. You are moving from "generic tools" (saturated) to "specific utility" (high value).

Below is the deep dive you asked for, starting with the Technical Spec for the Review Widget, followed by unsaturated "Bridge" ideas, a reality check on the SQL tool, and a strategy for your AI models.

### 1. Technical Spec: "The Local Badge" (Review Widget)
**The Pitch:** A "set and forget" script that small business owners (plumbers, dentists) can paste into their Wix/WordPress footer to show a "Live Google Review Badge."
**Why it sells:** Most plugins are subscription-based ($10/mo). You sell the **Source Code** to a marketing agency for $3,000, and they resell it to 500 clients for free as a value-add.

#### Architecture
* **Backend (Python/FastAPI or Node):**
    * **Function:** Hits the Google Places API (or scrapes if you want to be risky, but API is safer for a sold product).
    * **Caching:** *Critical.* You cannot hit the Google API every time a user loads the dentist's website (it costs money).
    * **Logic:** Fetch reviews once every 24 hours -> Store in a simple JSON file or SQLite DB -> Serve *that* static data to the frontend.
* **Frontend (Vanilla JS):**
    * A single `widget.js` file (under 10kb).
    * It injects a `div` into the client’s DOM.
    * **No dependencies:** Do not require React/Vue for the client script. It must be raw JS to avoid conflicts with their weird WordPress themes.
* **The "Deliverable" Package:**
    * A Docker container with the backend.
    * A setup script that asks for their Google Maps API Key and Place ID.
    * A documentation PDF titled "How to deploy this on a $5 DigitalOcean Droplet."

---

### 2. "Bridge" Extensions (Unsaturated Ideas)
You are looking for "Glue Code"—software that connects two distinct professional worlds that don't talk to each other.

* **"Jira to Human" Translator (PM to Client Bridge)**
    * *The Problem:* Developers write Jira updates like "Refactored backend API endpoints for latency." Account Managers have to rewrite this for clients as "Made the site load faster."
    * *The Extension:* A Chrome extension that lives in Jira/Linear. You highlight a technical comment, click "Client Safe," and it uses an LLM to rewrite it into polite, non-technical corporate speak, ready to paste into an email.
* **"Slack Thread to Confluence/Notion" (The Knowledge Saver)**
    * *The Problem:* Great technical decisions happen in Slack threads, then get lost forever.
    * *The Extension:* A button in Slack (or a browser extension viewing the web version) that takes a thread, summarizes the *decision made* and the *rationale*, formats it as a documentation entry, and pushes it to a specific Notion database.
* **"CSV to PDF Invoice" (The Freelance Fixer)**
    * *The Problem:* Many niche SaaS tools export data as ugly CSVs. Freelancers need pretty PDFs to get paid.
    * *The Extension:* A browser action that reads the current page's table (e.g., a time-tracking site that charges for PDF exports) and locally generates a PDF invoice using a simple template you provide.

---

### 3. Proprietary AI Models (The "High-Value" Play)
You mentioned "proprietary" models. In 2025, "proprietary" doesn't mean you invented a new Transformer architecture; it means **you fine-tuned Llama/Mistral on a dataset nobody else has.**

* **The "Construction Bid Leveler"**
    * *The Data:* Get 500 past "Bid Sheets" (PDFs) from a friendly general contractor. These documents are messy, non-standard tables where subcontractors quote prices.
    * *The Model:* Fine-tune a model to extract specific line items (e.g., "Drywall labor," "Sq ft price") into a standardized Excel format.
    * *The Product:* A drag-and-drop tool where a General Contractor drops 5 disparate PDF bids from different electricians, and your model outputs a single "Apples-to-Apples" comparison spreadsheet.
    * *Why it wins:* ChatGPT creates a mess of this because it doesn't understand construction jargon or the specific output format GCs need.

* **"The Lease Abstractor" (Commercial Real Estate)**
    * *The Problem:* Commercial leases are 80 pages of dense legal text. Property managers need to know 3 dates: "Rent Escalation Date," "Lease Expiration," and "Notice Deadline."
    * *The Product:* A local-run model (privacy is key here) that ingests a PDF and spits out a JSON summary of just the *critical dates and financial liabilities*.
    * *The Customer:* Boutique commercial real estate firms who can't afford enterprise software like Yardi.

---

### 4. The SQL "Clean-Up" Tool (Analysis)
**Idea:** A cheaper Redgate SQL Dependency Tracker/Cleaner.
**Verdict:** **Viable, but the "Revenue Method" is key.**

* **Saturation:** The *enterprise* market is saturated (Redgate, ApexSQL). The *indie/freelancer* market is not. Most devs use free scripts from `dbatools` (PowerShell) which are powerful but have no UI.
* **The "Gap":** A **Visual** Garbage Collector. Scripts list orphaned objects in text. A tool that shows a *node graph* of your database and highlights disconnected "islands" (orphaned tables/stored procs) in red is very valuable.
* **Security Barrier:**
    * **Do not make this a SaaS.** No DBA will let you connect a cloud app to their production DB.
    * **Make it a Desktop App (Electron/Tauri).** The connection string never leaves their machine.
* **Revenue Method:**
    * **One-time License Key ($49 - $99).** Use a service like **Gumroad** or **LicenseSpring** to handle the keys.
    * The app checks the license on startup. If valid, it runs.
    * **Marketing Angle:** "The Database Janitor." Don't sell "schema visualization"; sell "finding the 40GB of useless data slowing down your backups."

### High-Value Next Step
Since you already have SQL and Python skills, the **SQL "Database Janitor"** is the most immediate overlap with your capabilities.

**Would you like me to write a Python script using `networkx` (graph library) that takes a dummy SQL schema and actually generates a visualization of "orphaned" tables?** You could use this as the core engine for that product.
"


ME:
"
Let's put the Database Janitor idea to the side temporarily. I'm intrigued, and I could potentially test it (not as a desktop app but just as the code itself) on my own databases to make sure it's useful. Quick question on that before we move on: Should I market the desktop app to businesses, developers, or both?

I wasn't thrilled with the bridge extension does you provided. I was slightly interested in the AI/ML ideas, but not amazed... Go ahead and generate more of those ideas. Also try to expand into one or more other categories of one-time purchase products.
"

AI:
"
This is a great pivot. You are moving from "generic tools" (saturated) to "specific utility" (high value).

Below is the breakdown you asked for.

### Marketing Your "Database Janitor"
* **Target Audience:** **Individual Developers & Freelancers (Bottom-Up).**
    * *Why:* Businesses (CTOs) buy "Enterprise Governance Platforms" (like Redgate) for $5,000/year because they need audit logs and support contracts. They won't buy a $50 app from a solo founder.
    * *The Strategy:* Market to the developer who is tired of cleaning up a messy dev database. They have a discretionary budget (or personal credit card) for a $49 tool that saves *them* a headache this weekend.

---

### Part 1: Better "Bridge" Extensions (High-Friction Professional Flows)
These ideas focus on **"Walled Gardens"**—software that notoriously doesn't play nice with others, forcing highly paid professionals to do manual copy-pasting.

#### 1. The "Clio to Outlook" Case Filer (Legal Tech)
* **The Pain:** Lawyers live in Outlook, but they bill in Clio. Every email they receive from a client needs to be saved into that client's specific folder in Clio for legal compliance. Currently, this is often a manual "save as PDF -> upload" process.
* **The Product:** An Outlook Add-in (side panel) that scans the sender's email address, matches it to a "Matter" in Clio, and offers a **single button: "File to Matter."** It saves the email body and attachments directly to the right Clio folder.
* **Why it sells:** It saves billable hours. A lawyer charging $300/hr will happily pay $99 one-time to save 15 minutes a day.

#### 2. The "Shopify to QuickBooks Desktop" Syncer (E-commerce)
* **The Pain:** Many small shop owners use **QuickBooks Desktop** (not Online) to save money, but Shopify only natively integrates well with the Online version. They are manually typing daily sales totals into QB Desktop.
* **The Product:** A desktop utility (Python/Tkinter) that connects to the Shopify API, downloads yesterday's sales, and formats them into an **IIF file** (Intuit Interchange Format) that can be imported directly into QB Desktop.
* **Why it sells:** Existing solutions are expensive monthly SaaS connectors ($50/mo). A $100 one-time "Connector Script" is a steal.

#### 3. The "MLS Listing Assistant" (Real Estate)
* **The Pain:** Real estate agents type listing details into a Word doc, then manually re-type them into the MLS (Multiple Listing Service), then re-type them *again* into social media.
* **The Product:** A Chrome Extension that reads the "Create Listing" page on the MLS. It adds a button: "Import from Doc." The agent pastes their messy Word doc text, and your tool (using simple regex or cheap LLM) auto-fills the 50 form fields (Bedrooms, Bathrooms, Sq Ft, Description).

---

### Part 2: High-Value AI/ML "Extraction" Products
These use your "Proprietary Model" concept. You fine-tune a small model (like Llama-3-8B) on a very specific, messy document type and sell the **solution**, not the model.

#### 1. "The HazMat Reader" (Safety Data Sheets)
* **The Data:** **Safety Data Sheets (SDS)** are PDFs that every chemical supplier sends with their product. They are mandatory for compliance but strictly non-standard in layout.
* **The Problem:** Warehouses must maintain a digital registry of every chemical on-site. Poor interns spend weeks typing "Flash Point: 45°C" from PDFs into Excel.
* **The Product:** A drag-and-drop tool where a Safety Officer drops 100 SDS PDFs. Your model extracts specific fields (CAS Number, Flash Point, First Aid info) and returns a clean CSV ready for their inventory system.
* **The Market:** Logistics companies, warehouses, manufacturing plants.

#### 2. The "Bill of Lading (BOL) Digitizer" (Logistics)
* **The Data:** Bills of Lading are the "receipts" of the shipping world. They are often scanned, crumpled pieces of paper with handwritten notes.
* **The Problem:** Freight brokers receive thousands of these via email and have to manually type the "Weight" and "Commodity" into their TMS (Transportation Management System).
* **The Product:** An API or "Hot Folder" on their desktop. They save the PDF there, and your tool OCRs it, extracts the **Consignee, Weight, and PRO Number**, and saves it as a structured JSON or Excel row.
* **The Market:** Independent freight brokers and trucking dispatchers.

---

### Part 3: A New Category – "Local Data Utilities"
This category fits your Data Engineer background perfectly. These are **"Unix pipes for the Desktop"**—tools that process data locally (privacy-first) without needing a cloud subscription.

#### 1. The "PII Scrubber" for CSVs
* **The Idea:** Developers often need to use production data for testing, but they can't because it has emails and phone numbers (PII).
* **The Product:** A standalone desktop app. You drop a 5GB CSV into it. It detects columns that look like PII (Emails, SSNs, Names) and **hashes or anonymizes** them (e.g., replaces "John Smith" with "User_492") while preserving the referential integrity (so "John Smith" is always "User_492" across files).
* **Why it sells:** It enables "Safe Staging Data" without uploading sensitive customer info to a 3rd party web tool.

#### 2. "The Excel Diff" (Visual Spreadsheet Comparator)
* **The Idea:** "Track Changes" in Excel is terrible. When a finance team has `Budget_v1.xlsx` and `Budget_v2_FINAL.xlsx`, they don't know what changed in the 50 hidden formulas.
* **The Product:** A tool that takes two Excel files and generates a **visual report** (HTML or PDF). It highlights *exactly* which cells changed values, which formulas were altered, and if any rows were secretly deleted.
* **Why it sells:** Accountants and Data Analysts live in fear of the "fat finger" error. This is their insurance policy.

### Summary of the "Best Bets"
1.  **Fastest to Build:** The **"PII Scrubber"** (It's pure Python/Pandas logic wrapped in a GUI).
2.  **Highest One-Time Price:** The **"Shopify to QB Desktop" Bridge** (Business owners will pay $100+ to stop manual entry).
3.  **Best "Proprietary" Moat:** The **"HazMat Reader"** (Requires a dataset of SDS sheets to train, which is hard for copycats to get quickly).

Which of these sparks the most interest? I can help you outline the "Minimum Viable Product" feature list for any of them.
"



ME:
"
I was interested in the HazMat Reader idea because it seemed niche enough, but you seem to be implying that SDS files are difficult to come by. Why wouldn't that prevent me from capitalizing on this idea?

I'm also interested in the Clio to Outlook extension (which is actually one-directional Outlook to Clio), but I'm surprised that Outlook allows third party extensions. Seems like it would be tough to get through their ecosystem. Also, I need to be able to test this myself. 

All the products of this type need to be relatively quick and easy for me to develop. Major moats are double-edged swords that would discourage me from pursuing the idea, unless it's such a slam dunk that I should devote more energy to it.

Also interested by the Excel product. Is this really not a saturated product?
"


AI:
"
This is the right mindset. You are spotting the "moats" (barriers to entry) and evaluating if they are surmountable.

Here is the "Red Light / Green Light" analysis based on a deeper check of the market and your constraints.

### 1. Outlook to Clio (Legal Tech)
**Verdict: RED LIGHT (Kill the idea)**
* **The Killer:** I dug into the official Clio documentation. Their *official* Outlook add-in already does exactly what we discussed: "File case related email... to Clio Manage."
* **The Lesson:** If the platform owner (Clio) has an "Official Add-in," do not compete with it unless their add-in has a 2-star rating. (Clio’s is decent).
* **The Pivot:** Ignore "Outlook Add-ins." The Microsoft ecosystem requires a rigorous approval process (AppSource) that will slow you down. Stick to **Chrome Extensions** or **Desktop Apps** where you control the distribution.

---

### 2. The HazMat Reader (AI/ML Extraction)
**Verdict: YELLOW LIGHT (High Friction, High Reward)**
* **The "Data" Problem:** You are right, there isn't a "Download All SDS" button. That *is* the moat.
* **How to beat it:**
    * **Don't scrape the world.** You only need ~200 PDFs to fine-tune a Llama-3-8B model to be "good enough" for a demo.
    * **The Strategy:** Go to *one* major manufacturer's site (e.g., Sigma-Aldrich or Fisher Scientific). They often have public SDS links for every product. Write a Python script to scrape just 500 of them.
* **Why pursue it?** Because "lazy" developers won't do this. If you build the model, you have a proprietary asset. You can sell the *API access* to logistics software companies who don't have ML engineers.

---

### 3. The Excel Diff Tool (Desktop Utility)
**Verdict: GREEN LIGHT (Best "Quick Win")**
* **Market Analysis:** I researched the competitors. They are expensive and clunky.
    * **xlCompare:** ~$99 (One-time)
    * **Draftable:** ~$129/year (Subscription)
    * **Synkronizer:** ~$80 (One-time)
* **The Gap:** There is no "Indie" version for $15–$25.
* **Your Advantage:**
    * **Privacy:** Market it as "100% Offline. Your financial data never touches the cloud." (Competitors often have cloud components).
    * **AI Feature:** Add a "Smart Explanation" feature. Instead of just highlighting a cell in red, use a small local LLM (or cheap API call) to explain the change: *"The formula changed from SUM to AVERAGE, causing a 40% drop in value."*

**Implementation Path:**
* **Tech Stack:** Python + **Tauri** (for a lightweight GUI) or **PyQt**.
* **Core Lib:** `pandas` for reading files, `deepdiff` for comparison.
* **Distribution:** Gumroad (handles license keys for you).

---

### 4. New Brainstorm: "Niche" AI Extraction Products
Since the Outlook idea died, here are two fresh ideas that leverage your Data Engineering + AI interest.

#### Idea A: The "Bank Statement Normalizer" (Mortgage/Lending)
* **The Pain:** Mortgage brokers receive PDF bank statements from 50 different banks (Chase, Wells Fargo, Local Credit Union). They all look different. The broker needs to calculate "Average Daily Balance" and "Total Deposits" to approve a loan.
* **The Product:** A drag-and-drop desktop tool.
    * Input: A messy PDF bank statement (Chase, BoA, etc.).
    * Output: A clean, standardized Excel `.xlsx` with columns: `Date`, `Description`, `Amount`, `Transaction Type`.
* **Why it works:** Generic PDF-to-Excel converters fail on bank statements because of the weird headers, footer disclaimers, and multi-column layouts. A fine-tuned model (or even a clever regex engine) specific to *bank statements* is highly valuable.
* **Customer:** Independent Mortgage Brokers.

#### Idea B: The "Government RFP Shredder"
* **The Pain:** Government "Requests for Proposals" (RFPs) are 100+ page PDFs full of boilerplate text. A contractor wants to know: *1. What is the due date? 2. What are the mandatory tech requirements? 3. What is the penalty clause?*
* **The Product:** "RFP-to-Brief." Drop in the PDF, get a 1-page "Bid/No-Bid" summary sheet.
* **The Tech:** This is a perfect use case for "RAG" (Retrieval-Augmented Generation). You don't even need to fine-tune a model. You just need a script that chunks the PDF and asks GPT-4o (via API) to extract those 3 specific answers.
* **Cost to build:** ~1 weekend.
* **Sales Model:** Pay-per-use (buy 10 credits for $20) or a standalone app for $49.

### Which path fits your current energy?
1.  **The "Excel Diff"**: Pure coding, low risk, proven market, you undercut on price.
2.  **The "RFP Shredder"**: High value, leverages modern AI API calls (RAG), solves a "boring" business problem.

(I can provide the Python logic for the "Excel Diff" core engine immediately if you want to test the complexity.)
"

ME:
"
I like the Excel Diff idea! I feel like that's something I can easily test. Please do deep research on xlCompare and Synkronizer to produce a thorough game plan for how I would differentiate my product (differentiating just by software is totally fine with me).
"


```

---

### File: `docs\rust_docs\excel_diff_difficulty_analysis.md`

```markdown
Here’s the difficulty picture for the product, from “hardest” down, based on the docs you gave me.

---

## 1. Difficulty rubric

I’ll score every hurdle on four 1–5 axes and sum them (min 4, max 20):

* **C – Conceptual / algorithmic complexity**
  1 = straightforward plumbing; 5 = you’re effectively doing research-level algorithm design.
* **U – Uncertainty / reverse‑engineering / spec risk**
  1 = fully specified & stable; 5 = lots of undocumented behavior / evolving formats.
* **S – System surface area**
  1 = small, isolated component; 5 = cross‑cuts most of the engine and APIs.
* **P – Performance & scale pressure**
  1 = can be slow; 5 = must be extremely fast and memory‑efficient on large files.

**Total difficulty score = C + U + S + P (range 4–20).**

---

## 2. Ranked technical hurdles

### Summary table

| Rank    | Hurdle                                                                             | C | U | S | P | Total  |
| ------- | ---------------------------------------------------------------------------------- | - | - | - | - | ------ |
| 1       | H1. High‑performance 2D grid diff engine (row/col alignment, block moves, DB mode) | 5 | 3 | 5 | 5 | **18** |
| 2       | H2. Streaming, memory‑efficient parsing & IR build (Rust/WASM, 100MB files)        | 4 | 3 | 4 | 5 | **16** |
| 2 (tie) | H4. Semantic M diff engine (step‑aware + AST diff)                                 | 5 | 3 | 4 | 4 | **16** |
| 4       | H3. M language parser & step model                                                 | 4 | 4 | 4 | 3 | **15** |
| 4 (tie) | H9. Robust handling of weird/legacy/future DataMashup/Excel variants               | 3 | 5 | 4 | 3 | **15** |
| 6       | H5. DAX / data‑model parser & semantic diff                                        | 4 | 4 | 3 | 3 | **14** |
| 7       | H6. Excel formula parser & semantic diff                                           | 4 | 3 | 3 | 3 | **13** |
| 8       | H7. PackageParts & embedded OPC/Embedded.Value extraction                          | 3 | 3 | 3 | 3 | **12** |
| 8 (tie) | H10. Cross‑platform packaging & integration (Mac/Win CLI, Web/WASM, Git)           | 3 | 2 | 4 | 3 | **12** |
| 8 (tie) | H11. Comprehensive test harness, fixtures, fuzzing, golden oracles                 | 3 | 2 | 4 | 3 | **12** |
| 8 (tie) | H12. Human‑facing hierarchical diff reporting (DiffOp → UX/API)                    | 3 | 3 | 4 | 2 | **12** |
| 12      | H8. Host container + DataMashup extraction & top‑level MS‑QDEFF framing            | 2 | 2 | 3 | 2 | **9**  |
| 13      | H13. Permission bindings & DPAPI verification                                      | 2 | 3 | 2 | 1 | **8**  |

Below is what each hurdle actually is and why it scored the way it did.

---

## 3. Hurdles in detail (hardest → easiest)

### H1. High‑performance 2D grid diff engine (spreadsheet + database modes) – **18**

**What it is**

* Row‑ and column‑alignment for arbitrary sheets using Hunt–Szymanski–style LCS on row signatures, then similar alignment for columns, plus detection of inserted/deleted rows, moved blocks, and key‑based “database mode” with heuristic key inference. 
* Needs to correctly distinguish “row moved”, “row changed”, “pure insert/delete”, and handle huge ranges without blowing up.

**Why it’s so hard**

* **C=5:** You’re adapting advanced sequence‑alignment algorithms (LCS/HS, Hungarian matching, rolling hashes) to a noisy 2D domain and combining them into a coherent pipeline.
* **U=3:** The math is known, but “what users expect” (e.g. when to call something a move vs delete+add) is not formally specified.
* **S=5:** This sits in the absolute hot path of the product: every workbook diff depends on it, and it also underpins how formula, M, and DAX differences get surfaced.
* **P=5:** It must run near‑linearly on ~100MB workbooks, so any naive O(n²) behavior will be visible as “this tool hangs”.

---

### H2. Streaming, memory‑efficient parsing & IR build – **16**

**What it is**

* Streaming Open XML + DataMashup parsing to build the normalized `Workbook` / `Sheet` / `Grid` / `DataMashup` IR in Rust, and the same engine compiled to WASM for the browser.

**Why it’s hard**

* **C=4:** You’re doing non‑trivial streaming XML/ZIP parsing and incremental IR construction in a low‑level language, balancing safety and speed.
* **U=3:** Libraries exist, but combining them with your specific IR and performance targets (including WASM limitations) is not “cookie cutter”.
* **S=4:** Everything else builds on this IR; if this layer is wrong or slow, the whole product inherits that.
* **P=5:** This is the first major bottleneck for “instant diff on 100MB files”. Mis‑steps here ripple into all higher‑level algorithms.

---

### H4. Semantic M diff engine (step‑aware + AST diff) – **16**

**What it is**

* Given two parsed M queries (AST + step list), align steps, detect added/removed/changed steps, and summarize precise semantic changes (filter conditions, join types, column sets, etc.), with AST diff as a fallback. 

**Why it’s hard**

* **C=5:** You’re designing your own costed sequence‑diff over “steps”, plus tree edit distance over M ASTs, and then mapping that into human‑readable explanations.
* **U=3:** M is quirky; what’s “the same” vs “different” semantically isn’t always obvious, especially with re‑ordered steps and equivalent expressions.
* **S=4:** This is one of the flagship differentiators of the product (semantic diff for Power Query), so it touches API, reports, and UX.
* **P=4:** You’ll potentially run this over dozens of queries per workbook; each diff is small, but users will notice if semantic diffs feel laggy.

---

### H3. M language parser & step model – **15**

**What it is**

* A production‑grade M parser (lexer + grammar) that produces `MModuleAst`, and a normalized step model (`MStep`, `StepKind`, `StepParams`) that ties back to Power Query’s transformation concepts.

**Why it’s hard**

* **C=4:** Writing a robust parser and AST for a full language is non‑trivial, especially one with its own quirks like M.
* **U=4:** While there’s some documentation, a lot of real‑world behavior is implicit in how Power Query emits code; you’ll have to handle edge cases and odd constructs that aren’t well documented. 
* **S=4:** All M‑related features—semantic diffs, query signatures, rename detection—build on this.
* **P=3:** Queries are usually not huge, but you still need to parse them quickly and safely.

---

### H9. Robust handling of weird/legacy/future formats – **15**

**What it is**

* Making the engine resilient to malformed or legacy DataMashup streams, non‑compliant files, and future MS‑QDEFF versions, using tools like binwalk, Kaitai, and property‑based tests.

**Why it’s hard**

* **C=3:** The logic is more about defensive programming than fancy algorithms, but it’s subtle.
* **U=5:** By definition, you’re dealing with undocumented behavior, historical quirks, and future changes that Microsoft hasn’t written yet.
* **S=4:** Error handling and fallback behavior permeate almost every parsing path; get it wrong and you crash on exactly the messy files customers care most about.
* **P=3:** Usually not on the hottest path, but you still must not degrade performance with over‑zealous scanning.

---

### H5. DAX / data‑model parser & semantic diff – **14**

**What it is**

* Parsing the tabular data model (tables, relationships, measures) from Excel/Power BI containers and building ASTs for DAX measures; then diffing them semantically.

**Why it’s hard**

* **C=4:** Similar in difficulty to formula parsing, but DAX has its own semantics and context rules.
* **U=4:** The underlying model and file layout differ between Excel and PBIX/PBIT, and documentation is patchier.
* **S=3:** It’s a later‑phase feature, but critical for “Modern Excel / Power BI” positioning.
* **P=3:** Measure expressions are small, so performance concerns are moderate.

---

### H6. Excel formula parser & semantic diff – **13**

**What it is**

* Parsing cell formulas into ASTs, canonicalizing them, and diffing to distinguish formatting‑only changes from logical changes in formulas. 

**Why it’s hard**

* **C=4:** You need a robust parser that handles all Excel function syntax, references, and edge cases.
* **U=3:** The grammar is mostly stable, but there are locale and version quirks.
* **S=3:** Used across many sheets but localized to the formula layer.
* **P=3:** There can be lots of formulas, but each one is small, so tree‑diff cost is manageable.

---

### H7. PackageParts & embedded OPC/Embedded.Value extraction – **12**

**What it is**

* Interpreting the `PackageParts` section of the DataMashup stream as an inner OPC/ZIP package, reading `/Config/Package.xml`, `/Formulas/Section1.m`, and recursively handling `/Content/{GUID}` mini‑packages used by `Embedded.Value`.

**Why it’s hard**

* **C=3:** Nested ZIP/OPC handling and plumbing isn’t trivial but is well understood.
* **U=3:** Specs + community tooling exist, but embedded content layouts vary in practice.
* **S=3:** This is how you actually get to the M code and embedded queries, so it’s important but somewhat localized.
* **P=3:** Decompression and IO must be efficient, but not algorithmically extreme.

---

### H10. Cross‑platform packaging & integration (Mac/Win, CLI, Web/WASM, Git) – **12**

**What it is**

* Shipping the Rust core as native binaries (Mac/Win) with a CLI, integrating as a difftool/mergetool for Git/SVN/etc., and as a WASM module behind a web viewer.

**Why it’s hard**

* **C=3:** Mainly build‑, tooling‑, and deployment complexity rather than deep algorithms.
* **U=2:** Toolchains are known, but OS‑specific quirks and Git integration behaviors can surprise you.
* **S=4:** This spans your whole delivery story—CI, installers, web bundling, and integrations.
* **P=3:** Runtime performance is mostly governed by the core engine, but packaging can impact startup time and resource use.

---

### H11. Comprehensive test harness, fixtures, fuzzing, golden oracles – **12**

**What it is**

* A multi‑layer test strategy: unit and integration tests per milestone, a Python fixtures repo that generates real Excel/PBIX scenarios, golden comparisons against Data Mashup Cmdlets, property‑based and performance tests. 

**Why it’s hard**

* **C=3:** Designing good tests and fixtures for this many layers is non‑trivial.
* **U=2:** The plan is clearly laid out, but test data will keep evolving as you discover new edge cases.
* **S=4:** This touches every part of the engine and is key to preventing regressions.
* **P=3:** Test suites must stay fast enough for CI, which becomes a real constraint with large fixtures.

---

### H12. Human‑facing hierarchical diff reporting – **12**

**What it is**

* Designing `DiffOp` types, aggregating raw differences into workbook/sheet/query‑level stories, and shaping this into JSON and UI views that feel intuitive (e.g., GitHub‑style diffs for grid and M).

**Why it’s hard**

* **C=3:** The core logic isn’t algorithmically extreme but requires careful structuring and summarization.
* **U=3:** User expectations around what constitutes a “useful explanation” are fuzzy and must be iterated on.
* **S=4:** The reporting model is the contract between engine, CLI, and any web or desktop UI.
* **P=2:** Mostly not performance‑critical compared to parsing and alignment.

---

### H8. Host container + DataMashup extraction & top‑level MS‑QDEFF framing – **9**

**What it is**

* Treating Excel/PBIX as OPC/ZIP containers, finding the `<DataMashup>` part in `customXml` or root `DataMashup` file, base64 decoding where needed, and parsing the top‑level MS‑QDEFF framing (version + four length‑prefixed sections with invariants).

**Why it’s hard**

* **C=2:** Mostly careful I/O and binary parsing.
* **U=2:** MS‑QDEFF is documented; blogs fill in gaps.
* **S=3:** This is the first step for any M‑aware diff; if it fails, the rest can’t run.
* **P=2:** The work is small relative to the rest of the pipeline.

---

### H13. Permission bindings & DPAPI verification – **8**

**What it is**

* Handling the Permission Bindings section (DPAPI‑protected checksum tying permissions to the mashup); on non‑Windows platforms likely treating it as opaque bytes, optionally validating on Windows with DPAPI. 

**Why it’s hard**

* **C=2:** Using DPAPI is straightforward, and a “treat as opaque” policy is simple.
* **U=3:** Exact behavior across Excel versions and failure modes can be a bit underspecified.
* **S=2:** Important mainly for faithfully mirroring Excel’s privacy behavior; orthogonal to diff logic.
* **P=1:** Rarely on a hot performance path.

---

## 4. How to read this ranking

* The **top cluster (H1, H2, H3, H4, H9)** are your “architecture‑defining” problems: grid alignment, streaming parsing, full M understanding, and robustness to weird inputs. Getting these right (and tested) will make almost everything else easier.
* **Middle items (H5–H7, H10–H12)** are substantial, but you can parallelize them once the core IR and diff spine exist.
* **Bottom items (H8, H13)** are important but relatively contained and can be scoped tightly.

If you’d like, next step we can turn this into a phased implementation roadmap (e.g., which hurdles to tackle in what order to de‑risk the project fastest).
```

---

### File: `docs\rust_docs\excel_diff_education_plan.md`

```markdown
### 1. Tailored Rust Reading List

Given your Python background and the specific use case (file parsing, algorithms, cross-platform/WASM), here is a focused reading list.

#### 1.1 Stage 1: Fundamentals and the Ownership Model (Essential)

1.  **"The Rust Programming Language" (The Book):** (Free Online) The official documentation. Read it thoroughly. Focus intensely on Chapters 4 (Ownership), 10 (Generics, Traits, and Lifetimes), and 13 (Iterators and Closures).
2.  **Rustlings:** (GitHub) A collection of small exercises that force you to fix broken Rust code. This is the best way to internalize the concepts from The Book and understand compiler errors.

#### 1.2 Stage 2: Practical Application and Idiomatic Rust

3.  **"Programming Rust, 2nd Edition" (O'Reilly):** This book goes deeper into the mechanics of how Rust achieves its safety and performance guarantees, crucial for writing efficient parsing algorithms.
4.  **"Command-Line Rust" by Ken Youens-Clark:** Provides excellent, practical examples of reading files, parsing data, handling errors robustly, and structuring Rust projects—all directly relevant to your core engine.

#### 1.3 Stage 3: Essential Crates (Libraries) for This Project

You must study the documentation for the libraries that will form the backbone of your application:

*   **`zip`:** Essential for unpacking the `.xlsx` (OPC) structure and the nested DataMashup binary.
*   **`quick-xml`:** A high-performance, streaming XML parser. Critical for reading the XML parts of the Excel file (like sheet data) without loading everything into memory.
*   **`serde`:** The standard framework for Serializing and Deserializing Rust data structures (useful for configuration and internal data representation).
*   **`thiserror` and `anyhow`:** For robust and idiomatic error handling, which is essential when parsing complex file formats.
*   **`rayon`:** For parallelizing the comparison algorithms to maximize performance on multi-core CPUs.
```

---

### File: `docs\rust_docs\excel_diff_marketing_plan.md`

```markdown
### 1. Go-to-Market Phases
**Phase 1: Beachhead.** Target the neglected macOS segment (finance pros, VCs, consultants). Channels: SEO (“Excel compare Mac”), Mac App Store for trust, Product Hunt/Show HN for tech visibility. Pricing: freemium web viewer plus pro desktop at ~$15–$20/month.  
**Phase 2: Modern Excel Attack.** Launch M/DAX diff; publish technical deep dives proving catches that incumbents miss; engage Power BI and financial modeling communities; partner with trainers and consultants. Positioning: “The first comparison tool for Modern Excel.”  
**Phase 3: Workflow Expansion.** Add collaboration (comments on diffs, shareable links, audit trails) and Git/CI/CD integration; pursue direct sales to finance/audit/compliance teams with team and enterprise tiers.

### 2. Pricing Strategy
Benchmarks: xlCompare $99.99 perpetual; Synkronizer ~€99–€199. A $75 one-time license is likely too low. Prefer a “modern perpetual” at ~$149 including one year of updates, with optional ~$49/year renewals, or a SaaS plan at $15–$20/month for desktop with unlimited comparisons. Keep the web viewer free/read-only for small files as top-of-funnel and conversion driver.


### 3. Sales Channels: Personal Website, Stripe, and Gumroad

**Is it feasible to sell this through a personally owned website using Stripe and Gumroad? Yes, absolutely.** This is the standard approach for independent software vendors (ISVs).

#### The Crucial Role of the Merchant of Record (MoR)

The key challenge in selling software globally is tax compliance (VAT, Sales Tax).

*   **Stripe (Payment Processor):** Stripe handles the payment, but *you* are responsible for calculating, collecting, and remitting all global taxes. This is a massive administrative and legal burden.
*   **Merchant of Record (MoR) (e.g., Gumroad, Paddle, Lemon Squeezy):** The MoR acts as the legal reseller. They handle payments, invoicing, and, critically, all tax compliance. They charge a slightly higher fee but save immense effort.

**Recommendation:** Use a specialized MoR designed for software, such as **Paddle** or **Lemon Squeezy**. They offer better licensing and subscription management features than Gumroad (which is more focused on general creators) and handle the tax burden, allowing you to focus on the product.

### 4. Acquisition Outlook
With core tech (Rust engine, WASM/web, macOS app) and Modern Excel parsing, an early tech/IP acquisition could land in the $300k–$700k range. With cross-platform delivery, M/DAX leadership, and demonstrated revenue/brand traction (Phase 2), xlCompare or others could rationalize $1.5M–$4M for build-vs-buy, market expansion, and defensive motives—especially if competitive tension is introduced.

```

---

### File: `docs\rust_docs\excel_diff_meta_programming.md`

```markdown
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

This choice is recorded in a decision file, for example:

`plans/2025-11-25-cycle.yml`

A typical decision record contains:

* The chosen work type.
* The primary target subsystem (for example, grid diff, M parser, DataMashup host, semantic diff).
* A brief list of reasons grounded in documentation and observed test behavior.
* Risks of deferring this work.
* Pointers to relevant design and test documents.

Decision records are append-only. They provide a historical trace of why each cycle focussed where it did.

### 3.2 Specification and tests-first

Once the work type is chosen, the planner writes a mini-spec for the cycle, typically under `plans/` with a date and short description in the file name.

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

1.  `prompt_original.md`: The exact prompt sent to the AI model to initiate the work.
2.  `plan_response.md`: The full response/plan received from the AI.
3.  `activity_log.txt`: The running log of actions taken during implementation.

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

The outcome is written in a structured artifact, for example:

`results/2025-11-25-tests.yml`

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

Only if the evaluation is acceptable does the cycle proceed to release.

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
* Architecture and risk are managed at a higher level of abstraction, with their own documents and regular reviews.
* The process itself evolves through retrospectives and versioned prompts.

Any new agent or contributor should read this document before making changes, and should treat the described artifacts and flows as the canonical way to move the system forward.
```

---

### File: `docs\rust_docs\excel_diff_m_query_parse.md`

```markdown
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
```

---

### File: `docs\rust_docs\excel_diff_product_differentiation_plan.md`

```markdown
# **Competitive Intelligence Report: Strategic Differentiation in the Excel Comparison Software Market**

## **Executive Summary**

The global financial and operational landscape remains tethered to the spreadsheet. Despite the proliferation of specialized SaaS verticals, Microsoft Excel endures as the "dark matter" of enterprise software—unseen in architectural diagrams but holding the critical mass of business logic. Within this ecosystem, the problem of *version control* and *differential analysis* (diffing) represents a persistent source of operational risk. Two primary incumbents, **xlCompare** and **Synkronizer**, have dominated the desktop-based comparison market for nearly two decades. This report provides an exhaustive, expert-level dissection of these tools to formulate a differentiation strategy for a new market entrant.  
The analysis reveals a market bifurcated by architectural philosophy: xlCompare operates as a standalone executable prioritizing stability and developer-adjacent workflows (CLI, Git integration), while Synkronizer functions as an embedded COM Add-in prioritizing visual immediacy and accounting-style reconciliation. However, both incumbents suffer from significant legacy debt. They are shackled to the Windows operating system, neglect the burgeoning "Modern Excel" stack (Power Query/M and Power Pivot/DAX), and offer user interfaces that predate modern design principles.  
The proposed game plan for differentiation rests on three pillars: **Platform Ubiquity** (targeting the neglected macOS and web-based demographics), **Semantic Depth** (parsing modern data models rather than just cell grids), and **Workflow Modernization** (bridging the gap between technical version control and business-user collaboration). By leveraging modern memory-safe languages (e.g., Rust) and web-standard interfaces, a new entrant can disrupt the duopoly by solving the "Crash and Lag" performance issues inherent to COM-based add-ins while offering the collaborative fluidity of cloud-native applications.

## **1\. The Strategic Landscape of Spreadsheet Management**

### **1.1 The Persistent Problem of Spreadsheet Risk**

To understand the competitive positioning of xlCompare and Synkronizer, one must first contextualize the problem they solve. Spreadsheets are fundamentally mutable environments. Unlike compiled software code, where logic and data are distinct, Excel conflates the two in a two-dimensional grid. A user can accidentally overwrite a formula with a hardcoded value, insert a row that breaks a VLOOKUP range, or introduce a circular reference that subtly skews a financial model.  
The demand for comparison tools is driven by the need to mitigate this risk. In regulated industries—banking, insurance, pharmaceuticals—the ability to audit the changes between Financial\_Model\_v1.xlsx and Financial\_Model\_v2.xlsx is not a convenience; it is a compliance requirement. The incumbents have built their businesses on this necessity, yet their approaches reflect the computing constraints of the early 2000s rather than the collaborative realities of the 2020s.

### **1.2 The "Diff" Dilemma in Binary Formats**

The technical core of the challenge lies in the file format. Since the release of Office 2007, Excel files have utilized the Open XML format (.xlsx), which is essentially a zipped archive of XML documents. Standard text comparison tools (like the basic diff command in Unix or lightweight text editors) are useless against these files because the XML structure is verbose and non-linear. A tiny change in a cell value might result in a massive shift in the underlying XML tree structure, rendering standard diffs unreadable.  
Both xlCompare and Synkronizer have developed proprietary parsing engines to interpret this structure, but they apply fundamentally different philosophies to how that interpretation is presented to the user. xlCompare treats the spreadsheet as a structured document to be parsed externally, while Synkronizer treats it as a live object to be queried internally. This distinction dictates every aspect of their performance, stability, and feature set.

## **2\. Deep Dive Analysis: xlCompare**

### **2.1 Architectural Philosophy: The Standalone Powerhouse**

xlCompare defines itself through its independence. It is a **standalone desktop application** that does not require Microsoft Excel to be running, or even installed, to perform its primary functions. This is a critical architectural decision that insulates the tool from the instability of the Excel process.  
By operating as a separate executable (xlCompare.exe), the software manages its own memory allocation. This is particularly advantageous when dealing with large workbooks. Excel is notorious for memory management issues, often crashing when third-party add-ins attempt to manipulate large ranges of cells via the COM (Component Object Model) interface. xlCompare bypasses this entirely by parsing the file on disk. It reads the XML components of the .xlsx package directly, constructs its own internal representation of the data, and performs the comparison logic in a protected environment.  
This "portable" nature is a significant selling point for enterprise IT environments. The application does not require administrator rights to install, meaning it can be deployed by end-users in restrictive corporate environments without triggering UAC (User Account Control) prompts. This reduces friction in the sales cycle, as the tool can bypass complex IT procurement vetting for "installed software" by running as a localized utility.

### **2.2 Feature Set Granularity**

#### **2.2.1 The Three-Way Comparison Engine**

A standout feature in xlCompare’s arsenal is its native support for **three-way comparison**. In software development, merging code often involves a "Base" version (the common ancestor), a "Mine" version (local changes), and a "Theirs" version (remote changes). xlCompare has ported this paradigm to spreadsheets.  
The interface provides specific panels for these three inputs, allowing users to resolve merge conflicts with a high degree of granularity. This capability specifically targets the "Developer-Analyst"—the financial modeler who creates complex forecasting tools and manages versions using source control concepts, even if they aren't using Git explicitly. The ability to see the *origin* of a conflict (i.e., "This value was 10 in the Base, I changed it to 12, but my colleague changed it to 15") provides the context necessary for safe merging.  
\#\#\#\# 2.2.2 Command Line Interface (CLI) and Automation xlCompare’s commitment to the developer persona is most evident in its robust Command Line Interface. The tool exposes a wide array of switches that allow it to be operated "headless" or as a slave process to other applications.

* *Diff Syntax:* The command xlCompare.exe \-mine:\[file1\] \-base:\[file2\] allows for rapid, scripted comparisons.  
* **Merge Syntax:** The command xlCompare.exe \-merge "C:\\File1.xlsm" "C:\\File2.xlsm" \-output:"C:\\Result.xlsx" enables automated consolidation of workbooks. This is a powerful feature for automated reporting pipelines where a "master" file needs to be updated with "field" submissions nightly without human intervention.  
* **Version Control Integration:** xlCompare explicitly markets its integration with **Git, SVN, and Perforce**. Users can configure their .gitconfig to use xlCompare as the default difftool for .xlsx files. When a user runs a git diff command, instead of seeing a wall of binary garbage, xlCompare launches and visually renders the changes. This integration bridges the gap between the binary nature of Excel and the text-based nature of version control systems.

#### **2.2.3 Primary Keys and Virtual Database Logic**

One of the most difficult challenges in spreadsheet comparison is **alignment**. If a user inserts a row at the top of a sheet, a naive comparison tool will see every subsequent row as "changed" because the row numbers no longer match. xlCompare addresses this via a user-defined **Primary Key** system. Users can right-click a column header (e.g., "Employee ID" or "SKU") and designate it as a Key. The comparison engine then ignores physical row positioning and matches records based on this unique identifier. This effectively treats the spreadsheet as a database table rather than a visual grid, ensuring that row insertions, deletions, or sorts do not corrupt the comparison results.

### **2.3 Pricing and Licensing Model**

xlCompare employs a tiered pricing strategy designed to capture both the casual freelancer and the entrenched enterprise team.

* **The "Freelancer" Tier (Monthly):** At **$9.99/month**, xlCompare offers a low-risk entry point. This is strategically astute, as many consultants only need a comparison tool for the duration of a specific audit or project. This subscription model lowers the barrier to entry significantly compared to competitors requiring large upfront capital.  
* **The "Professional" Tier (Perpetual):** At **$99.99**, the perpetual license appeals to the traditional software buyer who dislikes recurring SaaS fees. This license includes minor updates but often excludes major version jumps, creating a standard upgrade revenue cycle.  
* **The "Team" Tier:** A 5-user pack is priced at **$399.99**, offering a slight discount to encourage departmental adoption.  
* **Evaluation Constraints:** The free evaluation version is feature-rich but time-boxed to **15 days**. Crucially, the evaluation version **disables the "Save" functionality**. This allows users to verify that the tool works and finds the differences, but prevents them from using it to complete a merge task without paying—a classic "shareware" monetization tactic that balances demonstration of value with monetization enforcement.

### **2.4 Weaknesses and Strategic Vulnerabilities**

Despite its technical robustness, xlCompare has significant vulnerabilities:

* **Windows Exclusivity:** The application is strictly **Windows-only**. It does not run on macOS or Linux. This is a critical strategic blindness in 2025, where a significant portion of the tech-forward finance and data science community uses MacBook Pros.  
* **Legacy UI/UX:** The interface, while functional, relies on the older "Ribbon" UI metaphor. It lacks the fluidity, dark mode support, and high-DPI polish expected of modern SaaS tools. It feels like a utility from the Windows 7 era.  
* **Lack of Collaborative Context:** While it finds differences, it offers limited tools for *discussing* them. There is no "commenting" system on the diff itself, nor a way to share a read-only view of the diff with a stakeholder via a URL.

## **3\. Deep Dive Analysis: Synkronizer**

### **3.1 Architectural Philosophy: The Embedded Add-in**

Synkronizer takes a diametrically opposite approach to xlCompare. It is built as a **COM Add-in** that lives directly inside the Excel application. This "parasitic" (in the biological, non-pejorative sense) architecture means that Synkronizer shares the window, memory, and process space of the host Excel instance.  
The strategic advantage of this approach is **context retention**. A user does not have to leave Excel, open a separate app, and load files. They simply click a button in the Ribbon, and the comparison happens right in front of them. The "diff" is not an external report; it is the spreadsheet itself, transformed. This reduces cognitive load and makes the tool feel like a native feature of Excel, which is highly appealing to non-technical users (accountants, project managers) who are intimidated by external "developer" tools.

### **3.2 Feature Set Granularity**

#### **3.2.1 Visual "Heat Mapping" and Reporting**

Synkronizer’s primary interface is the spreadsheet grid itself. It uses a sophisticated **color-coding system** to represent changes :

* **Yellow:** Indicates cells where values have changed.  
* **Orange:** Indicates cells where the *result* of a formula has changed (but the formula itself might be the same, implying a change in precedent cells).  
* **Green:** Represents inserted rows or columns.  
* **Light Blue:** Represents deleted rows or columns.  
* **Lavender:** Highlights duplicates.

This visual immediacy is Synkronizer’s greatest strength. A user can scroll through their familiar workbook and instantly spot variances. Furthermore, Synkronizer generates a **Difference Report**—a separate workbook that lists every change as a hyperlinked record. Clicking a hyperlink in the report jumps the user’s cursor to the exact cell in the original workbook. This navigation loop is incredibly tight and efficient for line-by-line auditing.

#### **3.2.2 "Project" Management**

Recognizing that financial reporting is often cyclical, Synkronizer includes a **Projects** feature. This allows users to save comparison configurations (e.g., "Monthly Budget vs. Actuals"). A user can define that File A is always the "Master," File B is the "Target," and specific exclusion rules (e.g., "Ignore formatting in Column C") apply. This transforms the tool from a one-off utility into a workflow automation platform for recurring tasks.

#### **3.2.3 Database Mode vs. Spreadsheet Mode**

Synkronizer explicitly bifurcates its logic into two modes :

* **Spreadsheet Mode:** This is a cell-by-cell comparison. It assumes the grid structure is paramount. This is ideal for financial models where the position of a cell (e.g., B12 \= "Net Income") is fixed.  
* **Database Mode:** This mode treats the worksheet as a flat-file database. It requires the user to select a **Primary Key** (similar to xlCompare). Synkronizer then ignores the row order and matches records based on the key. This is essential for reconciling lists, such as vendor payments or employee rosters, where the data might be sorted differently between versions.

### **3.3 Pricing and Licensing Model**

Synkronizer positions itself as a premium, professional-grade tool with a higher entry price point than xlCompare.

* **Standard Edition:** Priced around **€99**, this is the entry level for basic users.  
* **Developer Edition:** Priced significantly higher at **€199**, this tier unlocks the "power" features: VBA automation and Command Line control. This segmentation strategy effectively captures consumer surplus from enterprise customers who have the budget and technical need for automation, while keeping a lower price for individual accountants.  
* **Maintenance and Upgrades:** The license is perpetual, but "major upgrades" are not free; they are offered at a 50% discount. This creates a "semi-subscription" revenue tail, encouraging users to pay periodically to stay current with new Excel versions.  
* **Trial Limitations:** The free trial of Synkronizer is severely restricted by data volume, typically capped at a small range like **A1:Z100**. This allows users to test the *functionality* but prevents them from using it for real work on substantial files, a highly effective conversion trigger.

### **3.4 Weaknesses and Strategic Vulnerabilities**

* **Performance Instability:** Because Synkronizer runs inside Excel, it is bound by Excel’s memory limits. Reddit threads and user feedback indicate that large files or complex operations can cause Excel to hang or crash. The tool is fighting for resources with the very application it is analyzing.  
* **Destructive Workflow:** To highlight differences, Synkronizer often physically modifies the formatting of the cells in the open workbook. While this can be undone, it creates a risk of "dirtying" the data. A user might accidentally save the file with the yellow highlight formatting, corrupting the "clean" version of the model.  
* **Installation Friction:** As a COM Add-in, Synkronizer requires installation privileges that modify the Excel environment. In strict banking IT environments, installing COM add-ins is often blocked by Group Policy Object (GPO) settings, whereas a portable executable like xlCompare might slip through.

## **4\. Comparative Technical Analysis**

### **4.1 Alignment Algorithms: The "Secret Sauce"**

The core differentiator between a text diff (like Notepad++) and a spreadsheet diff is **alignment**.

* **xlCompare** relies on a proprietary implementation of the Longest Common Subsequence (LCS) algorithm, heavily modified for 2D grids and bolstered by user-defined keys. Its performance is generally O(n\*m) but optimized for sparse matrices.  
* **Synkronizer** appears to use a "Hash-Map" approach in its Database Mode. It likely hashes the primary key columns to create a unique ID for every row, allowing for O(n) matching complexity. However, its Spreadsheet Mode (visual alignment) is computationally heavier, as it must constantly check for inserted/deleted rows to shift the comparison window, leading to the performance drag observed in large files.

### **4.2 Memory Management**

* **xlCompare (Win):** By running as an external process, it can address up to the OS memory limit (typically huge on modern 64-bit systems). It parses XML via streaming, meaning it doesn't need to load the entire DOM (Document Object Model) into RAM at once.  
* **Synkronizer (Loss):** Bound by the Excel process. Even with 64-bit Excel, the overhead of the COM object wrapper adds significant latency. Every cell interaction ("Read Cell A1") is a marshalling call across the COM boundary, which is inherently slower than internal memory access.

### **4.3 Integration Ecosystems**

* **xlCompare (DevOps Ready):** Its ability to hook into Git and SVN makes it the default choice for the "Financial Engineering" sector—quants and modelers who code.  
* **Synkronizer (Office Ready):** Its VBA hooks allow it to be integrated into *Excel Macros*. A controller can write a macro that says "Open File, Run Synkronizer, Email Report." This makes it the default choice for the "Operational Finance" sector.

## **5\. The "Modern Excel" Gap: A Strategic Pivot**

The most profound insight from the research is the **total absence of support** in both tools for the "Modern Excel" stack. Microsoft has fundamentally evolved Excel over the last decade with the introduction of **Power Query (M)** and **Power Pivot (DAX)**.

### **5.1 The Power Query Blind Spot**

Power Query (Get & Transform) allows users to build complex ETL (Extract, Transform, Load) pipelines within Excel. The logic for these transformations sits in "M" code, which is stored in a binary part of the .xlsx package (DataMashup).

* **The Reality:** A user can change a filter in Power Query that drastically alters the financial results, yet the *grid* (A1:Z100) might look structurally identical until the data is refreshed.  
* **The Gap:** Neither xlCompare nor Synkronizer effectively parses or diffs the M-code scripts. They are blind to the "backend" of modern Excel files. A competitor that offers "Semantic Diffing" for M-code (highlighting that a "Remove Rows" step was added) would immediately capture the high-end analyst market.

### **5.2 The Data Model (DAX) Blind Spot**

Similarly, Power Pivot allows for the creation of relational data models and DAX measures (e.g., Total Sales \= SUM(Sales\[Amount\])).

* **The Gap:** Changes to these measures are critical. If a user changes a DAX measure definition, the output on a Pivot Table changes. The incumbents might catch the *result* change in the Pivot Table cell, but they cannot explain the *cause* (the DAX change).  
* **The Opportunity:** A tool that visualizes the Data Model schema and diffs DAX measures would be a "Blue Ocean" product for Power BI and Excel professionals.

## **6\. Platform & UX Strategy: The Path to Differentiation**

To disrupt this duopoly, a new entrant must not just be "better"; it must be different in kind.

### **6.1 The Mac Opportunity**

Both incumbents are Windows-only. This is a massive strategic vulnerability.

* **The Demographic:** Startups, VCs, and tech companies run on Macs. They currently have *zero* native options for professional Excel comparison. They are forced to use virtualization (Parallels) just to run Synkronizer.  
* **The Play:** Build a **native macOS application** (using Swift or a cross-platform framework like Electron/Tauri). This instantly grants 100% market share of the Mac demographic.

### **6.2 Web-Based "Zero Install"**

Corporate IT security is tightening. Installing .exe files or COM add-ins is becoming harder.

* **The Play:** A **WebAssembly (WASM)** powered web app. Users drag and drop two files into the browser. The parsing happens *locally* in the browser (preserving privacy/security), but no installation is required. This matches the "xltrail" model but for ad-hoc comparison rather than just Git repo hosting.

### **6.3 Visual Semantics: GitHub vs. Excel**

* **Current State:** Synkronizer uses Excel formatting (Yellow backgrounds). This is "destructive" and visually messy.  
* **New Standard:** Adopt the **"GitHub Pull Request"** visual metaphor. Show a clean, side-by-side "Before" and "After" view with red/green shading *over* the data, not *in* the data. Use a "Unified Diff" view for rows. This appeals to the modern user who is accustomed to SaaS tools like Notion, Airtable, and GitHub.

## **7\. Strategic Game Plan: The Roadmap**

This roadmap outlines the step-by-step execution to launch a category-defining product.

### **Phase 1: The Core Engine (Months 1-6)**

* **Technology Stack:** Rust. Rust provides memory safety and blazingly fast XML parsing (via libraries like quick-xml). It can compile to native binaries (for CLI/Desktop) and WASM (for Web).  
* **Algorithm:** Implement a multi-stage, hierarchical comparison engine:
    *   **Hierarchical Scope:** Compare at Workbook, Object (VBA/Charts), Semantic (M-code/DAX), and Grid levels.
    *   **Grid Alignment:**
        *   *Database Mode (Keyed):* O(N) Hash-Based Indexing with automatic key inference.
        *   *Spreadsheet Mode (2D):* Replace simple Myers Diff with a **Multi-Pass Hierarchical Alignment**. Use Hunt-Szymanski for block anchoring, followed by Similarity Scoring (Jaccard) and the Hungarian Algorithm for optimal cell matching within blocks. Include explicit Move Detection.
    *   **Semantic Diffing:** Use Abstract Syntax Tree (AST) comparison (e.g., Zhang-Shasha) for M-code, DAX, and formulas to detect logical changes rather than just text changes.  
* **MVP Feature:** "Instant Diff." Market the speed. "Compare 100MB files in under 2 seconds." This directly attacks the performance weakness of Synkronizer.

### **Phase 2: The Platform Expansion (Months 7-12)**

* **Mac First Launch:** Launch the desktop app on macOS first. Use the lack of competition to build initial traction and SEO dominance for "Excel Compare Mac."  
* **Web Viewer:** Launch the free "Read-Only" web viewer. This acts as the top-of-funnel marketing tool. "Drag, Drop, See Diff." No signup required for basic view.

### **Phase 3: The "Modern" Attack (Months 13-18)**

* **M-Code Parser:** Implement a robust extractor for the `DataMashup` binary stream. This requires handling the nested OPC package structure (unzipping the `item1.bin` stream which is a secondary ZIP archive) to access the `Section1.m` scripts. Render with syntax highlighting and perform AST-based diffing.
* **DAX Diff:** Parse the `xl/model/` structure to reconstruct the data model schema and DAX measures (similar to `model.bim`). Identify changes in measure definitions and table relationships.  
* **Marketing:** "The First Comparison Tool for Modern Excel." Target Power BI user groups and Financial Modeling World Cup participants.

### **Phase 4: The Workflow Moat (Months 19+)**

* **Collaboration Layer:** Allow users to "Comment" on a specific cell difference in the web view. "Why did this change?" \-\> "Updated per CFO request."  
* **Shareable Links:** Allow a user to generate a secure, expiring link to a diff report to send to a client or auditor. This replaces the clunky "Email a PDF report" workflow of Synkronizer.

## **8\. Conclusion**

The market for Excel comparison is not "solved"; it is merely "settled" on outdated technology. xlCompare and Synkronizer have successfully monetized the need for row-matching and cell-diffing on Windows, but they have failed to adapt to the changing landscape of data analytics. They remain tools for the "Spreadsheet Era," while the market is moving toward the "Data Product Era."  
By building a platform-agnostic, high-performance engine that treats Excel files as sophisticated data models (including their Power Query and DAX components) rather than just grids of text, a new entrant can render the incumbents obsolete. The strategy is not to compete on "better highlighting"—it is to compete on "better understanding" of what a spreadsheet actually is in 2025\.

### **Table 1: Competitive Feature Matrix & Differentiation Opportunities**

| Feature Domain | xlCompare (The Utility) | Synkronizer (The Add-in) | New Product Strategy |
| :---- | :---- | :---- | :---- |
| **Platform** | Windows (Native) | Windows (Excel Add-in) | **Universal (Mac/Win/Web)** |
| **Parsing** | External XML Parse | Internal COM Calls | **Rust/WASM Streaming** |
| **Modern Excel** | Ignored | Ignored | **Native M & DAX Support** |
| **Alignment** | User-defined Keys | Database Mode | **Auto-Heuristic \+ Keys** |
| **Pricing** | Sub ($10) \+ Perp ($99) | High Perp (€99-€199) | **Freemium \+ SaaS Team** |
| **Git Ops** | Strong CLI | Weak/None | **Native Web-Based CI/CD** |
| **Safety** | Non-destructive | Modifies Cells (Colors) | **Overlay UI (Non-destructive)** |

This table summarizes the tactical gaps. The incumbents are strong in the *traditional* columns but empty in the *future* columns. That is where the market will be won.

### **Table 2: Detailed Pricing & Licensing Breakdown**

| Tier | xlCompare | Synkronizer | Strategic Implication |
| :---- | :---- | :---- | :---- |
| **Trial** | 15 Days, **No Save** | Limited Range (**A1:Z100**) | Competitors cripple utility. **Strategy:** Offer full utility for small files, cripple only *bulk* or *export* features. |
| **Entry** | $9.99/mo (Subscription) | €99 (Perpetual Standard) | xlCompare wins the "quick need" user. **Strategy:** $15/mo SaaS with "cancel anytime." |
| **Pro/Dev** | $99.99 (Perpetual) | €199 (Developer Edition) | Synkronizer charges premium for CLI/VBA. **Strategy:** Include CLI in standard Pro tier to win developers. |
| **Team** | $399 (5 Users) | Volume Discounts | **Strategy:** "Seat-based" is outdated. Use "Workspace" pricing (unlimited users, limited projects) to encourage viral spread. |

This analysis confirms that while price competition exists, the *model* competition (SaaS vs. Perpetual) is the lever to pull. Moving the value from the *license key* to the *workflow capability* (sharing, history, collaboration) aligns with modern software purchasing behaviors.

#### **Works cited**

1\. Download Excel File Comparison Tool, https://xlcompare.com/download.html 2\. xlCompare Command Line Parameters \- Compare Excel Files, https://xlcompare.com/excel-compare-command-line-parameters.html 3\. xlCompare \- Download and install on Windows \- Microsoft Store, https://apps.microsoft.com/detail/xp9ccdkdcvs4wl?hl=en-US\&gl=US 4\. Version History \- Compare Excel Files, https://xlcompare.com/changelog.html 5\. Merge two Excel Worksheets into one from command line \- xlCompare, https://xlcompare.quora.com/Merge-two-Excel-Worksheets-into-one-from-command-line 6\. Integrate xlCompare into SVN client \- Compare Excel Files, https://xlcompare.com/svn-integration.html 7\. Using xlCompare with Perforce (P4V) for Excel (XLSX,XLSM) files, https://xlcompare.com/perforce.html 8\. xlCompare Help Library \- Compare Excel Files, https://xlcompare.com/help.html 9\. Order Excel File Compare Tool \- Compare Excel Files, https://xlcompare.com/order.html 10\. VeriDiff vs Beyond Compare vs xlCompare: Professional File Comparison Tool Review, https://veridiff.com/blog/veridiff-vs-beyond-compare-vs-xlcompare 11\. How to compare two Excel files or sheets for differences \- Ablebits.com, https://www.ablebits.com/office-addins-blog/compare-two-excel-files-sheets/ 12\. Synkronizer Excel Compare Tool: How to compare two excel files, https://www.synkronizer.com/ 13\. Feature list of professional and developer edition \- Synkronizer ..., https://www.synkronizer.com/excel-compare-tool-editions 14\. Frequently Asked Questions... \- Synkronizer Excel Compare Tool, https://www.synkronizer.com/compare-excel-files-faq 15\. Best value for money and great volume discounts. \- Synkronizer Excel Compare Tool, https://www.synkronizer.com/excel-compare-pricing 16\. Purchase Best Performing Latest Versions of Synkronizer, https://www.synkronizer.com/purchase 17\. Synkronizer 11 User Manual, https://www.synkronizer.com/files/synk11\_user\_manual.pdf 18\. How do I get Excel to stop crashing with large data models? \- Reddit, https://www.reddit.com/r/excel/comments/191nwn7/how\_do\_i\_get\_excel\_to\_stop\_crashing\_with\_large/ 19\. Issues with Spreadsheet Compare? : r/excel \- Reddit, https://www.reddit.com/r/excel/comments/1212dz4/issues\_with\_spreadsheet\_compare/ 20\. 5 tools to compare Excel files \- xltrail, https://www.xltrail.com/blog/compare-excel-files 21\. Excel solver download and instantly try Synkronizer for free., https://www.synkronizer.com/excel-compare-faq?sm=performance 22\. How Power Query and Power Pivot work together \- Microsoft Support, https://support.microsoft.com/en-us/office/how-power-query-and-power-pivot-work-together-a5f52cba-2150-4fc0-bb8f-b21d69990bc0 23\. Advantage of Data Model over Power Query? : r/excel \- Reddit, https://www.reddit.com/r/excel/comments/m1snj8/advantage\_of\_data\_model\_over\_power\_query/ 24\. Excel Power Pivot & Data Model explained \- YouTube, https://www.youtube.com/watch?v=Gf4HmkR7\_FE 25\. xltrail \- Version Control for Excel Spreadsheets \- xltrail is a version control system for Excel workbooks. It tracks changes, compares worksheets and VBA, and provides an audit trail for easy collaboration., https://www.xltrail.com/
```

---

### File: `docs\rust_docs\excel_diff_technical_document.md`

```markdown
## 1. Scope and Requirements

The diff engine’s job is:

1. Take two Excel (or PBIX/PBIT) workbooks.
2. Build a *semantic* internal representation:

   * Workbook / sheets / tables / cells.
   * Power Query M queries and metadata (via DataMashup). 
   * Power Pivot / DAX measures (later phase). 
3. Produce a **hierarchical diff**:

   * Workbook & object structure changes.
   * Grid/table changes (rows, columns, cells).
   * Semantic changes in M queries, DAX, and formula logic.
   * Metadata‑only changes (load destinations, privacy, groupings, etc.). 

The architecture is tuned for:

* Multi‑platform Rust/WASM engine. 
* “Instant diff” behavior even on ~100 MB workbooks by using streaming, linear or near‑linear algorithms where possible. 

---

## 2. Internal Data Model

All diff algorithms operate on a normalized IR; parsing is handled by the DataMashup parser and Open XML layer. 

### 2.1 Workbook‑level types

At a high level:

```text
Workbook {
    id: WorkbookId,
    sheets: Vec<Sheet>,
    data_model: Option<DataModel>,   // DAX, relationships (phase 3+)
    mashup: Option<DataMashup>,      // M queries + metadata
}

Sheet {
    name: String,
    kind: SheetKind,                 // Worksheet, Chart, Macro, etc.
    grid: Grid,                      // 2D cells
    tables: Vec<Table>,              // Excel Tables
}

Grid {
    nrows: u32,
    ncols: u32,
    rows: Vec<Row>,
}

Row {
    index: u32,                      // 0-based logical index
    cells: Vec<Cell>,
}

Cell {
    row: u32,
    col: u32,
    address: CellAddress,            // "A1"
    formula: Option<FormulaAst>,     // parsed AST
    value: Option<CellValue>,        // typed value
    format: CellFormatSummary,       // simplified style info
}
```

We *do not* keep the raw XML at this layer; that’s already validated at parse time.

### 2.2 Power Query (M) model

The M parser blueprint exposes: 

```text
DataMashup {
    queries: Vec<Query>,
    metadata: Metadata,
}

Query {
    name: String,            // "Section1/Foo"
    section_member: String,  // "Foo"
    expression_m: String,    // original code
    ast: MModuleAst,         // parsed AST
    steps: Vec<MStep>,       // normalized pipeline-style representation
    meta: QueryMetadata,
}
```

* `steps` is the crucial structure for semantic diff. Each step corresponds to a meaningful UI operation (filter, join, column removal, etc.) when possible.

### 2.3 Data model (DAX) – future‑phase

Analogous to queries:

```text
DataModel {
    tables: Vec<ModelTable>,
    relationships: Vec<ModelRelationship>,
    measures: Vec<Measure>,
}

Measure {
    table: String,
    name: String,
    dax_source: String,
    dax_ast: DaxAst,
}
```

The diff algorithms below are designed so DAX can plug into the same AST diff machinery as M.

---

## 3. High‑Level Diff Pipeline

The diff engine is organized as a cascade of increasingly fine‑grained comparers:

1. **Object graph diff**

   * Workbook metadata, sheets, named ranges, tables, charts, VBA modules.
2. **Tabular diff (Database Mode)**

   * For sheets/tables with a key column.
3. **Grid diff (Spreadsheet Mode)**

   * 2D alignment for non‑keyed sheets (financial models, templates).
4. **Semantic diff for logic**

   * M queries, DAX, and cell formulas via AST comparison.
5. **Metadata diff**

   * Load destinations, query groups, privacy flags, etc.

Each stage operates on a well‑typed input and emits a stream of `DiffOp` objects:

```text
enum DiffOp {
    SheetAdded { name: String, ... },
    SheetRemoved { name: String, ... },
    RowAdded { sheet: SheetId, row: RowIdx, ... },
    RowRemoved { ... },
    CellEdited { sheet: SheetId, addr: CellAddress, from: CellSnapshot, to: CellSnapshot },
    MQueryChange { name: String, detail: MQueryChangeDetail },
    DaxMeasureChange { ... },
    MetadataChange { path: MetadataPath, from: MetaValue, to: MetaValue },
    // etc.
}
```

The **frontend** and **CLI** consume these ops to render visual diffs or JSON reports, and the **testing plan** uses the same ops to assert correctness. 

---

## 4. Object Graph Diff

Before looking at cells, we diff the *structure* of the workbook.

### 4.1 Sheet and object identity

We treat:

* Sheets keyed by (case‑insensitive) name plus type.
* Tables keyed by sheet + table name.
* Named ranges keyed by name.

Algorithm (for any keyed object set):

1. Build maps `A: name -> objectA` and `B: name -> objectB`.
2. Objects only in `A` → `Removed`.
3. Only in `B` → `Added`.
4. In both → recursively diff their content.

To detect **renames**, we add a cheap similarity layer:

* Compute a signature for each object (e.g., hash of first N non‑empty cells or hash of M query name set).
* For objects with high signature similarity but different names, run a stable matching (Hungarian) on `1 - similarity` cost matrix and emit `Renamed` events when cost is below a threshold.

Complexity is dominated by building maps O(n) and a small Hungarian instance O(k³), where k is number of ambiguous candidates (usually small).

---

## 5. Tabular Diff (Database Mode)

For sheets/tables that behave like relational tables (lists of transactions, dimension tables), we use **key‑based diff** for O(N) alignment. This covers the “Database Mode” case described in the product plan. 

### 5.1 Key discovery

We have three modes:

1. **User‑provided key** (strongest): user marks one or more key columns.
2. **Metadata key**: Excel table “unique key” or query metadata tells us the primary key.
3. **Heuristic key inference**:

   * Candidate columns: those with no blanks and high uniqueness ratio.
   * Prefer integer/ID‑like columns (`[A-Z]*\d+` patterns, GUIDs).
   * If no single column qualifies, consider column combinations (limited to small subsets).

We expose the chosen key in the diff report for transparency.

### 5.2 Keyed diff algorithm

Given:

* Two tables `TA` and `TB`.
* Key function `key(row) -> Key`.

1. Build hash maps:

   ```text
   mapA: Key -> Vec<RowA>
   mapB: Key -> Vec<RowB>
   ```

   (vectors allow us to surface duplicate‑key issues explicitly.)

2. For each key in `mapA ∪ mapB`:

   * If key ∈ `A` only → `RowRemoved`.
   * If key ∈ `B` only → `RowAdded`.
   * If key in both:

     * Handle duplicates:

       * If both sides have multiple rows, treat as a *duplicate key cluster* and run a small Hungarian match using row similarity (e.g., Jaccard similarity on changed columns).
     * For each matched pair, run **row diff** (see below).

3. **Row diff**:

   * Compare cells field‑wise:

     * If formulas differ semantically (AST diff) or values differ → `CellEdited`.
     * Track which columns changed to produce per‑row summaries.

Complexity:

* Hash map construction: O(N) (N = total rows).
* Row comparisons: O(M) per matched row (M = columns).
* Hungarian clusters are rare and small; cost negligible compared to O(N·M).

---

## 6. Grid Diff (Spreadsheet Mode)

This is the “financial model” case where *physical layout* and 2D alignment matter and there may be no clear key. Here we deploy the **multi‑pass hierarchical alignment** strategy sketched in the product plan. 

Key idea: reduce the 2D problem to a sequence of mostly 1D problems (rows, then columns), with smart anchors and move detection.

### 6.1 Row‑level alignment via Hunt–Szymanski

We treat each sheet as a sequence of row signatures:

1. For each row, compute a signature:

   * Hash of `(non‑blank cell positions, cell values up to normalization)`.
   * Optionally fold in formula structure but ignore constants for robustness.

2. Apply **Hunt–Szymanski** (HS) longest common subsequence algorithm on these signatures:

   * HS improves over naive O(n²) LCS by focusing on *rare* symbols, giving O((n + r) log n) where `r` is number of equal‑signature pairs.
   * Result: an ordered set of “anchor” row matches: `(iA, jB)` pairs believed to be the same logical row.

3. Between anchors, classify stretches:

   * A block of rows present in A only → deleted block.
   * In B only → inserted block.
   * Mixed → ambiguous; we recurse with a more permissive similarity metric or fall back to cell‑wise diff.

This alignment already gives robust detection of row insertions/deletions and reorders without treating the entire sheet as “changed”.

### 6.2 Column alignment

Within each aligned row block (range of rows that are matched 1‑to‑1 across sheets), we align columns:

1. Compute column signatures from header row + sample body rows.
2. Run another HS pass on column sequences to detect inserted/deleted/renamed columns.
3. Use these aligned columns when computing cell-level edits, which avoids marking entire blocks as changed when only some columns moved.

For worksheets with “headerless” regions, we can run the same algorithm on a restricted region (e.g., modeling block) identified by heuristics or user hints.

### 6.3 Block move detection

We want to distinguish **moved blocks** from delete+insert pairs.

1. For each maximal deleted row block in A, compute a composite block hash:

   * Combine per-row signatures and relative column signatures.
2. Search in B for inserted blocks with matching or similar hashes.
3. When a match is found:

   * Run a detailed cell diff inside the candidate blocks.
   * If a high fraction of cells are equal, emit `BlockMoved` instead of separate add/remove ops.

This is essentially a rolling‑hash search (rsync‑style), constrained to plausible windows to stay near O(N).

### 6.4 Cell‑level edit detection

Once row/column alignment is known, the cell diff is straightforward:

For each aligned `(rowA, rowB)` and aligned `(colA, colB)`:

1. Compare **formula ASTs** (if any).
2. Compare displayed values and types.
3. Compare formats (if we surface formatting diffs).

Classification:

* If formula ASTs equal (under canonicalization) but values differ:

  * Probably a recalculation difference only → optional `ValueOnlyChanged`.
* If formula AST changed:

  * `FormulaChanged` with an embedded AST diff (see Section 8).
* If one side empty and other non‑empty:

  * `CellAdded` or `CellRemoved`.

For unaligned rows/cols, we emit bulk operations (`RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`) rather than per‑cell ops.

---

## 7. M Query Diff Algorithms

The M diff engine is where we differentiate strongly vs incumbents; it sits on top of the DataMashup parser.  

There are three layers:

1. **Query‑level alignment** (which queries exist?).
2. **Textual diff** (early milestone). 
3. **Semantic (AST + step) diff** (core differentiator). 

### 7.1 Query alignment

We treat queries as keyed by `name = "Section1/MemberName"`.

1. Build maps `A: name -> QueryA`, `B: name -> QueryB`.
2. Direct matches on name → candidate pairs.
3. For unmatched queries, detect **renames**:

   * Compute a `query_signature`:

     * Multiset of top‑level step kinds and names.
     * Normalized hash of the AST (e.g., tree structure without identifiers).
   * For name‑mismatched queries with similar signature, run Hungarian matching on `1 - similarity` cost to find best rename candidates.
   * Thresholded matches become `Renamed { from, to }` events.

Remaining unmatched queries are additions/removals.

### 7.2 Textual diff (Milestone 6)

For each aligned query pair:

1. Compare `expression_m` strings.
2. If identical → no definition change.
3. If different:

   * Run a Myers diff (or other standard text diff) at line level and embed that in `DefinitionChanged` detail.
4. Additionally, compare `QueryMetadata` (load destination, privacy, group):

   * If only metadata changed → emit `MetadataChangedOnly`.

This layer gives a fast MVP and matches the testing plan’s `MQueryDiff` enum. 

### 7.3 Semantic (AST + step) diff

Once AST parsing is in place, we upgrade `DefinitionChanged` to structured semantic information.

#### 7.3.1 Canonicalization

Before diffing:

1. Strip whitespace and comments.
2. Normalize:

   * Commutative operations: sort operands (for simple arithmetic, logical ops).
   * `let` chains: canonicalize step order when dependencies permit (e.g., reorder independent steps if desired).
3. Normalize identifiers if you want to treat purely cosmetic renames as non‑changes (configurable).

If two canonical ASTs are byte‑identical, we treat the queries as semantically equal even if text is very different. (This supports the “formatting only” milestone.) 

#### 7.3.2 Step‑aware diff

Most user‑visible changes correspond to adding/removing/modifying **steps** in the query’s transformation pipeline.

We represent each query as:

```text
MStep {
    name: String,           // "Filtered Rows", "Removed Other Columns"
    kind: StepKind,         // Filter, GroupBy, Join, RenameColumns, ...
    parameters: StepParams, // structured field for each kind
}
```

Algorithm:

1. Build sequences `SA` and `SB` of MSteps (in order).

2. Compute an alignment using a costed sequence diff (e.g. dynamic programming):

   * Cost 0 if `kind` and key parameters match.
   * Moderate cost for parameter changes (e.g., filter predicate changes).
   * Higher cost for insert/remove.

3. The DP yields an alignment matrix; we backtrack to produce step‑level changes:

   * `StepAdded { position, step }`
   * `StepRemoved { position, step }`
   * `StepModified { from, to, detail }`

4. For each `StepModified`, we can drill into parameter structure:

   * For filters: report column and predicate change (“Region changed from `<> null` to `= "EMEA"`”). 
   * For joins: report join type changes (Inner → LeftOuter), join key changes, etc.
   * For projections: columns added/removed.

This gives exactly the semantics the testing plan expects for filters, column removals, join changes, etc. 

#### 7.3.3 Fallback tree edit distance

For steps we can’t classify, or for expressions inside steps, we can fall back to **tree edit distance** (Zhang–Shasha):

* Nodes are AST constructs.
* Edit operations: insert, delete, substitute.
* Costs chosen so that structurally small changes lead to small distances.

This yields compact diffs (“function call changed from `Table.AddColumn` to `Table.TransformColumns`”) without requiring handcrafted handling for every possible M construct.

---

## 8. DAX and Formula Diff Algorithms

DAX and Excel formulas are both expression languages; we can reuse much of the M semantic machinery.

### 8.1 Parsing and canonicalization

For each formula / measure:

1. Parse into an AST (operators, function calls, references).
2. Canonicalize:

   * Normalize whitespace, casing.
   * Reorder commutative subtrees when safe.
   * Optional: normalize equivalent syntaxes (`AND(a,b)` vs `a && b`).

If canonical ASTs are equal → no logical change (formatting only).

### 8.2 Expression diff

For differing ASTs:

1. Run tree edit distance to identify changed subtrees.
2. Summarize at a human‑useful granularity:

   * “Measure `TotalSales` changed aggregation from SUM to AVERAGE.”
   * “Filter condition on `Calendar[Year]` changed from `>= 2020` to `>= 2021`.”

Implementation detail:

* Because DAX formulas are relatively small, typical tree edit distance costs are tiny; performance is dominated by parsing, not diff.

---

## 9. Metadata Diff Algorithms

Metadata includes:

* Where queries load (sheet vs model vs connection‑only).
* Query display folders/groups.
* Table relationships (for the data model).
* Permissions/privacy flags.

We treat metadata as a *typed key–value tree*:

```text
MetadataPath = Vec<String>;  // e.g. ["MQuery", "Section1/Foo", "LoadToSheet"]
MetaValue    = Enum { Bool, Int, String, Enum, Json, ... }
```

Algorithm:

1. Flatten both metadata trees into maps from `MetadataPath` to `MetaValue`.
2. For each path in the union:

   * If value only in A → `MetadataRemoved`.
   * Only in B → `MetadataAdded`.
   * Both but unequal → `MetadataChanged`.

Changes are grouped under logical domains (e.g., query `Foo`’s load destinations), which drives user‑facing categories like `MetadataChangedOnly` when the query logic didn’t change. 

---

## 10. Complexity and Performance

### 10.1 Grid diff

Let:

* `R` = rows, `C` = columns.

Row HS alignment: O((R + r) log R) where `r` is number of equal signature pairs; typical real‑world sheets have relatively distinctive rows (low `r`).

Within blocks:

* Column HS: O((C + c) log C).
* Cell diffs: only for aligned rows/cols, typically O(R·C) in the common case but with early exits for equal rows/cells.

Block move detection uses rolling hashes and windows, keeping the total cost linear in practice.

### 10.2 Tabular diff

Keyed diff is O(N) in number of rows plus O(M) per changed row; we never perform global quadratic algorithms on big tables.

### 10.3 M / DAX diff

* Query alignment: O(Q log Q) for maps and small matching, `Q` = number of queries.
* Step alignment uses dynamic programming; step sequences rarely exceed a few dozen entries, so complexity is negligible.
* AST diff costs are small because expressions are compact.

The design is consistent with the product‑plan goal: “compare 100MB files in under ~2 seconds” given streaming parsers and native Rust performance. 

---

## 11. Implementation Patterns in Rust

### 11.1 Diffable trait

To keep the engine modular:

```text
trait Diffable {
    type Diff;
    fn diff(&self, other: &Self) -> Self::Diff;
}
```

We implement `Diffable` for:

* `Workbook`, `Sheet`, `Grid`, `Row`, `Cell`.
* `Query`, `MStep`, `Measure`.
* Metadata trees.

Each `Diff` type is convertible to `Vec<DiffOp>`, so a generic driver can orchestrate the whole comparison:

```text
fn diff_workbooks(a: &Workbook, b: &Workbook) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    ops.extend(a.diff(b).into_ops());
    ops
}
```

### 11.2 Streaming and memory

* XML and DataMashup parsing is already designed to be streaming and bounded. 
* For huge sheets, we can compute row signatures on a streaming basis, storing only hashes and basic row metadata until we need to inspect cells.

### 11.3 Testability

The testing plan is aligned with these abstractions:

* Unit tests exercise `Diffable` implementations (e.g., M query diff kinds). 
* Integration tests run end‑to‑end from real `.xlsx` pairs to JSON diff reports, asserting counts and categories (added/removed/definition vs metadata changes). 

This separation keeps the core algorithms independently verifiable while still matching the product‑level contracts (CLI, web viewer).

---

## 12. Putting It All Together

End‑to‑end, the diff identification process is:

1. Parse both workbooks into `Workbook` IR, including `DataMashup` and data model if present. 

2. Run:

   * Object graph diff (sheets, tables, queries, measures).
   * For each sheet/table: choose **Database** or **Spreadsheet** mode and run the corresponding alignment algorithm.
   * For each aligned cell/formula/measure: perform AST diff to distinguish formatting vs logic changes.
   * For each M query: run step‑aware semantic diff; same for DAX measures.
   * For metadata: run tree diff, using specialized categories like `MetadataChangedOnly`.

3. Aggregate the resulting `DiffOp` stream by scope (Workbook → Sheet → Object → Cell/Query) for presentation and for automated consumers (CLI/JSON, CI integrations).

The result is an engine that:

* Understands Excel files as multi‑layered data products, not just grids.
* Uses alignment and AST techniques that are asymptotically efficient and tuned to real‑world patterns.
* Lines up exactly with the testing milestones and product roadmap you already outlined.

```

---

### File: `docs\rust_docs\excel_diff_testing_plan.md`

```markdown
Here's a concrete, end-to-end testing blueprint reorganized into interleaved vertical-slice phases (per test_reorganization.md). The phases advance container handling, grid parsing, and M parsing together so we surface the grid alignment/memory risks early while still aiming at semantic M-query diffing.

For each phase:
* What the Rust side does
* What to test (unit/integration/property/perf)
* How the Python fixture repo participates

### Priority tags

Use `[G]` for release-gating tests, `[H]` for hardening/nice-to-have, `[E]` for exploratory/fuzz cases, and `[RC]` for resource-constrained guardrails (explicit memory/time ceilings). Phases 1–3 default to `[G]` unless a test is explicitly tagged otherwise; later phases call out `[H]/[E]/[RC]` inline.

### Phase index (map)

| Phase | Focus | IDs | Key risks exercised |
| ----- | ---------------------------- | ------------------- | --------------------- |
| 0 | Harness & fixtures | - | Tooling only |
| 1 | Containers, basic grid IR, WASM build guard | M1–M2, PG1 | H2, H8, WASM |
| 2 | IR semantics (snapshots, M) + streaming budget | PG2–PG3, PG5–PG6, M3–M5 | H2, H3, H7, H8, H9 |
| 3 | MVP diff slice + early DataMashup fuzzing | PG4, M6, G1–G7 | H1, H3, H4, H9 |
| 3.5 | PBIX host support | PBIX-1 | H8, H9 |
| 4 | Advanced alignment & DB mode (incl. adversarial grids) | M7, G8–G12, D1–D10 | H1, H4 |
| 5 | Polish, perf, metrics | M8–M12, P1–P2 | H2, H9, H10, H11, H12 |
| 6 | DAX/model stubs (post-MVP) | DX1 | Data model / DAX |

### MVP readiness

| Capability | Must work before MVP | Can land just before release | Post-MVP |
| ------------------------------- | --------------------------- | ------------------------------ | -------- |
| Excel grid diff | Yes |  |  |
| Excel DataMashup + M diff | Yes |  |  |
| PBIX with DataMashup |  | Yes |  |
| PBIX without DataMashup (tabular model) |  |  | Yes |
| DAX / data model diff |  |  | Yes |

---

## 0. Test harness & repo layout

Before milestones, define the basic shape:

* **Rust repo (`excel-diff-core`)**

  * `src/…` – core parser + diff engine
  * `tests/…` – integration tests that open real `.xlsx`/`.pbix`
  * `fixtures/…` – copied/generated Excel / PBIX files
  * Optional: `testdata_manifest.yaml` – list of scenarios and file paths (consumed by both Python and Rust)

* **Python fixtures repo (`excel-fixtures`)**

  * Python scripts that:

    * Create new Excel/PBIX files from scratch
    * Clone and mutate base fixtures for variant tests
  * Writes to a shared checked‑out directory like `../excel-diff-core/fixtures/generated/…`
  * You can drive it by a manifest keyed by scenario IDs so LLM‑generated Python has a simple contract.

On CI: a setup step runs “generate fixtures” (Python) before `cargo test`. Locally you can run that occasionally or on demand.

--- 

---

### Metrics export for planner agents

To keep the AI “planner” loop data-driven instead of parsing text logs:

* Add a harness feature flag `metrics-export` that writes `target/metrics/current_run.json` after test/bench runs.
* Capture at least:

  * `duration_micros` or `time_ms_per_mb`
  * `alloc_bytes` / `peak_memory_usage_kb` (via `dhat` or a custom allocator wrapper)
  * `aligned_row_ratio` (aligned rows / total rows) for grid tests
* In CI, publish `current_run.json` as an artifact and optionally diff it against a `baseline.json` to spot perf regressions automatically.

---

---

## How the Python fixture repo fits in

To make this all workable with LLM-authored Python:

1. **Define a simple manifest schema** (YAML/JSON) in the Rust repo:

   ```yaml
   - id: m_add_query
     description: "Query added to workbook B"
     kind: excel_pair
     a: "fixtures/generated/m_add_query_a.xlsx"
     b: "fixtures/generated/m_add_query_b.xlsx"
   - id: metadata_simple
     kind: excel_single
     path: "fixtures/generated/metadata_simple.xlsx"
   ```

2. **Python script responsibilities**

   * Reads the manifest.
   * For each entry:

     * If files missing or `--force` flag set, (re)generate them.
   * Generation patterns per scenario:

     * `excel_single`: create workbook with specified queries/settings.
     * `excel_pair`: start from a base version and apply mutations to create B.

3. **Rust tests**

   * Use the same manifest for test discovery:

     * e.g., a helper `load_fixture_pair("m_add_query")` that returns two paths.
   * This keeps Rust tests declarative and stable even as the Python side evolves.

---

---

### Real-world corpus ("Museum of Horrors")

* Maintain an ingested set of non-synthetic workbooks (`fixtures/real_world/**`) from open datasets (Excel from LibreOffice/Apache POI/Office versions).
* Use corpus entries as seeds for fuzzing (Phase 3) and regression tests; never rely solely on Python-generated fixtures.
* Track provenance/allowlist via a small manifest so CI can download or cache them without bloating the repo.

---

---

## Phase 1 - Unified Container (Skeleton)

Goal: build the shared workbook skeleton. Combine the original container-open tests (M1/2) with basic sheet discovery (PG1) so both M and grid data have a home in the same IR.

### WASM compatibility guard ([G], Phase 1)

* CI step: `cargo check --target wasm32-unknown-unknown --no-default-features`.
* Purpose: fail fast if a dependency drags in host-only I/O, threads, or libc assumptions.
* Keep the core parsing/diff crates `no_std`-friendly where feasible; gate any host adapters behind feature flags.
* A deeper wasm smoke test (headless) lands in Phase 2; this Phase 1 gate is a build-only sentinel.

### Milestone 1 – “Hello bytes”: core binary & Excel container I/O

**Rust capability**

* Open a file path.
* Detect/validate that a `.xlsx` is a ZIP container.
* Read raw parts (no DataMashup yet).

**Tests**

### 1.1 Basic file opening

**Goal:** ensure you never regress on simple I/O (paths, errors, cross‑platform).

* **Unit tests (Rust)**

  * `open_file_success`: open a tiny text file in `fixtures/smoke/hello.txt`, assert bytes length > 0.
  * `open_nonexistent_file`: ensure you return a well‑typed error (not panic) with a useful error kind (e.g., `NotFound`).
  * `open_directory_instead_of_file`: error path is correct.

No Python needed here; just check in tiny fixtures.

### 1.2 "Is this a ZIP?" tests

**Goal:** robustly recognize an OPC/ZIP file (Excel / PBIX) vs random inputs.

* **Unit tests ([G] unless noted)**

  * `detect_zip_excel_file`: open `minimal.xlsx` from fixtures; verify that your ZIP detection passes.
  * `reject_non_zip`: pass a `.txt` or random bytes, assert you get a specific error (`NotAnExcelZip`).
  * `[H] reject_non_excel_zip`: feed a ZIP without `[Content_Types].xml` (not an OPC container), assert you get a `NotExcelContainer`-style error rather than a generic failure.

* **Python fixtures**

  * Script to generate `minimal.xlsx` with no Power Query at all (simple workbook with one sheet, a few constant cells).
  * `[H] random_zip.zip`: empty ZIP with a dummy text file to exercise the non-Excel ZIP case.

---

### Milestone 2 - Host container + DataMashup bytes (Excel first)

Now you implement the **host container layer** for Excel: find the `<DataMashup>` part in Excel and base64-decode it. PBIX host support moves to Phase 3.5 so you can harden the Excel path first.

**Rust capability**

* For `.xlsx/.xlsm/.xlsb`:

  * Open `[Content_Types].xml`
  * Iterate `/customXml/item*.xml`
  * Find root element `<DataMashup xmlns="http://schemas.microsoft.com/DataMashup">`
  * Base64-decode its text -> `dm_bytes`

* Keep the parser reusable so the same logic works for `.pbix/.pbit` once enabled in Phase 3.5.

**Tests**

### 2.1 “No DataMashup” vs “Exactly one”

* **Fixtures (Python)**

  * `no_power_query.xlsx`: vanilla workbook.
  * `single_mashup.xlsx`: workbook with a single trivial Power Query.
  * `two_mashups.xlsx`: contrived workbook with two `customXml` entries having `<DataMashup>` (should be invalid, but good robustness test).

  Python would use `openpyxl`/Excel COM or a pre‑baked file copied from a template where you manually created 1–2 Power Queries.

* **Integration tests (Rust)**

  * `extract_mashup_none`: from `no_power_query.xlsx`, your API should clearly report “no DataMashup found” with a benign error variant.
  * `extract_mashup_single`: from `single_mashup.xlsx`, you get non‑empty `dm_bytes`.
  * `extract_mashup_multiple`: from `two_mashups.xlsx`, you either:

    * Choose one deterministically and log/warn, or
    * Return a “multiple DataMashup parts” error. Decide behaviour and codify with the test.

### 2.2 Base64 correctness & corruption

* **Fixtures**

  * `corrupt_base64.xlsx`: same as `single_mashup.xlsx` but you byte‑flip part of the `<DataMashup>` text (Python can open the ZIP, edit that XML string).

* **Tests**

  * `corrupt_base64_errors`: ensure you detect invalid base64 and surface a clear error, not garbage bytes.

---

### PG1 – Workbook → Sheet → Grid IR sanity

Goal: the parser yields a correct `Workbook / Sheet / Grid / Row / Cell` structure from real Excel files. 

### Fixtures

1. `pg1_basic_two_sheets.xlsx`

   * Sheet `Sheet1`: 3×3 block (A1:C3) of constants (numbers + text).
   * Sheet `Sheet2`: 5×2 block (A1:B5) of constants.

2. `pg1_sparse_used_range.xlsx`

   * Sheet `Sparse`:

     * A1 and B2 non‑empty.
     * G10 non‑empty (forces used range to extend).
     * Entire row 5 and column D completely empty in between (to test “holes”).

3. `pg1_empty_and_mixed_sheets.xlsx`

   * Sheet `Empty`: completely empty.
   * Sheet `ValuesOnly`: 10×10 constants.
   * Sheet `FormulasOnly`: 10×10 simple formulas referencing `ValuesOnly`.

4. (Optional later) `pg1_merged_and_hidden.xlsx`

   * A few merged cells over A1:B2, some hidden rows/columns.
   * Purely to codify whatever IR policy you choose for merged/hidden cells (even if policy is “we ignore merges in Grid”).

### Tests

**PG1.1 – Basic workbook structure**

* Open `pg1_basic_two_sheets.xlsx`.
* Assert:

  * `Workbook.sheets.len() == 2`.
  * Sheet names `"Sheet1"` and `"Sheet2"` in order.
  * `Sheet.kind` for both is `Worksheet`.
  * `Sheet1.grid.nrows == 3`, `ncols == 3`.
  * `Sheet2.grid.nrows == 5`, `ncols == 2`.

**PG1.2 – Sparse used range → grid extents**

* Open `pg1_sparse_used_range.xlsx`.
* On `Sparse`:

  * `grid.nrows` and `grid.ncols` match Excel’s used range (should include row/col of G10).
  * `Row.index` values run from 0..(nrows‑1) with no gaps.
  * Cells:

    * There is a non‑empty cell object at `A1`, `B2`, `G10`.
    * All cells outside the used range are either absent or represented as “empty” according to your IR policy (test codifies which).

**PG1.3 – Empty vs non‑empty sheets**

* Open `pg1_empty_and_mixed_sheets.xlsx`.
* Assert:

  * `Empty.grid.nrows == 0` and `ncols == 0`; empty sheets standardize to a 0x0 used range (matching Excel's empty used range semantics).
  * `ValuesOnly.grid.nrows == 10`, `ncols == 10`.
  * `FormulasOnly.grid.nrows == 10`, `ncols == 10`.
  * For `ValuesOnly`, at least one cell has non‑`None` `value` and `formula == None`.
  * For `FormulasOnly`, at least one cell has `formula.is_some()` and `value.is_some()` (parsed formula + cached result).

**PG1.4 – (Optional) Merged / hidden policy**

* Open `pg1_merged_and_hidden.xlsx`.
* Decide your IR rules (e.g., “only top‑left cell in a merged range exists; others are logically empty”).
* Assert that the `Grid` representation of merged and hidden regions matches that rule exactly.

---

---

## Phase 2 - Basic Parsing (Parallel M + Grid)

Goal: parse binaries/XML into Rust IR for both domains and validate memory model early. Bring up addressing, snapshots, and the DataMashup framing/metadata (M3/4/5) alongside grid parsing (PG2/3/5/6).

### Streaming memory budget guard ([RC], Phase 2)

* Fixture: Python generates a ~100MB XML/Excel file with simple rows (e.g., repeating `<row><c><v>1</v></c></row>`).
* Test harness: run the parser under a 50MB heap limit (custom `GlobalAlloc`, `cap`, or OS-level limit) and assert the parse completes.
* Fails the phase if the process OOMs—this proves streaming rather than DOM loading.

### WASM smoke test (headless) ([G], Phase 2)

* Add `wasm-bindgen-test` dev-dependency and a `tests/wasm_smoke.rs` that parses a tiny embedded `.xlsx` byte array.
* Run `wasm-pack test --headless` (or equivalent webdriver runner) in CI for this single case.
* Purpose: ensure the parsing core remains pure and browser-safe beyond the Phase 1 build-only gate.

### PG2 – Addressing and index invariants

Goal: row/column indices, numeric coordinates, and `"A1"` addresses are wired up and consistent everywhere. 

### Fixtures

1. **No Excel file needed** for pure address helpers – use in‑memory tests.

2. `pg2_addressing_matrix.xlsx`

   * Single sheet `Addresses`:

     * Cells populated specifically at:

       * A1, B2, C3, Z1, Z10, AA1, AA10, AB7, AZ5, BA1, ZZ10, AAA1.
     * Each cell’s *text* equals its address (`"A1"`, `"B2"`, etc.) so you can cross‑check easily.

### Tests

**PG2.1 – index_to_address small grid**

* Pure unit tests on helper functions:

  * (0,0) → `"A1"`
  * (0,25) → `"Z1"`
  * (0,26) → `"AA1"`
  * (0,27) → `"AB1"`
  * (0,51) → `"AZ1"`
  * (0,52) → `"BA1"`

**PG2.2 – Round‑trip address_to_index**

* For a list of addresses: `["A1","B2","Z10","AA1","AA10","AB7","AZ5","BA1","ZZ10","AAA1"]`

  * `address_to_index(addr)` → `(r,c)`
  * `index_to_address(r,c)` → `addr`
  * Assert round‑trip equality.

**PG2.3 – IR cells carry correct addresses**

* Open `pg2_addressing_matrix.xlsx`.
* Iterate over all non‑empty cells in the grid:

  * For each cell `c`:

    * Assert `c.address` string equals the text value stored in that cell.
    * Assert converting `(c.row, c.col)` back to an address matches `c.address`.
* This proves the IR’s row/col indices and its notion of `CellAddress` are consistent with the Excel layout.

---

### PG3 – Cell snapshots and comparison semantics

Goal: define and verify what a “cell snapshot” (the payload inside `CellEdited`) contains and how equality works. 

### Fixtures

1. **In‑memory only**: construct `Grid` objects with hand‑built `Cell` values.

2. `pg3_value_and_formula_cells.xlsx`

   * Sheet `Types`:

     * A1: number `42`.
     * A2: text `"hello"`.
     * A3: boolean `TRUE`.
     * A4: empty cell.
     * B1: formula `=A1+1`.
     * B2: formula `="hello" & " world"`.
     * B3: formula that returns boolean (e.g., `=A1>0`).

### Tests

**PG3.1 – Snapshot from basic value cells**

* In‑memory unit tests:

  * Build a `Cell` with `value = Number(42)`, no formula, default format.
  * Call whatever function/materialization builds a `CellSnapshot`.
  * Assert:

    * Snapshot row/col/address match the cell.
    * Snapshot’s value kind is numeric, with 42.
    * Snapshot formula field is `None`.
* Repeat for text and boolean.

**PG3.2 – Snapshot from formula cells**

* Open `pg3_value_and_formula_cells.xlsx`.
* For cells B1, B2, B3:

  * Build snapshots.
  * Assert:

    * `formula` field in snapshot contains the original formula text or AST reference.
    * `value` field contains the displayed value (e.g., `43` for B1, `"hello world"` for B2, `TRUE` for B3).
    * Address/row/col are correct.

**PG3.3 – Snapshot equality semantics**

* Define (in prose, not code) your equality rule for snapshots: e.g.,

  * Equal if type + value + formula text + relevant format fields are equal; ignore volatile formatting like “last calc timestamp”.
* Unit tests:

  * Build pairs of snapshots:

    * Same value, same formula, identical format → `equal`.
    * Same formula text, different numeric result (e.g., different cached value) → either `not equal` or “equal if ignoring cache” depending on your policy (test codifies choice).
    * Same value, trivial formatting difference (if you intend to ignore some formatting) → test either equality or inequality as you intend.

**PG3.4 – Snapshot stability across parse round‑trip**

* Open `pg3_value_and_formula_cells.xlsx`, build snapshots, serialize them (e.g., to JSON) and immediately deserialize.
* Assert the deserialized snapshot equals the original snapshot under your equality rule.

---

### PG5 – In‑memory grid diff smoke tests (no Excel)

Goal: prove the core grid‑diff logic produces the right `DiffOp`s on tiny, hand‑built `Grid`s before you involve Excel parsing, row alignment, keys, etc.

### Fixtures

No files – build `Grid`/`Row`/`Cell` objects entirely in memory.

You’ll want a helper to construct small grids like:

* `grid_1x1(value)`
* `grid_2x2(values)`
* `grid_nxm(...)`

### Tests

**PG5.1 – 1×1 identical grids → empty diff**

* Build `GridA` with one row, one cell = number 1.
* Build `GridB` as an exact clone.
* Run “grid diff” that assumes both grids use `Spreadsheet Mode` and *no* row/column reordering.
* Assert the diff op list is empty.

**PG5.2 – 1×1 value change → single CellEdited**

* Same as PG5.1, but `GridB` cell = 2.
* Assert:

  * Exactly one `DiffOp`.
  * It is `CellEdited` for address `"A1"`.
  * `from` snapshot value is 1, `to` is 2.

**PG5.3 – 1×2 row appended at end**

* `GridA`: one row, one column: A1=1.
* `GridB`: two rows, one column: A1=1, A2=2.
* Run diff.
* Assert:

  * A single `RowAdded` (or contiguous block add) representing row index 1.
  * No `CellEdited` on A1.

**PG5.4 – 2×1 column appended at end**

* `GridA`: one column, two rows (A1,A2).
* `GridB`: two columns, two rows (A1,A2 plus B1,B2).
* Assert:

  * `ColumnAdded` for the new column index.
  * No `CellEdited` on existing cells.

**PG5.5 – Same shape, multiple cell edits, no structure change**

* `GridA`: 3×3 with values 1..9.
* `GridB`: same except three scattered cells changed.
* Assert:

  * Exactly three `CellEdited` ops, with correct addresses and values.
  * No `RowAdded/Removed` or `ColumnAdded/Removed`.

**PG5.6 – Degenerate grids**

* Empty vs empty: both `GridA` and `GridB` have `nrows=0`, `ncols=0`. → diff is empty.
* Empty vs 1×1: treat this as a `RowAdded` + `ColumnAdded` or as a single `CellAdded` depending on your chosen semantics; tests must pin down that choice so later code doesn’t quietly drift.

These tests ensure the diff implementation works in a vacuum before you plug in row/column alignment, keys, or Excel parsing.

---

### PG6 – Object graph vs grid responsibilities

Goal: prove that sheet‑level/object‑graph diff and grid diff are cleanly separated: adding/removing/renaming sheets doesn’t accidentally emit row/column/cell ops.

### Fixtures

1. `pg6_sheet_added_{a,b}.xlsx`

   * A:

     * Sheet `Main`: 5×5 block of constants.
   * B:

     * Sheet `Main`: identical contents.
     * Sheet `NewSheet`: small 3×3 block.

2. `pg6_sheet_removed_{a,b}.xlsx`

   * Reverse of above (B has only `Main`; A has `Main`+`OldSheet`).

3. `pg6_sheet_renamed_{a,b}.xlsx`

   * A: sheet `OldName` with some content.
   * B: same sheet renamed to `NewName`, identical grid.

4. `pg6_sheet_and_grid_change_{a,b}.xlsx`

   * A: sheet `Main` with 5x5 grid, sheet `Aux` untouched.
   * B:

     * `Main`: same size but a couple of cell edits.
     * `Aux`: unchanged.
     * New sheet `Scratch`: some constants.

5. `pg6_renamed_and_changed_{a,b}.xlsx`

   * A: sheet `Summary` with a small data block.
   * B: same sheet renamed to `P&L` with a couple of cell edits inside.

### Tests

**PG6.1 – Sheet addition doesn’t trigger grid ops on unchanged sheets**

* Diff `pg6_sheet_added_a.xlsx` vs `pg6_sheet_added_b.xlsx`.
* Assert:

  * Exactly one `SheetAdded` (`"NewSheet"`).
  * No `Row*` / `Column*` / `Cell*` ops for `Main`.

**PG6.2 – Sheet removal symmetrical**

* Diff `pg6_sheet_removed_a.xlsx` vs `pg6_sheet_removed_b.xlsx`.
* Assert:

  * Exactly one `SheetRemoved` (`"OldSheet"`).
  * No grid ops for `Main`.

**PG6.3 – Sheet rename vs remove+add**

Depending on your object graph diff design: 

* If you support `SheetRenamed`:

  * Diff `pg6_sheet_renamed_a.xlsx` vs `pg6_sheet_renamed_b.xlsx`.
  * Assert:

    * One `SheetRenamed { from: "OldName", to: "NewName" }`.
    * No grid ops.
* If you *don’t* support renames (treat as remove+add):

  * Assert:

    * One `SheetRemoved("OldName")`, one `SheetAdded("NewName")`.
    * Still no grid ops.
* Either way, this proves rename/add/remove does not cascade into bogus cell diffs.

**PG6.4 ? Sheet & grid changes composed cleanly**

* Diff `pg6_sheet_and_grid_change_a.xlsx` vs `_b.xlsx`.
* Assert:

  * `SheetAdded("Scratch")`.
  * For `Main`: whatever `CellEdited`/`Row*`/`Column*` ops are appropriate for the few cell tweaks.
  * No grid ops for `Aux`.
* This ensures that the object-graph layer and grid layer both fire, but in a controlled, separable way.

**PG6.5 ? Rename plus grid edits semantics**

* Diff `pg6_renamed_and_changed_a.xlsx` vs `_b.xlsx`.
* Assert (choose and codify one policy):

  * Preferred: `SheetRenamed("Summary" -> "P&L")` plus the specific `CellEdited` ops for the changed cells.
  * If you treat renames as remove+add: `SheetRemoved("Summary")` + `SheetAdded("P&L")` plus the cell edits for that new sheet.
* No other grid ops on unchanged sheets; this nails down rename semantics when content also changes.

### Milestone 3 – MS‑QDEFF top‑level framing

Now you parse `dm_bytes` into the `Version + 4 length‑prefixed sections` structure. 

**Rust capability**

* Given `dm_bytes`:

  * Read `Version` (u32 LE)
  * Read 4 length fields (u32 LE each)
  * Slice buffer into: `package_bytes`, `permissions_bytes`, `metadata_bytes`, `permission_bindings_bytes`
  * Enforce invariants (version, sizes, no overflow).

**Tests**

### 3.1 “Happy path” framing

* **Fixture source**

  * You can generate `dm_bytes` synthetically in Rust for unit tests (bypass Excel) since this is pure binary. For integration tests, use `single_mashup.xlsx` and run host‑extraction first.

* **Unit tests**

  * `parse_minimal_zero_lengths`: construct a `dm_bytes` buffer with:

    * `Version = 0`
    * All length fields = 0
    * Total size = 4 + 4*4
    * Assert you get empty slices for all four sections and no error.
  * `parse_basic_nonzero_lengths`: synthetic buffer where each section is e.g., `b"AAAA"`, `b"BBBB"`, etc. Assert all slices match exactly.

### 3.2 Invariant tests (negatives)

From your blueprint: version must be 0 for now, lengths must fit within the buffer. 

* **Unit tests**

  * `reject_future_version`: set `Version = 1`, assert either a `FutureVersion` warning/error depending on your policy.
  * `reject_truncated_stream`: create buffer where `PackagePartsLength` is larger than actual bytes; ensure `BoundsError` not panic.
  * `reject_overflow_sum`: craft lengths whose sum > buffer; parser must fail gracefully.

### 3.3 Round‑trip framing

* **Integration tests**

  * Extract `dm_bytes` from a real workbook, parse into sections, then re‑emit the header (keeping section bytes identical) and confirm it equals the original `dm_bytes`. This proves your slicing is correct and non‑destructive.

Python is not needed for these tests beyond providing the original workbook.

---

### Milestone 4 – Semantic sections: PackageParts / Permissions / Metadata / Bindings

Now you implement the **semantic layer** per the blueprint: treat `PackageParts` as ZIP, parse Permissions XML, Metadata XML, and treat Permission Bindings as opaque. 

**Rust capability**

* From the four slices, you can construct:

```text
struct DataMashup {
    version: u32,
    package_parts: PackageParts,
    permissions: Permissions,
    metadata: Metadata,
    permission_bindings_raw: Vec<u8>,
}
```



### 4.1 PackageParts / OPC tests

**Fixtures (Python)**

* `one_query.xlsx`: workbook with a single query `Section1/Foo`.
* `multi_query_with_embedded.xlsx`: workbook:

  * Several queries.
  * At least one `Embedded.Value` using `/Content/{GUID}` (e.g., referencing a small static table). 

Python can create these by:

* Recording Excel macros or using COM automation to add Power Queries; or
* If that’s too annoying, start from a manually built template `.xlsx` checked into the fixtures repo and only tweak cells, not the mashup.

**Tests**

* `package_parts_contains_expected_entries`:

  * Open `one_query.xlsx`
  * Ensure:

    * `/Formulas/Section1.m` exists
    * `/Config/Package.xml` exists
    * `/Content/…` is empty
* `embedded_content_detection`:

  * Open `multi_query_with_embedded.xlsx`
  * Ensure:

    * `/Content/{GUID}` entries are found
    * Each can be opened as a nested OPC package
    * Their own `/Formulas/Section1.m` is non‑empty

### 4.2 Permissions XML tests

**Fixtures**

* `permissions_defaults.xlsx`: workbook with Power Query and default privacy/firewall settings.
* `permissions_firewall_off.xlsx`: workbook where you explicitly set “Ignore privacy level checks” in Power Query.

**Tests**

* `permissions_parsed_flags`:

  * Confirm `FirewallEnabled` and `WorkbookGroupType` (or equivalent) flip between the two fixtures.
  * If XML is missing or malformed, default values are correct (design tests for that too, e.g., by corrupting the Permissions bytes manually).

### 4.3 Metadata XML tests

Per blueprint, Metadata XML has `LocalPackageMetadataFile` with `Formulas` entries keyed by `SectionName/FormulaName`, plus load destinations etc. 

**Fixtures**

* `metadata_simple.xlsx`: 2 queries:

  * `Section1/LoadToSheet` → load to table on sheet.
  * `Section1/LoadToModel` → load only to data model.
* `metadata_query_groups.xlsx`: queries organized in folders (Power Query groups).
* `metadata_hidden_queries.xlsx`: some queries are connection‑only / not loaded.

**Tests**

* `metadata_formulas_match_section_members`:

  * Parse Metadata XML.
  * Parse `Section1.m` into members.
  * Assert number of `ItemType=Formula` entries equals number of shared members in `Section1.m` (minus known oddities like step entries) – your blueprint already calls this out as an invariant. 
* `metadata_load_destinations`:

  * For each query, assert load settings from Metadata (sheet vs model vs both) match what you manually configured when creating the fixtures.
* `metadata_groups`:

  * Assert group hierarchy (e.g., “Inputs/DimTables”) is correctly mapped.

### 4.4 Permission Bindings

At this stage you don’t need to decrypt/validate; you just surface presence. 

* `permission_bindings_present_flag`:

  * On a normal workbook, `has_bindings` should be true if the field is non‑empty.
* `permission_bindings_missing_ok`:

  * On a contrived `dm_bytes` with zero‑length bindings, ensure you handle gracefully.

---

### Milestone 5 – Domain layer: M queries & metadata API

Here you build an ergonomic API like:

```text
struct Query {
    name: String,           // "Section1/Foo"
    section_member: String, // "Foo"
    expression_m: String,   // raw M code for this query
    metadata: QueryMetadata
}
```



**Rust capability**

* Split `Section1.m` into named members.
* Associate each member with its metadata entry by `SectionName/FormulaName`.
* Produce an ordered `Vec<Query>` and top‑level `Metadata` object.

**Tests**

### 5.1 Section1.m splitting

**Fixtures**

* `section_single_member.m` (unit test only).
* `section_multiple_members.m` with:

  * `section Section1;`
  * `shared Foo = ...;`
  * `shared Bar = ...;`
  * Private member `Baz = ...;`

**Unit tests**

* `parse_single_member_section`:

  * Ensure you get one `section_member: "Foo"`, `expression_m` contains exactly that body.
* `parse_multiple_members`:

  * Shared members recognized as queries; private ones either included/excluded depending on API design. Tests codify that decision.
* `tolerate_whitespace_comments`:

  * Add comments and random blank lines; ensure splitting still works.

### 5.2 Query–metadata join

**Integration tests (using real Excel)**

* `metadata_join_simple`:

  * From `metadata_simple.xlsx`, ensure:

    * Query list has two queries: `Section1/LoadToSheet`, `Section1/LoadToModel`.
    * Each has metadata where `load_to_sheet`, `load_to_model` flags match expectations.
* `metadata_join_url_encoding`:

  * Create a query with special characters in the name that require URL encoding (`"Query with space & #"`).
  * Ensure you correctly map metadata `ItemPath` to section member after decoding.

### 5.3 Domain invariants

* `query_names_unique`:

  * Ensure `name` is unique within a DataMashup instance.
* `metadata_orphan_entries`:

  * Build a fixture where metadata lists a `Section1/Nonexistent` formula (edit XML manually). Your parser should:

    * Either drop it with a warning, or
    * Surface it as an “orphan” entry; the test should assert your chosen behaviour.

---

---

## Phase 3 - MVP Diff (Vertical Slice)

Goal: first end-to-end diff slice. Define the DiffOp contract, ship textual M diffs, and land the simplest spreadsheet-mode diffs (G1?G7) so the product can compare real files even before advanced alignment.

### Early DataMashup fuzzing (bit-flip) [E?G if stable]

* Move fuzz hardening up from later phases: add a `cargo-fuzz` target that mutates length prefixes and payload bytes of the `DataMashup` framing parser.
* Seed corpus with valid headers extracted from golden fixtures plus a few “weird” real-world workbooks (LibreOffice/Apache POI outputs).
* Success: no panics; parser returns `Result::Err` for corrupted streams. Run for at least 1 hour in CI/nightly.

### PG4 – DiffOp plumbing & wire contract

Goal: all `DiffOp` variants exist, serialize properly, and can be round‑tripped independently of any real diff algorithm.

### Fixtures

No Excel fixtures required – this is purely type‑level / serialization testing.

### Tests

**PG4.1 – Construct each DiffOp variant**

* Unit tests that *manually* construct:

  * `CellEdited { sheet, addr, from: snapshot1, to: snapshot2 }`
  * `RowAdded { sheet, row_idx, row_signature, … }`
  * `RowRemoved { … }`
  * `ColumnAdded`, `ColumnRemoved`, `BlockMovedRows`, `BlockMovedColumns` (or whatever set you decide).
* For each, assert that:

  * Mandatory fields are present and non‑default.
  * Optional fields behave as intended (e.g., `block_hash` maybe `None`).

**PG4.2 – JSON (or wire‑format) shape**

* For each DiffOp instance from PG4.1:

  * Serialize to your wire format (probably JSON for CLI/web).
  * Assert (as string or as parsed JSON object) that:

    * Enum tags are as documented (e.g., `"kind": "CellEdited"` or similar).
    * Sheet identifiers, addresses, and snapshot payloads appear under the expected keys.
    * No extraneous / internal fields leak through.

**PG4.3 – Round‑trip stability**

* Deserialize the JSON back to a `DiffOp`.
* Assert:

  * The variant type is identical.
  * Key fields (`sheet`, `addr`, `from`, `to`) match the original.
* This codifies the contract between engine and frontends/CLI that the meta‑process relies on.

**PG4.4 – DiffOp list / report container**

* Construct a “fake report” with a small vector/list of DiffOps mixing several variants.
* Serialize and deserialize the whole collection.
* Assert order and contents are preserved.
* This is the object you’ll later stream out of the end‑to‑end diff pipeline, so locking it early avoids painful schema changes later.

---

### Milestone 6 – Textual M diff engine (first working diff)

Before AST semantic diffing, you’ll probably stand up a simpler text‑level diff for M code so you can ship something sooner and then harden it.

**Rust capability**

* Given two `DataMashup` domain objects:

  * Align queries by `name` (Section1/QueryName).
  * Report:

    * Added queries
    * Removed queries
    * Changed queries (text diff on `expression_m`)
    * Metadata‑only changes

Think of a `MQueryDiff` type like:

```text
enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String },
    DefinitionChanged,
    MetadataChangedOnly
}
```

(You might not support rename yet; tests can initially treat renames as removed+added, then evolve.)

**Tests**

### 6.1 Basic M diffs

**Fixtures (Python)**

* `m_add_query_{a,b}.xlsx`:

  * A: one query `Foo`.
  * B: queries `Foo`, `Bar` (same result data).
* `m_remove_query_{a,b}.xlsx`:

  * Reverse of the above.
* `m_change_literal_{a,b}.xlsx`:

  * A: `Foo` with `= 1`.
  * B: `Foo` with `= 2`.
* `m_metadata_only_change_{a,b}.xlsx`:

  * A: `Foo` loads to sheet.
  * B: `Foo` load destination is model only; M code identical.

**Integration tests**

For each `{a,b}` pair:

* Run full pipeline: open Excel → extract DataMashup → domain queries → diff.
* Assert:

  * `m_add_query`: exactly one diff with kind `Added` for `Bar`.
  * `m_remove_query`: one `Removed` for `Bar`.
  * `m_change_literal`: single `DefinitionChanged` for `Foo`; optionally assert the text diff shows changed literal.
  * `m_metadata_only_change`: `MetadataChangedOnly` for `Foo` (no `DefinitionChanged`).

### 6.2 Renames (optional first pass)

**Fixture**

* `m_rename_query_{a,b}.xlsx`:

  * A: query `Foo` with body `= 1`.
  * B: query `Bar` with identical body and metadata.

**Test**

* Initially, your engine may report `Removed(Foo)` and `Added(Bar)`. Codify that behaviour in tests.
* Later, when you add rename detection (e.g., based on identical expression + metadata), update the test to assert a single `Renamed { from: "Foo", to: "Bar" }`.

### 6.3 Embedded contents

**Fixture**

* `m_embedded_change_{a,b}.xlsx`:

  * Both versions have same top‑level queries.
  * Only change is inside `/Content/{GUID}/Formulas/Section1.m` for an embedded mini‑mashup used by `Embedded.Value`.

**Tests**

* Decide domain model:

  * Either treat embedded contents as separate “queries” or as an attribute of the parent query.
* Then assert:

  * A diff exists precisely at the embedded content location.
  * No spurious changes reported on unrelated queries.

---

These assume you already have `Workbook` → `Sheet` → `Grid` IR and basic DiffOp types like `CellEdited`, `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `BlockMoved`.

Each milestone has:

* **Core capability** — what the engine must correctly do.
* **Fixture sketch** — what kind of workbook(s) to generate.
* **Checks** — what the diff output should look like.

---

### G1 – Identical sheet → empty diff

**Core capability**

The grid diff engine can compare two small, identical sheets and produce **no grid‑level DiffOps**.

**Fixture sketch**

* `equal_sheet_{a,b}.xlsx`

  * One worksheet, 5×5 grid, simple constants (numbers, strings).
  * B is a byte‑for‑byte copy of A.

**Checks**

* Diff emits **no** `RowAdded/Removed`, `ColumnAdded/Removed`, or `CellEdited`.
* Any workbook/sheet‑level “compared these two sheets” metadata is present, but **diff list is empty**.

---

### G2 – Single cell literal change

**Core capability**

Detect one changed value in an otherwise identical grid and surface a single `CellEdited` op.

**Fixture sketch**

* `single_cell_value_{a,b}.xlsx`

  * A: 5×5 grid, constants.
  * B: identical except `C3` changes from `1` to `2`.

**Checks**

* Exactly one `CellEdited` for `C3`.
* Old/new value snapshots correct; no row/column structure ops emitted.

---

### G3 – Single cell formula change (same value vs different value)

**Core capability**

Distinguish **formula change** from **value change**, using formula ASTs where available.

**Fixture sketch**

* `single_cell_formula_same_result_{a,b}.xlsx`

  * A: `C3 = A3 + B3`.
  * B: `C3 = B3 + A3` (commutative, same result for all current inputs).
* `single_cell_formula_diff_result_{a,b}.xlsx`

  * A: `C3 = SUM(A1:A10)`.
  * B: `C3 = AVERAGE(A1:A10)`.

**Checks**

* For “same result” pair: canonicalized ASTs equal → either:

  * No `CellEdited` at all, **or**
  * A dedicated “FormulaFormattingOnly” flag, but *not* a semantic change.
* For “different result” pair: `CellEdited` at `C3` with a **formula‑change detail**, not just a raw “value changed”.

---

### G4 – Format‑only change vs content change

**Core capability**

Ensure pure formatting edits don’t show up as logical changes when configuration says “ignore formatting”.

**Fixture sketch**

* `format_only_{a,b}.xlsx`

  * A: 5×5 numbers, default formatting.
  * B: same values & formulas, but some cells bold, different colors, different number formats.

**Checks**

* With “ignore formatting” option on:

  * No grid diff ops at all.
* With “show formatting” option on:

  * Diffs are typed as “format changed” (whatever format DiffOps you define), but **no row/column/cell content ops**.

---

### G5 – Multiple independent cell edits in a fixed grid

**Core capability**

Handle multiple scattered cell edits without collapsing them into bogus row/column structure changes.

**Fixture sketch**

* `multi_cell_edits_{a,b}.xlsx`

  * A: 20×10 grid of constants.
  * B: 5–10 cells scattered across different rows and columns changed (mix of numbers and strings).

**Checks**

* Diff lists exactly those addresses as `CellEdited` (correct old/new values).
* No `RowAdded/Removed` or `ColumnAdded/Removed` for this scenario.

---

### G6 – Simple row append / truncate (bottom of sheet)

**Core capability**

Detect rows appended or removed **at the end** of a sheet and surface explicit row add/remove ops.

**Fixture sketch**

* `row_append_bottom_{a,b}.xlsx`

  * A: rows 1–10 with simple sequential IDs in column A.
  * B: rows 1–12 (rows 11–12 newly added).
* `row_delete_bottom_{a,b}.xlsx`

  * A: rows 1–12.
  * B: rows 1–10.

**Checks**

* Append case: two `RowAdded` ops for logical row indices 11 and 12; no spurious `CellEdited`.
* Delete case: two `RowRemoved` ops for 11 and 12.

---

### G7 – Simple column append / truncate (right edge)

**Core capability**

Symmetric to G6 but for columns.

**Fixture sketch**

* `col_append_right_{a,b}.xlsx`

  * A: columns A–D filled.
  * B: A–F (E and F new).
* `col_delete_right_{a,b}.xlsx`

  * A: columns A–F.
  * B: columns A–D.

**Checks**

* Append: `ColumnAdded` for E, F; no cell‑by‑cell updates.
* Delete: `ColumnRemoved` for E, F.

---

--- 

## Phase 3.5 - PBIX host support (post-Excel)

Goal: reuse the Excel DataMashup parser for `.pbix/.pbit` containers once framing/metadata (Milestones 3–5) are stable. This is release-gating for PBIX, but `[H]` for the Excel-first MVP.

### PBIX presence/absence

**Fixtures (Python) [H]**

* `legacy.pbix`: a small PBIX with queries (includes a `DataMashup` file).
* `enhanced_metadata.pbix`: PBIX where Power BI no longer stores `DataMashup` (tabular model only).

**Integration tests ([H] until PBIX ships)**

* `extract_mashup_pbix_legacy`: confirm you find `DataMashup` and produce bytes via the shared Excel parser.
* `extract_mashup_pbix_enhanced`: return a structured domain error like `NoDataMashupUseTabularModel` (tabular-only path), not a panic.

---

## Phase 4 - Algorithmic Heavy Lifting

Goal: tackle the hardest algorithms. Add semantic M AST diffing (M7), advanced spreadsheet alignment/moves (G8?G12), and the database-mode keyed diffs (D1?D10) that exercise H1/H4 risks.

### Milestone 7 – Semantic (AST) M diffing

This is where your differentiator shows up: abstract‑syntax‑tree diffing rather than raw text diff. 

**Rust capability**

* Parse `expression_m` into AST.
* Normalize irrelevant differences:

  * Whitespace
  * Comments
  * Possibly step order when it’s semantically irrelevant (careful here).
* Compare ASTs for semantic equality / produce a semantic diff.

**Tests**

### 7.1 Formatter round-trip is a no-op

**Fixtures**

* `m_formatting_only_{a,b}.xlsx`:

  * A: ugly M with no newlines.
  * B: the same query run through a different pretty-printer (re-indented, newlines/comments shuffled), identical semantics.
* `m_formatting_only_{b_variant}.xlsx`:

  * Same as B but with a single identifier tweak (e.g., `Table1` -> `Table2`) for a negative-control diff.

**Tests**

* Parse A vs B to AST and diff: **zero** semantic DiffOps allowed (formatting-only changes must not surface).
* Parse A vs `b_variant`: expect a single semantic change (identifier rename or load target) to be reported.

### 7.2 Reordering non‑dependent steps

If you choose to treat some reorders as semantic‑no‑ops, you need tests for it.

* `m_step_reorder_{a,b}.xlsx`:

  * A & B differ only by the order of independent `let` bound steps.
* Assert semantic diff is empty.

### 7.3 Specific semantic changes

Test cases that map directly onto user‑visible Power Query edits:

* `m_filter_added_{a,b}.xlsx`:

  * A: query without filter step.
  * B: same query with a `Table.SelectRows` filter on a column (“Region = EMEA”).
* `m_column_removed_{a,b}.xlsx`:

  * A: includes a `RemovedOtherColumns` step keeping `["A","B","C"]`.
  * B: keeps `["A","B"]` only.

Tests should assert:

* There is a `DefinitionChanged` with structured detail, e.g.,

  * Change type: “step added” with name and signature.
  * Or “filter predicate changed on column Region from `<> null` to `= "EMEA"`”.

You don’t have to implement the reporting format yet, but the test can at minimum assert that:

* Exactly one “semantically significant” change is reported for the query.
* The description mentions the step’s name or type.

---

### Spreadsheet-Mode advanced alignment (G8?G12)

#### G8a – Adversarial repetitive patterns [RC]

**Core capability**

Catch worst-case Hunt-Szymanski behaviour on highly repetitive signatures before it lands in prod.

**Fixture sketch**

* `adversarial_grid_{a,b}.xlsx`:

  * Rows of near-duplicate signatures (e.g., row of `A,B,C,D...` vs `A,A,B,B,C,C...`) across thousands of rows/cols with slight offsets.
  * Generates many plausible alignments to stress the edit graph.

**Checks**

* Wrap diff in a strict timeout (e.g., <500ms or a small multiple of linear baseline).
* Expect a deterministic, bounded set of DiffOps (row/col moves or edits) with no timeout or explosive op counts.

#### G8 – Single row insert/delete in the middle (row alignment)

**Core capability**

Use row signatures / LCS so inserting or deleting a single row in the **middle** of the sheet doesn’t mark everything below as changed.

**Fixture sketch**

* `row_insert_middle_{a,b}.xlsx`

  * A: rows 1–10 with an ID column and stable content.
  * B: identical except a new row inserted between 5 and 6.
* `row_delete_middle_{a,b}.xlsx`

  * A: rows 1–10.
  * B: identical except row 6 removed.

**Checks**

* Exactly **one** `RowAdded` or `RowRemoved` at the appropriate logical position.
* Rows below the insertion/deletion (7–10) are aligned correctly — no `CellEdited` / phantom changes.

---

#### G9 – Single column insert/delete in the middle (column alignment)

**Core capability**

Column‑signature LCS alignment works just like row alignment when a column is inserted or removed in the middle.

**Fixture sketch**

* `col_insert_middle_{a,b}.xlsx`

  * A: columns A–H with stable header row and data.
  * B: new column inserted between C and D.
* `col_delete_middle_{a,b}.xlsx`

  * A: A–H.
  * B: without column D.

**Checks**

* One `ColumnAdded` / `ColumnRemoved`.
* Cells in columns after the insertion/deletion line up correctly and only show diffs where actual content changed.

---

#### G10 – Contiguous block of rows inserted / deleted

**Core capability**

Treat a **block of adjacent rows** as a block add/remove, not as a mixture of adds and edits.

**Fixture sketch**

* `row_block_insert_{a,b}.xlsx`

  * A: rows 1–10.
  * B: an additional block of rows 4–7 inserted, with distinctive content.
* `row_block_delete_{a,b}.xlsx`

  * A: rows 1–10.
  * B: rows 4–7 removed.

**Checks**

* Either:

  * One `BlockAddedRows { start_row, count }` / `BlockRemovedRows`, **or**
  * A sequence of four `RowAdded`/`RowRemoved` with contiguous indices and no spurious `CellEdited`.
* Rows outside the block perfectly aligned.

---

#### G11 – Block move (rows) detection

**Core capability**

Detect when a contiguous block of rows has **moved** rather than been removed and re‑added, using block hashing / similarity.

**Fixture sketch**

* `row_block_move_{a,b}.xlsx`

  * A: rows 1–20 with a distinctive 4‑row block (e.g., rows 5–8 tagged “BLOCK”).
  * B: same data, but that 4‑row block moved to rows 13–16 with identical content.

**Checks**

* Diff emits a single `BlockMovedRows { from: 5..8, to: 13..16 }` (or equivalent), **not** 4 removes + 4 adds.
* No `CellEdited` inside the moved block.

---

#### G12 – Column / rectangular block move

**Core capability**

Same as G11, but for columns and 2D rectangular blocks.

**Fixture sketch**

* `column_move_{a,b}.xlsx`

  * A: columns A–H, with column C clearly distinguishable.
  * B: column C moved to position F.
* `rect_block_move_{a,b}.xlsx`

  * A: a 3×3 data block at (rows 3–5, cols B–D).
  * B: same block moved to a new location but unchanged internally.

**Checks**

* `BlockMovedColumns` or equivalent for column move.
* For rectangle: a single rectangular move op (or combination of row/column moves) and no `CellEdited` in the block.

---

These milestones exercise the **keyed, database‑mode diff** where row order is irrelevant and rows are matched on primary keys.

---

### D1 – Keyed equality (no differences)

**Core capability**

When a sheet/table is in Database Mode with a known key, identical data produces **no row/cell diffs**, regardless of row order.

**Fixture sketch**

* `db_equal_ordered_{a,b}.xlsx`

  * A: table with columns `[ID, Name, Amount]`, IDs 1..10.
  * B: same rows, same order.
* `db_equal_reordered_{a,b}.xlsx`

  * B: same rows but randomly permuted.

**Checks**

* In both pairs, diff yields no `RowAdded/Removed` and no `CellEdited`.
* In the reordered pair, engine successfully matches by key instead of row index.

---

### D2 – Single keyed row added / removed

**Core capability**

Treat new/missing keys as single row add/remove events.

**Fixture sketch**

* `db_row_added_{a,b}.xlsx`

  * A: IDs 1..10.
  * B: IDs 1..11 (new ID = 11).
* `db_row_removed_{a,b}.xlsx`

  * A: IDs 1..11.
  * B: IDs 1..10.

**Checks**

* Exactly one `RowAdded` / `RowRemoved` with key = 11.
* No cell edits on shared keys.

---

### D3 – Keyed row updated (non‑key column changes)

**Core capability**

Detect changes to non‑key fields on an existing key as **cell edits**, not new rows.

**Fixture sketch**

* `db_row_update_{a,b}.xlsx`

  * Same set of IDs.
  * For ID = 7, `Amount` changes from 100 to 120; all other columns equal.

**Checks**

* One `CellEdited` (for that row’s `Amount`), keyed to ID = 7.
* No row add/remove events.

---

### D4 – Pure reorder vs structural diff

**Core capability**

Prove reordering alone is ignored, but reordering *plus* changes still surfaces changes correctly.

**Fixture sketch**

* `db_reorder_only_{a,b}.xlsx`

  * A and B identical except random row order.
* `db_reorder_and_change_{a,b}.xlsx`

  * Same as above, plus one record’s `Amount` changed.

**Checks**

* Reorder‑only: empty diff.
* Reorder+change: diff only lists cell edits on changed keys; no structural ops.

---

### D5 – Composite primary key

**Core capability**

Support multi‑column keys and match rows correctly on their combination.

**Fixture sketch**

* `db_composite_key_{a,b}.xlsx`

  * Key is `[Country, CustomerID]`.
  * Add a row that creates a new `[Country, CustomerID]` pair; change one non‑key column for an existing pair.

**Checks**

* New combination → `RowAdded`.
* Existing combination with changed non‑key field → `CellEdited`.
* No false matches when only one part of composite key matches.

---

### D6 – Duplicate key clusters surfaced explicitly

**Core capability**

Detect duplicate keys on either side and expose them as a special case (cluster diff) rather than silently mis‑matching.

**Fixture sketch**

* `db_duplicate_keys_{a,b}.xlsx`

  * A: ID 5 appears twice with slightly different data.
  * B: ID 5 appears twice with a different pair of rows.

**Checks**

* Engine surfaces a `DuplicateKeyCluster { key: 5, left_rows, right_rows }` or equivalent.
* Within that cluster, small Hungarian / best‑match logic can be tested (e.g., each left row paired with the closest right row).
* No assumption that there is a single canonical row for that key.

---

### D7 – User‑provided key vs metadata vs heuristic

**Core capability**

Respect **explicit key choice** over metadata or heuristic inference.

**Fixture sketch**

* `db_key_priority_{a,b}.xlsx`

  * Sheet with columns `[RowID, CustomerID, Name]`.
  * Metadata suggests table key is `RowID`.
  * User specifies key = `CustomerID`.

**Checks**

* Diff uses `CustomerID` as key:

  * Reordering by `RowID` does not cause diffs.
  * Changes that only affect `RowID` (with constant customer) are not treated as row identity changes.
* A separate test where user *doesn’t* supply key checks that metadata key is used instead.

---

### D8 – Simple heuristic key inference (unambiguous)

**Core capability**

Infer a key when neither user nor metadata provides one, using uniqueness / null‑rate heuristics.

**Fixture sketch**

* `db_infer_key_simple_{a,b}.xlsx`

  * Columns `[Index, Name, Amount]`.
  * `Index` is 1..N, unique, no blanks. Other columns have duplicates.
  * B: reorders rows, changes one `Amount`.

**Checks**

* Engine chooses `Index` as inferred key; reorder only yields no diffs, changed `Amount` yields a single `CellEdited`.
* Expose inferred key in diff report (for debug / transparency).

---

### D9 – Ambiguous key inference → safe fallback

**Core capability**

When no clear key exists (e.g., all columns have duplicates or nulls), fall back safely (e.g., row‑index or spreadsheet‑mode diff) and clearly report that key inference failed.

**Fixture sketch**

* `db_infer_key_ambiguous_{a,b}.xlsx`

  * Small table where all columns have many duplicates and nulls.
  * B: reorder rows.

**Checks**

* Engine reports “no reliable key” / Database Mode fallback.
* Diff behavior is documented:

  * Either treat as Spreadsheet Mode (so reorder shows as row moves), **or**
  * Treat row index as implicit key, so reorder shows as structure diff.
* The important part is: **no bogus “no diff”** when data has truly changed; and behavior is deterministic.

---

### D10 – Mixed sheet: database table region + free‑form grid

**Core capability**

Handle a sheet that has a structured, keyed table region (Database Mode) plus free‑form cells around it (Spreadsheet Mode) and diff each region appropriately.

**Fixture sketch**

* `db_mixed_sheet_{a,b}.xlsx`

  * Table `Sales` (keyed by `OrderID`) in range `B3:F100`.
  * Free‑form commentary / formulas above and to the right.
  * B: changes some table rows and some standalone cells, reorders the table rows.

**Checks**

* Table region:

  * Reorder ignored; only true data changes surfaced as keyed row/cell diffs.
* Free‑form region:

  * Spreadsheet Mode semantics: cell and possibly row/column structure diffs.
* No cross‑contamination where free‑form cells get treated as part of the table, or vice versa.

---

---

## Phase 5 - Polish & Production

Goal: polish and production hardening. Cover workbook-level M scenarios, fuzzing/regressions, performance suites (including grid perf P1/P2), and product-level CLI/API contracts.

### Milestone 8 – Workbook‑level M diff scenarios (end‑to‑end)

Now you move from micro‑cases to realistic, multi‑query workbooks – the kind of thing you’d demo.

**Rust capability**

* Given two Excel files, produce a full “M diff report” at workbook scope:

  * Query additions/removals/renames
  * Per‑query semantic diffs
  * Metadata diffs (load destinations, groups, privacy flags)
  * Summary stats (“3 queries changed, 2 added, 1 removed”)

**Fixtures (Python)**

Here your Python repo shines. Define named scenarios and let Python build both A and B:

1. **“ETL Pipeline with staging + facts + dimensions”**

   * A: 5–10 queries:

     * `StgSales`, `StgCustomers`, `DimDate`, `DimCustomer`, `FactSales`.
   * B: modifications:

     * Add a new dimension query (`DimProduct`).
     * Change filter in `StgSales` to drop test data.
     * Change group of `DimCustomer` (“Dimensions / Customers” -> “Master Data”).

2. **“Broken but structurally similar pipeline”**

   * A & B have same queries and steps, but in B:

     * One key step changes join type from Inner to LeftOuter.

3. **“Query load changes only”**

   * A: some queries load to sheet; B: those toggled to model only.

4. **“Mixed Excel+PBIX sample set”**

   * A: `.xlsx`; B: `.pbit` with equivalent queries (for future cross‑host consistency tests).

**End‑to‑end tests**

* `pipeline_scenario_detailed_diff`:

  * Run full diff on scenario (1).
  * Assert high‑level expectations:

    * `added_queries` count is 1; name `DimProduct`.
    * `changed_queries` includes `StgSales` with at least one semantic diff.
    * Group membership diff reported for `DimCustomer`.
* `join_type_change_surface_cause`:

  * Scenario (2): ensure diff explanation clearly indicates join type change (not just “Text changed”).
* `load_settings_only`:

  * Scenario (3): check that queries appear as `MetadataChangedOnly` without `DefinitionChanged`.

These tests double as validation that your earlier, more granular tests actually compose correctly.

---

### Milestone 9 – Fuzzing, golden tests, and regression harness

Once the basics work, you want to be very hard to break.

### 9.1 Golden oracles (Data Mashup Explorer / Cmdlets)

Your parser blueprint already suggests using Ben Gribaudo’s tools as oracles. 

* **Offline golden files**

  * For a subset of fixtures, check in:

    * The output of `Export-DataMashup -Raw` (Section1.m) and `Export-DataMashup -Item Metadata`.
  * Write tests that:

    * Parse workbook with Rust.
    * Serialize its internal representation back to JSON.
    * Compare against golden JSON (loose comparison to allow for field ordering differences).

This catches subtle schema interpretation bugs.

### 9.2 Property‑based tests on the binary framing

From your blueprint’s own suggestions: test invariants like length sums. 

* **Property tests**

  * Randomly generate valid `dm_bytes` headers and random body slices; ensure your parser never panics and always:

    * Either returns consistent slices or
    * Flags a specific error for invalid cases.
  * Randomly corrupt lengths, bytes, etc., and assert you get facing errors.

### 9.3 Differential fuzzing vs binwalk / OPC library

Use a binwalk‑like scan as a sanity check that your `PackageParts` slice really contains a ZIP with entries where you think they are. 

* For randomly selected fixtures:

  * Run your framing parser.
  * Independently scan `dm_bytes` with binwalk or a Rust ZIP library to find PK signatures.
  * Assert that your `package_parts` slice covers exactly the region that contains the primary ZIP.

### 9.4 Regression harness

Whenever you find a real‑world workbook that breaks something:

* Drop it into `fixtures/bugs/{issue-id}`.
* Add:

  * A focused unit/integration test describing the failure.
  * If useful, also a simplified hand-crafted reconstruction for faster tests.
* This gradually builds a "museum" of bad but realistic inputs.

### 9.5 Grid diff property/fuzz tests

**Property tests [E]**

* Generate small random grids, apply random edit sequences (cell edits, row/column inserts or deletes, simple block moves).
* Diff original vs mutated and assert:

  * Every emitted `DiffOp` matches an applied mutation; no "extra" ops.
  * Applying the diff ops back to A reconstructs B for simple cases.
* Keep grid sizes tiny for CI but run enough seeds to shake out alignment bugs, mirroring the DataMashup framing property tests.

---

### Milestone 10 – Performance & scalability testing

This ties into the “Compare 100MB in under 2 seconds” claim in your product plan. 

**Fixtures (Python)**

Generate large synthetic workbooks:

* Many queries (hundreds).
* Deep `let` chains.
* Large embedded contents.

Python can automate:

* Cloning a base query and tweaking a literal per copy.
* Creating minimal data tables to keep file size realistic but manageable.

**Perf tests**

* Use a non‑test harness (e.g., `cargo bench` or a small benchmark binary) rather than unit tests, but keep them well‑defined:

  * `parse_only_large_workbook`: time to open Excel → DataMashup → domain.
  * `diff_large_workbooks`: time to diff 2 large but similar workbooks.
* Track:

  * Total time.
  * Peak memory (if feasible in your environment).
* When `metrics-export` is enabled, emit `target/metrics/current_run.json` with `parse_time_ms_per_mb`, `peak_memory_usage_kb`, and `alignment_efficiency_score` for P1/P2 scenarios.
* Store baselines in CI; fail if regressions exceed a threshold.

---

### Milestone 11 – Product‑level tests: CLIs, APIs, and UX contracts

From the competitive analysis, your product should support CLI/Git and later Web/WASM. 

**CLI tests**

Assuming a CLI like:

```text
excel-diff m-diff old.xlsx new.xlsx --format json
```

* **Integration tests**

  * Run the binary against multiple `{a,b}` pairs from the earlier scenarios.
  * Assert:

    * Exit code 0 on success.
    * JSON parses successfully.
    * Schema fields (`queries_changed`, `queries_added`, etc.) match what the library already asserts.

**API contract tests**

If your Rust crate exposes a public API (used by e.g. GUI or WASM):

* Add tests that treat it as a black box:

  * Given two file paths → returns `DiffReport` object.
  * Serialize `DiffReport` and ensure it’s stable (versioned). This matters for web clients and future backward compatibility.

---

### Milestone 12 - Cross-platform determinism

Goal: enforce identical diff output across supported platforms/builds (Windows/Linux native + WASM).

**Tests ([H] now, [G] before multi-platform release)**

* Run the same small canonical fixture suite (one per major scenario) on native Windows, native Linux, and headless WASM in CI.
* Assert the JSON diff output is identical (or matches after a stable canonicalization); fail on drift.
* Capture artifacts on mismatch to debug float/order issues; keep the same wire schema for all targets.

---

You already have a general perf milestone, but two grid-focused ones are worth calling out separately, because the H1 difficulty item explicitly calls out "high-performance 2D grid diff" as one of the hardest problems.

---

### P1 – Large dense sheet, minimal changes

**Core capability**

Row/column alignment and cell diff remain near‑linear on a large dense sheet when only a few cells change.

**Fixture sketch**

* `grid_large_dense_{a,b}.xlsx`

  * A: 50k rows × 50 columns with synthetic but deterministic data.
  * B: identical except for 50 random `CellEdited` changes.

**Checks**

* Measured time and memory within agreed budget (record baseline in CI).
* Diff only lists those ~50 `CellEdited` ops; no spurious structure changes.

---

### P2 – Large sheet with random noise (worst‑case alignment)

**Core capability**

Even when row signatures collide and there is lots of noise, the Hunt–Szymanski alignment doesn’t degenerate to catastrophic behavior.

**Fixture sketch**

* `grid_large_noise_{a,b}.xlsx`

  * A: large sheet with pseudo‑random data.
  * B: another large sheet with different random data of the same shape.

**Checks**

* Runtime stays within acceptable multiple of P1 (record in perf suite).
* Diff primarily reports per‑cell edits; no pathological explosion in DiffOps or runtime.



---

## Phase 6 - DAX/model stubs (post-MVP placeholder)

Goal: anchor the later DAX/data-model diff work so the plan stays aligned with the architecture while staying explicitly post-MVP.

### DX1 - Measure parsing smoke test

**Fixtures (Python) [H, post-MVP]**

* `dax_measures_{a,b}.pbix` or `.xlsx` with a minimal tabular model containing two measures (e.g., `TotalSales` = SUM, `AvgSales` = AVERAGE); in B change one measure definition.
* Keep the fixture tiny; static checked-in files are fine if automating tabular model generation is painful.

**Tests ([H], post-MVP)**

* Parse measures into an AST and assert a simple change (SUM -> AVERAGE or literal tweak) surfaces as a structured `DefinitionChanged` (or equivalent) on that measure.
* Tag the scenario as post-MVP so it does not block Excel/M release trains but keeps DAX/data-model semantics on the radar.

---
```

---

### File: `fixtures\manifest.yaml`

```yaml
scenarios:
  # --- Phase 1.1: Basic File Opening ---
  - id: "smoke_minimal"
    generator: "basic_grid"
    args: { rows: 1, cols: 1 }
    output: "minimal.xlsx"

  # --- Phase 1.2: Is this a ZIP? ---
  - id: "container_random_zip"
    generator: "corrupt_container"
    args: { mode: "random_zip" }
    output: "random_zip.zip"
    
  - id: "container_no_content_types"
    generator: "corrupt_container"
    args: { mode: "no_content_types" }
    output: "no_content_types.xlsx"

  # --- PG1: Workbook -> Sheet -> Grid IR sanity ---
  - id: "pg1_basic_two_sheets"
    generator: "basic_grid"
    args: { rows: 3, cols: 3, two_sheets: true } # Sheet1 3x3, Sheet2 5x2 (logic in generator)
    output: "pg1_basic_two_sheets.xlsx"

  - id: "pg1_sparse"
    generator: "sparse_grid"
    output: "pg1_sparse_used_range.xlsx"

  - id: "pg1_mixed"
    generator: "edge_case"
    output: "pg1_empty_and_mixed_sheets.xlsx"

  # --- PG2: Addressing and index invariants ---
  - id: "pg2_addressing"
    generator: "address_sanity"
    args:
      targets: ["A1", "B2", "C3", "Z1", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1"]
    output: "pg2_addressing_matrix.xlsx"

  # --- PG3: Cell snapshots and comparison semantics ---
  - id: "pg3_types"
    generator: "value_formula"
    output: "pg3_value_and_formula_cells.xlsx"

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

  # --- Milestone 6: Basic M Diffs ---
  - id: "m_change_literal"
    generator: "mashup_inject"
    args:
      base_file: "templates/base_query.xlsx"
      # This query adds a step, changing the definition
      m_code: |
        section Section1;
        shared Query1 = let
            Source = Csv.Document(File.Contents("C:\data.csv"),[Delimiter=",", Columns=2, Encoding=1252, QuoteStyle=QuoteStyle.None]),
            #"Changed Type" = Table.TransformColumnTypes(Source,{{"Column1", type text}, {"Column2", type text}}),
            #"Added Custom" = Table.AddColumn(#"Changed Type", "Custom", each 2)
        in
            #"Added Custom";
    output: "m_change_literal_b.xlsx"

  # --- P1: Large Dense Grid (Performance Baseline) ---
  - id: "p1_large_dense"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "dense" # Deterministic "R1C1" style data
    output: "grid_large_dense.xlsx"

  # --- P2: Large Noise Grid (Worst-case Alignment) ---
  - id: "p2_large_noise"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "noise" # Random float data
      seed: 12345
    output: "grid_large_noise.xlsx"

  # --- D1: Keyed Equality (Database Mode) ---
  # File A: Ordered IDs 1..1000
  - id: "db_equal_ordered_a"
    generator: "db_keyed"
    args: { count: 1000, shuffle: false, seed: 42 }
    output: "db_equal_ordered_a.xlsx"

  # File B: Same data, random order (Tests O(N) alignment)
  - id: "db_equal_ordered_b"
    generator: "db_keyed"
    args: { count: 1000, shuffle: true, seed: 42 }
    output: "db_equal_ordered_b.xlsx"

  # --- D2: Row Added (Database Mode) ---
  - id: "db_row_added_b"
    generator: "db_keyed"
    args: 
      count: 1000 
      seed: 42 
      # Inject a new ID at the end
      extra_rows: [{id: 1001, name: "New Row", amount: 999}]
    output: "db_row_added_b.xlsx"
```

---

### File: `fixtures\pyproject.toml`

```yaml
[project]
name = "excel-fixtures"
version = "0.1.0"
description = "Deterministic artifact generator for Excel Diff testing"
readme = "README.md"
requires-python = ">=3.9"
dependencies = [
    "openpyxl>=3.1.0",
    "lxml>=4.9.0",
    "jinja2>=3.1.0",
    "pyyaml>=6.0",
]

[project.scripts]
generate-fixtures = "src.generate:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]

```

---

### File: `fixtures\README.md`

```markdown
# Excel Diff Fixtures Generator

This repository contains the deterministic artifact generator for the **Excel Diff** project. It produces a wide variety of `.xlsx` (and related) files used to validate and stress-test the Rust diff engine.

The goal is to have a reproducible, version-controlled source of truth for test cases, ranging from simple grids to corrupted containers and large performance benchmarks.

## Features

- **Deterministic Generation**: All fixtures are generated from code and seeded random number generators, ensuring identical outputs across runs.
- **Manifest Driven**: Scenarios are defined in `manifest.yaml`, decoupling configuration from code.
- **Diverse Test Cases**:
  - **Basic Grids**: Dense, sparse, and mixed content sheets.
  - **Corrupt Files**: Invalid ZIP headers, missing content types, byte-level corruption.
  - **Performance**: Large datasets (50k+ rows) for benchmarking.
  - **Database Mode**: Keyed tables to test `O(N)` alignment and diffing.
  - **Mashups**: Injections of M-code (Power Query) and custom parts into existing templates.

## Setup

This project is managed with standard Python tooling. You can use `uv` (recommended) or `pip`.

### Using `uv` (Recommended)

```bash
# Sync dependencies
uv sync

# Run the generator
uv run generate-fixtures
```

### Using `pip`

```bash
# Install dependencies
pip install -r requirements.txt

# Run the generator script directly
python src/generate.py
```

## Usage

The generator reads scenarios from `manifest.yaml` and produces files in the `fixtures/generated/` directory.

To generate all fixtures:

```bash
python src/generate.py
```

### Command Line Arguments

- `--manifest`: Path to the manifest file (default: `manifest.yaml`)
- `--output-dir`: Directory to output generated files (default: `fixtures/generated`)
- `--force`: Force regeneration even if files exist (default: false - *Note: implementation currently always overwrites*)

## Configuration (Manifest)

The `manifest.yaml` file defines the test scenarios. Each entry in the `scenarios` list requires:

- `id`: Unique identifier for the test case.
- `generator`: The registered name of the generator class to use.
- `output`: The filename for the generated artifact.
- `args`: (Optional) Dictionary of arguments passed to the generator.

**Example:**

```yaml
- id: "pg1_basic"
  generator: "basic_grid"
  args: 
    rows: 10
    cols: 5
  output: "basic_sheet.xlsx"
```

## Available Generators

| Generator Name | Description | Key Arguments |
|stub|---|---|
| `basic_grid` | Simple dense grids. | `rows`, `cols`, `two_sheets` |
| `sparse_grid` | Sheets with scattered data to test bounds. | - |
| `edge_case` | Empty sheets, mixed types, whitespace. | - |
| `address_sanity` | Specific cell targets (e.g., "A1", "ZZ10") to test addressing logic. | `targets` (list) |
| `value_formula` | Mix of static values and formulas. | - |
| `corrupt_container` | Invalid ZIP structures or missing XML parts. | `mode` ("random_zip", "no_content_types") |
| `mashup_corrupt` | Modifies bytes of a base template. | `base_file`, `mode` ("byte_flip") |
| `mashup_inject` | Injects content (like M-code) into templates. | `base_file`, `m_code` |
| `perf_large` | Large datasets for stress testing. | `rows`, `cols`, `mode` ("dense", "noise") |
| `db_keyed` | Tabular data with IDs to test row alignment. | `count`, `shuffle`, `extra_rows` |

## Project Structure

```
├── fixtures/
│   ├── generated/      # Output directory (git-ignored)
│   └── templates/      # Base Excel files used by mashup generators
├── src/
│   ├── generate.py     # Entry point
│   └── generators/     # Generator implementations
│       ├── base.py     # Base classes
│       ├── grid.py     # Standard grid generators
│       ├── corrupt.py  # ZIP/Container corruption
│       ├── mashup.py   # Template modification
│       ├── perf.py     # Performance generators
│       └── database.py # Keyed table generators
├── manifest.yaml       # Test case definitions
└── pyproject.toml      # Project metadata
```

## Adding a New Generator

1. Create a new class in `src/generators/` inheriting from `BaseGenerator`.
2. Implement the `generate(self, output_dir: Path, filename: str)` method.
3. Register the generator in `src/generate.py` in the `GENERATORS` dictionary.
4. Add a scenario using your new generator to `manifest.yaml`.
```

---

### File: `fixtures\requirements.txt`

```
openpyxl>=3.1.0
lxml>=4.9.0
jinja2>=3.1.0
pyyaml>=6.0

```

---

### File: `fixtures\src\generate.py`

```python
import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
from generators.grid import (
    BasicGridGenerator, 
    SparseGridGenerator, 
    EdgeCaseGenerator, 
    AddressSanityGenerator,
    ValueFormulaGenerator
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import MashupCorruptGenerator, MashupInjectGenerator
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_inject": MashupInjectGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
}

def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}")
        sys.exit(1)
    
    with open(manifest_path, 'r') as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}")
            sys.exit(1)

def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)

def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent
    
    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument("--manifest", type=Path, default=default_manifest, help="Path to the manifest YAML file.")
    parser.add_argument("--output-dir", type=Path, default=default_output, help="Directory to output generated files.")
    parser.add_argument("--force", action="store_true", help="Force regeneration of existing files.")
    
    args = parser.parse_args()
    
    manifest = load_manifest(args.manifest)
    ensure_output_dir(args.output_dir)
    
    scenarios = manifest.get('scenarios', [])
    print(f"Found {len(scenarios)} scenarios in manifest.")
    
    for scenario in scenarios:
        scenario_id = scenario.get('id')
        generator_name = scenario.get('generator')
        generator_args = scenario.get('args', {})
        outputs = scenario.get('output')
        
        if not scenario_id or not generator_name or not outputs:
            print(f"Skipping invalid scenario: {scenario}")
            continue
            
        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")
        
        if generator_name not in GENERATORS:
            print(f"  Warning: Generator '{generator_name}' not implemented yet. Skipping.")
            continue
        
        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(args.output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            print(f"  Error generating scenario {scenario_id}: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    main()
```

---

### File: `fixtures\src\__init__.py`

```python

```

---

### File: `fixtures\src\generators\base.py`

```python
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List

class BaseGenerator(ABC):
    """
    Abstract base class for all fixture generators.
    """
    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """
        Generates the fixture file(s).
        
        :param output_dir: The directory to save the file(s) in.
        :param output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass

    def _post_process_injection(self, file_path: Path, injection_callback):
        """
        Implements the "Pass 2" architecture:
        1. Opens the generated xlsx (zip).
        2. Injects/Modifies streams (DataMashup, etc).
        3. Saves back.
        
        This is a crucial architectural decision to handle openpyxl stripping customXml.
        """
        pass

```

---

### File: `fixtures\src\generators\corrupt.py`

```python
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class ContainerCorruptGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        mode = self.args.get('mode', 'no_content_types')
        
        for name in output_names:
            # Create a dummy zip
            out_path = output_dir / name
            
            if mode == 'random_zip':
                # Just a zip with a text file
                with zipfile.ZipFile(out_path, 'w') as z:
                    z.writestr("hello.txt", "This is not excel")
                    
            elif mode == 'no_content_types':
                # Create a valid excel in memory, then strip [Content_Types].xml
                buffer = io.BytesIO()
                import openpyxl
                wb = openpyxl.Workbook()
                # Add some content just so it's not totally empty
                wb.active['A1'] = 1
                wb.save(buffer)
                buffer.seek(0)
                
                with zipfile.ZipFile(buffer, 'r') as zin:
                    with zipfile.ZipFile(out_path, 'w') as zout:
                        for item in zin.infolist():
                            if item.filename != "[Content_Types].xml":
                                zout.writestr(item, zin.read(item.filename))

```

---

### File: `fixtures\src\generators\database.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])

        # Use deterministic seed
        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            # 1. Define Base Data (List of Dicts)
            # Schema: [ID, Name, Amount, Category]
            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            # 2. Apply Mutations (Additions)
            # This allows us to inject specific "diffs" like D2 (Row Added)
            for row in extra_rows:
                data_rows.append(row)

            # 3. Apply Shuffle (The core D1 test)
            if shuffle:
                rng.shuffle(data_rows)

            # 4. Write to Sheet
            # Header
            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                # Ensure strictly ordered list matching headers
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\grid.py`

```python
import openpyxl
from openpyxl.utils import get_column_letter
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class BasicGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        rows = self.args.get('rows', 5)
        cols = self.args.get('cols', 5)
        two_sheets = self.args.get('two_sheets', False)
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sheet1"
            
            # Fill grid
            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")
            
            # Check if we need a second sheet
            if two_sheets:
                ws2 = wb.create_sheet(title="Sheet2")
                # Different dimensions for Sheet2 (PG1 requirement: 5x2)
                # If args are customized we might need more logic, but for PG1 this is sufficient or we use defaults
                s2_rows = 5
                s2_cols = 2
                for r in range(1, s2_rows + 1):
                    for c in range(1, s2_cols + 1):
                         ws2.cell(row=r, column=c, value=f"S2_R{r}C{c}")

            wb.save(output_dir / name)

class SparseGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sparse"
            
            # Specifics for pg1_sparse_used_range
            ws['A1'] = "A1"
            ws['B2'] = "B2"
            ws['G10'] = "G10" # Forces extent
            # Row 5 and Col D are empty implicitly by not writing to them
            
            wb.save(output_dir / name)

class EdgeCaseGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        
        for name in output_names:
            wb = openpyxl.Workbook()
            # Remove default sheet
            default_ws = wb.active
            wb.remove(default_ws)
            
            # Empty Sheet
            wb.create_sheet("Empty")
            
            # Values Only
            ws_val = wb.create_sheet("ValuesOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    ws_val.cell(row=r, column=c, value=r*c)
            
            # Formulas Only
            ws_form = wb.create_sheet("FormulasOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    # Reference ValuesOnly sheet
                    col_letter = get_column_letter(c)
                    ws_form.cell(row=r, column=c, value=f"=ValuesOnly!{col_letter}{r}")
            
            wb.save(output_dir / name)

class AddressSanityGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        targets = self.args.get('targets', ["A1", "B2", "Z10"])
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Addresses"
            
            for addr in targets:
                ws[addr] = addr
                
            wb.save(output_dir / name)

class ValueFormulaGenerator(BaseGenerator):
    """PG3: Types, formulas, values"""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Types"
            
            ws['A1'] = 42
            ws['A2'] = "hello"
            ws['A3'] = True
            # A4 empty
            
            ws['B1'] = "=A1+1"
            ws['B2'] = '="hello" & " world"'
            ws['B3'] = "=A1>0"
            
            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import struct
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        return tree.find('//dm:DataMashup', namespaces=NS)

    def _process_excel_container(self, base_path, output_path, callback):
        """
        Generic wrapper to open xlsx, find customXml, apply a callback to the 
        DataMashup bytes, and save the result.
        """
        # Copy base file structure to output
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for item in zin.infolist():
                    buffer = zin.read(item.filename)
                    
                    # We only care about the item containing DataMashup
                    # Usually customXml/item1.xml, but we check content to be safe
                    if item.filename.startswith("customXml/item") and b"DataMashup" in buffer:
                        # Parse XML
                        root = etree.fromstring(buffer)
                        dm_node = self._get_mashup_element(root)
                        
                        if dm_node is not None:
                            # 1. Decode
                            # The text content might have whitespace/newlines, strip them
                            b64_text = dm_node.text.strip() if dm_node.text else ""
                            if b64_text:
                                raw_bytes = base64.b64decode(b64_text)
                                
                                # 2. Apply modification (The Callback)
                                new_bytes = callback(raw_bytes)
                                
                                # 3. Encode back
                                dm_node.text = base64.b64encode(new_bytes).decode('utf-8')
                                buffer = etree.tostring(root, encoding='utf-8', xml_declaration=True)
                    
                    zout.writestr(item, buffer)

class MashupCorruptGenerator(MashupBaseGenerator):
    """Fuzzes the DataMashup bytes to test error handling."""
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        if not base_file_arg:
            raise ValueError("MashupCorruptGenerator requires 'base_file' argument")

        # Resolve base file relative to current working directory or fixtures/templates
        base = Path(base_file_arg)
        if not base.exists():
             # Try looking in fixtures/templates if a relative path was given
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        mode = self.args.get('mode', 'byte_flip')

        def corruptor(data):
            mutable = bytearray(data)
            if len(mutable) == 0:
                return bytes(mutable)

            if mode == 'byte_flip':
                # Flip a byte in the middle
                idx = len(mutable) // 2
                mutable[idx] = mutable[idx] ^ 0xFF
            elif mode == 'truncate':
                return mutable[:len(mutable)//2]
            return bytes(mutable)

        for name in output_names:
            # Convert Path objects to strings for resolve() to work correctly if there's a mix
            # Actually output_dir is a Path. name is str.
            # .resolve() resolves symlinks and relative paths to absolute
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, corruptor)


class MashupInjectGenerator(MashupBaseGenerator):
    """
    Peels the onion:
    1. Parses MS-QDEFF binary header.
    2. Unzips PackageParts.
    3. Injects new M-Code into Section1.m.
    4. Re-zips and fixes header lengths.
    """
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        new_m_code = self.args.get('m_code')

        if not base_file_arg:
             raise ValueError("MashupInjectGenerator requires 'base_file' argument")
        if new_m_code is None:
             raise ValueError("MashupInjectGenerator requires 'm_code' argument")

        base = Path(base_file_arg)
        if not base.exists():
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        def injector(raw_bytes):
            return self._inject_m_code(raw_bytes, new_m_code)

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, injector)

    def _inject_m_code(self, raw_bytes, m_code):
        # --- 1. Parse MS-QDEFF Header ---
        # Format: Version(4) + LenPP(4) + PackageParts(...) + LenPerm(4) + ...
        # We assume Version is 0 (first 4 bytes)
        
        if len(raw_bytes) < 8:
            return raw_bytes # Too short to handle

        offset = 4
        # Read PackageParts Length
        pp_len = struct.unpack('<I', raw_bytes[offset:offset+4])[0]
        offset += 4
        
        # Extract existing components
        pp_bytes = raw_bytes[offset : offset + pp_len]
        
        # Keep the rest of the stream (Permissions, Metadata, Bindings) intact
        # We just append it later
        remainder_bytes = raw_bytes[offset + pp_len :]

        # --- 2. Modify PackageParts (Inner ZIP) ---
        new_pp_bytes = self._replace_in_zip(pp_bytes, 'Formulas/Section1.m', m_code)

        # --- 3. Rebuild Stream ---
        # New Length for PackageParts
        new_pp_len = len(new_pp_bytes)
        
        # Reconstruct: Version(0) + NewLen + NewPP + Remainder
        header = raw_bytes[:4] # Version
        len_pack = struct.pack('<I', new_pp_len)
        
        return header + len_pack + new_pp_bytes + remainder_bytes

    def _replace_in_zip(self, zip_bytes, filename, new_content):
        """Opens a ZIP byte stream, replaces a file, returns new ZIP byte stream."""
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()
        
        try:
            with zipfile.ZipFile(in_buffer, 'r') as zin:
                with zipfile.ZipFile(out_buffer, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
                    for item in zin.infolist():
                        if item.filename == filename:
                            # Write the new M code
                            zout.writestr(filename, new_content.encode('utf-8'))
                        else:
                            # Copy others
                            zout.writestr(item, zin.read(item.filename))
        except zipfile.BadZipFile:
            # Fallback if inner stream isn't a valid zip (shouldn't happen on valid QDEFF)
            return zip_bytes
            
        return out_buffer.getvalue()

```

---

### File: `fixtures\src\generators\perf.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)

        # Use deterministic seed if provided, otherwise system time
        rng = random.Random(seed)

        for name in output_names:
            # WriteOnly mode is critical for 50k+ rows in Python
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            # 1. Header
            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            # 2. Data Stream
            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    # Deterministic pattern: "R{r}C{c}"
                    # Fast to generate, high compression ratio
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    # Random floats: Harder to align, harder to compress
                    row_data = [rng.random() for _ in range(cols)]
                
                ws.append(row_data)

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\__init__.py`

```python
# Generators package

```

---

