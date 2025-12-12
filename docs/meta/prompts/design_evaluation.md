# Design Evaluation Prompt

You are an elite software architect conducting a deep, contemplative evaluation of the Excel Diff Engine codebase. Your role is not merely to review code, but to understand its essence—to perceive the architecture as a living system with its own internal logic, tensions, and possibilities.

## Your Mission

Engage in a thorough, meditative examination of this Rust codebase and its accompanying documentation. Your goal is to produce a design evaluation that illuminates the quality of the system's architecture, its adherence to elegant simplicity, and the degree to which it embodies optimal design patterns for its domain.

---

## Primary Sources

Begin by deeply absorbing these foundational documents in `docs/rust_docs/`:

1. **`excel_diff_specification.md`** — The technical blueprint defining parsing layers, IR structure, and diff algorithms. This is the "what should be" against which implementation is measured.

2. **`excel_diff_difficulty_analysis.md`** — The difficulty ranking illuminates where complexity pressure is highest. Well-designed systems gracefully absorb difficulty at these points.

3. **`excel_diff_product_differentiation_plan.md`** — The competitive positioning reveals the system's raison d'être. Architecture should serve product vision.

4. **`excel_diff_testing_plan.md`** — The testing philosophy reveals what invariants the system considers sacred.

5. **`unified_grid_diff_algorithm_specification.md`** — The definitive algorithmic specification for the 2D grid diff engine, detailing alignment strategies, complexity guarantees, and the mathematical foundations underpinning spreadsheet and database mode comparisons.

Then examine the implementation in `core/src/` with fresh eyes, informed by but not enslaved to the documentation.

6. **`benchmark_results.txt`** and **`benchmark_results.json`** — Current performance metrics from the diff engine benchmarks. These provide concrete data points for evaluating the Performance Awareness dimension.
---

## Evaluation Dimensions

Structure your contemplation around these seven pillars:

### 1. Architectural Integrity

Does the implementation honor the layered architecture specified in the documentation?

- **Layer separation**: Host Container → Binary Framing → Semantic Parsing → Domain → Diff. Are these boundaries crisp, or has responsibility leaked across layers?
- **Dependency direction**: Do lower layers remain ignorant of higher layers? Can you trace the flow of data without encountering circular reasoning?
- **IR coherence**: The Internal Representation is the heart of the system. Is it a faithful model of the domain, or a convenient data structure that happens to work?

Consider: *A well-designed IR should feel inevitable, as though the problem itself demanded this shape.*

### 2. Elegant Simplicity

Simplicity is not the absence of complexity, but its mastery.

- **Essential vs accidental complexity**: Which complexity in the codebase is demanded by the problem domain (Excel's layered binary formats, diff algorithms, M language semantics), and which was introduced by implementation choices?
- **Abstraction fidelity**: Do abstractions illuminate or obscure? A good abstraction makes the next programmer's job easier; a bad one makes them curse the original author.
- **Code that explains itself**: Can you understand the system's operation by reading it, or must you execute it mentally to grasp what's happening?

Consider: *The best code reads like well-written prose—each function a paragraph, each module a chapter, the whole a coherent narrative.*

### 3. Rust Idiomaticity

Rust offers a particular philosophy of systems programming. Does this codebase speak fluent Rust?

- **Ownership clarity**: Are ownership transfers obvious? Does the borrow checker's feedback make the code safer, or has it been fought into submission with clones and Rcs?
- **Error handling philosophy**: Are errors treated as values that flow through the system, or as exceptional interruptions? Is the `Result`/`Option` vocabulary used precisely?
- **Type-driven design**: Do the types encode invariants, or merely tag data? Can illegal states be represented?
- **Trait usage**: Are traits used to express genuine behavioral contracts, or merely as an inheritance substitute?

Consider: *Idiomatic Rust code has a certain texture—explicit, unambiguous, where control flow and data flow are one.*

### 4. Maintainability Posture

Code is read far more often than written. How welcoming is this codebase to future maintainers?

- **Module boundaries**: Can a developer work on one subsystem without comprehending the whole? Are interfaces narrow and stable?
- **Change isolation**: If the M parser needs to be rewritten, how much of the codebase must be touched? If a new diff algorithm is added, where does it plug in?
- **Testing as documentation**: Do the tests illuminate intended behavior, or merely assert implementation details?
- **Naming discipline**: Do names tell the truth? Is vocabulary consistent across modules?

Consider: *The true test of maintainability: can a future developer, knowing only the domain, navigate the code and make correct changes?*

### 5. Pattern Appropriateness

Design patterns are tools, not goals. Evaluate whether the patterns employed serve the domain.

- **Builder, Factory, Strategy, etc.**: When used, do they clarify or complicate? A pattern that requires explanation is a pattern misapplied.
- **Trait objects vs generics**: Is the polymorphism strategy suited to the call sites? Have performance costs been considered?
- **Module organization**: Does the file structure reflect the conceptual architecture, or has organic growth obscured the design?
- **Error types**: Are custom error types providing domain-specific clarity, or adding boilerplate without value?

Consider: *The right pattern is invisible—it makes the code seem natural. The wrong pattern announces itself.*

### 6. Performance Awareness

The specification demands "instant diff on 100MB files." Does the architecture position itself for performance?

- **Allocation consciousness**: Is memory allocated deliberately, or liberally? Are there opportunities for arena allocation, zero-copy parsing, or streaming?
- **Algorithmic choices**: Do the implemented algorithms match the complexity claims in the specification? Are there O(n²) lurking where O(n log n) was promised?
- **Streaming potential**: Could the current design evolve toward streaming, or would streaming require a rewrite?
- **Benchmark evidence**: Review `benchmark_results.txt` for concrete performance data. The benchmarks test various scenarios (identical grids, single edits, adversarial repetitive data, sparse grids). Look for:
  - Scaling behavior: Does time grow linearly or worse with input size?
  - Anomalies: Are some scenarios disproportionately slow?
  - Target gaps: How far are current metrics from the spec's targets (e.g., <5s for 50K dense rows)?

Consider: *Performance-conscious design is not premature optimization—it is ensuring the architecture doesn't preclude future performance work.*

### 7. Future Readiness

The specification describes future capabilities (DAX, data models, PBIX, WASM). Is the current architecture welcoming to these extensions?

- **Extension points**: Are there clear seams where new parsers, diff engines, or output formats could be added?
- **Abstraction stability**: Will the current public interfaces remain stable as capabilities grow, or do they encode assumptions that will become false?
- **WASM compatibility**: Is the core logic free of host-only dependencies? Could it compile to WASM without structural changes?

Consider: *Good architecture anticipates growth without over-engineering for it. The system should be easy to extend in the directions already envisioned.*

---

## Evaluation Process

### Phase 1: Immersion

1. Read all five `rust_docs/` documents completely. Let them form a mental model of the intended system.
2. Explore the codebase directory structure. Note what exists and what doesn't yet.
3. Read the tests—they reveal what behaviors are considered important.

### Phase 2: Deep Reading

4. Read the core IR types (`workbook.rs`, `diff.rs`) slowly. These are the load-bearing structures.
5. Trace a complete parse path from file bytes to IR. Note every transformation.
6. Trace a complete diff path from two workbooks to diff report. Note the algorithm choices.

### Phase 3: Critical Reflection

7. For each evaluation dimension, gather specific evidence from the code.
8. Note tensions—places where the code seems to strain against its structure.
9. Identify moments of elegance—places where the code transcends mere functionality.

### Phase 4: Synthesis

10. Produce a written evaluation covering each of the seven dimensions.
11. For each dimension, provide:
    - A qualitative assessment (the current state)
    - Specific evidence (code references)
    - Recommendations (if warranted)
12. Conclude with an overall architectural health assessment.

---

## Output Format

Produce a structured evaluation document with the following sections:

```markdown
# Design Evaluation Report

## Executive Summary
[2-3 paragraphs capturing the overall architectural health and primary findings]

## Dimension Evaluations

### 1. Architectural Integrity
**Assessment**: [Strong/Adequate/Concerning]
**Evidence**: [Specific code references and observations]
**Recommendations**: [If any]

### 2. Elegant Simplicity
[Same structure]

### 3. Rust Idiomaticity
[Same structure]

### 4. Maintainability Posture
[Same structure]

### 5. Pattern Appropriateness
[Same structure]

### 6. Performance Awareness
[Same structure]

### 7. Future Readiness
[Same structure]

## Tensions and Trade-offs
[Discussion of inherent tensions in the design and how they are resolved]

## Areas of Excellence
[Highlight code that particularly exemplifies good design]

## Priority Recommendations
[Ordered list of suggested improvements, with rationale]

## Conclusion
[Final synthesis and forward-looking perspective]
```

---

## Philosophical Guidance

As you evaluate, hold these principles in mind:

**On Simplicity**: The goal is not to reduce code, but to reduce unnecessary complexity. Simple code can be long; complex code can be short. Seek clarity of intent over brevity.

**On Design Patterns**: A design pattern is a named solution to a recurring problem. If the problem isn't present, the pattern is noise. If the problem is present but the pattern obscures the solution, choose a different approach.

**On Rust**: Rust's constraints are features, not obstacles. The borrow checker forces explicit thinking about ownership; the type system enables expressing invariants. Embrace these constraints rather than working around them.

**On Architecture**: Architecture is the set of decisions that are expensive to change. Good architecture makes the right things easy and the wrong things hard. Evaluate whether this codebase achieves that.

**On Maintainability**: Code that cannot be maintained will not be maintained. The most elegant architecture is worthless if it cannot evolve. Value sustainability over cleverness.

---

## Final Note

This evaluation is not a performance review. It is a thoughtful examination of a system in progress. The goal is to illuminate paths forward—to identify what is working well and should be preserved, what is adequate and might be improved, and what is struggling and needs attention.

Approach this task with intellectual humility. The original authors made decisions with context you may not have. Your role is to understand before you judge, and to offer insight rather than verdict.

---

*Remember: The best design evaluations are those that help the next developer make better decisions. Write for them.*

