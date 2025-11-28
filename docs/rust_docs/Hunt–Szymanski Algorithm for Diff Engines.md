# **Algorithmic Foundations for High-Precision Spreadsheet Difference Engines: An Exhaustive Analysis of the Hunt–Szymanski Algorithm**

## **1\. Historical Background and Primary Sources**

### **1.1 The Genesis of File Comparison**

The origins of automated file comparison are deeply rooted in the evolution of software engineering practices, specifically the need to track changes in source code. In the mid-1970s, as the Unix operating system was being developed at Bell Laboratories, the necessity for a robust utility to identify differences between file versions became apparent. Early attempts relied on ad-hoc heuristics, such as the "proof" program by Mike Lesk and comparison utilities on the GECOS system by Steve Johnson. These early tools often used simple line-alignment strategies that could fail to find the minimal set of changes, leading to confusing or "noisy" difference reports.  
The pivotal moment arrived with the collaboration between Douglas McIlroy and James W. Hunt. McIlroy, seeking a more rigorous mathematical foundation for the diff utility, challenged the reliance on heuristics. This led to the development of the first non-heuristic algorithms capable of guaranteeing the Longest Common Subsequence (LCS), a mathematical proxy for the "minimal edit script" required to transform one sequence into another.

### **1.2 The Hunt–Szymanski Contribution**

The algorithm now known as Hunt–Szymanski was formally introduced in the 1977 paper "A Fast Algorithm for Computing Longest Common Subsequences," published in the *Communications of the ACM*. The work was a refinement of an earlier technical report by Hunt and McIlroy (1976) and built upon theoretical generalizations proposed by Harold S. Stone regarding the work of Thomas G. Szymanski.  
The significance of the Hunt–Szymanski algorithm cannot be overstated. Prior to its introduction, the standard approach to solving the LCS problem involved dynamic programming methods, such as the Wagner-Fischer algorithm, which required time and space proportional to the product of the sequence lengths (O(MN)). For large files, this quadratic complexity was prohibitively expensive in terms of both CPU cycles and memory. Hunt and Szymanski introduced a paradigm shift by demonstrating that the problem could be solved efficiently by considering only the "matching pairs" between sequences. Their algorithm offered a complexity of O((R \+ N) \\log N), where R is the total number of matching pairs. This "sparse" approach meant that for typical source code files—where lines are often unique or repeat infrequently—the algorithm performed significantly faster than its quadratic predecessors.

### **1.3 Evolution and Legacy**

While the original Unix diff utilized the Hunt–Szymanski approach (specifically the Hunt-McIlroy variant), the landscape of difference algorithms has continued to evolve. In the 1980s, Eugene Myers introduced the O(ND) difference algorithm, which optimized for the size of the edit script (D) rather than the number of matches. Myers' algorithm became the default for GNU diff and later Git, primarily because it tends to produce more human-readable diffs in dense edit scenarios and avoids the worst-case performance pathologies of Hunt–Szymanski on repetitive data.  
However, Hunt–Szymanski remains a cornerstone of algorithmic theory in this domain. It is still widely referenced in bioinformatics for sequence alignment tasks where alphabet sizes are large and matches are sparse. Furthermore, its underlying mechanics—specifically the reduction of LCS to the Longest Increasing Subsequence (LIS) problem via match lists—form the basis for many modern optimizations and hybrid approaches, including the "Patience Diff" algorithm which prioritizes unique lines to generate semantically meaningful diffs.

## **2\. Formal Description and Complexity**

### **2.1 The Longest Common Subsequence (LCS) Problem**

To understand the mechanics of Hunt–Szymanski, we must first rigorously define the problem it solves. Let A and B be two sequences of lengths m and n respectively, drawn from a finite alphabet \\Sigma.  
A sequence C \= c\_1, c\_2, \\dots, c\_k is defined as a *subsequence* of A if there exists a strictly increasing sequence of indices 1 \\le i\_1 \< i\_2 \< \\dots \< i\_k \\le m such that A\[i\_j\] \= c\_j for all 1 \\le j \\le k.  
C is a *common subsequence* of A and B if it is a subsequence of both A and B. The LCS problem is to find a common subsequence of maximal length. Note that the LCS is not necessarily unique; there may be multiple common subsequences of the same maximum length.  
The relationship between the LCS and the "diff" (or shortest edit script) is inverse: finding the longest set of matching items is mathematically equivalent to finding the smallest set of insertions and deletions to transform A into B. If L is the length of the LCS, the size of the edit script D is given by:

### **2.2 The "K-Candidate" Paradigm**

The Hunt–Szymanski algorithm diverges from standard dynamic programming by focusing on **k-candidates**. A k-candidate is a pair of indices (i, j) such that A\[i\] \= B\[j\] and there exists a common subsequence of length k ending at these positions.  
The algorithm relies on the observation that we do not need to compute the LCS length for *every* pair of prefixes. We only need to track the "best" ways to form a common subsequence of length k. Specifically, for a given length k, we want to know the smallest possible index in sequence B that can end a common subsequence of length k. This is based on the greedy principle: ending a match as early as possible in sequence B leaves more "room" for subsequent matches.

### **2.3 The Algorithm Structure**

The algorithm proceeds in three distinct phases:

#### **Phase 1: Match List Construction**

We first construct a data structure that maps every element in sequence A to its occurrences in sequence B. Let MATCHLIST\[i\] be the set of indices \\{j \\mid B\[j\] \= A\[i\]\\}. Crucially, for the standard Hunt–Szymanski algorithm, these indices in MATCHLIST\[i\] must be sorted in **descending order**.

* **Reasoning:** The descending order is required to ensure that when we process the matches for a single element A\[i\], we do not use the same element A\[i\] multiple times to extend the same subsequence. By processing matches from right to left (large j to small j), we ensure that any update to the threshold array for length k does not influence the calculation for length k+1 within the same row iteration.

#### **Phase 2: Threshold Calculation**

We maintain an array THRESH (often denoted as T), where THRESH\[k\] stores the smallest index j in B such that there exists a common subsequence of length k ending at B\[j\].

* Initialize THRESH \= 0 and THRESH\[k\] \= \\infty for k \> 0\.  
* Iterate through each position i in A (from 1 to m):  
  * Retrieve the list of matching positions P \= \\text{MATCHLIST}\[i\].  
  * For each j \\in P (processed in descending order):  
    * Find the largest k such that THRESH\[k\] \< j. This step is typically performed using a **binary search** on the THRESH array, which remains sorted throughout the execution.  
    * We have found a common subsequence of length k ending before j. Therefore, we can extend this subsequence by appending the match (i, j) to create a common subsequence of length k+1.  
    * Update THRESH\[k+1\]: If j \< \\text{THRESH}\[k+1\], set THRESH\[k+1\] \= j. This records that we have found a "better" (earlier) ending position for a subsequence of length k+1.

#### **Phase 3: Solution Reconstruction**

To recover the actual diff (the sequence of changes) rather than just the length, we must store predecessor pointers. Whenever THRESH\[k+1\] is updated to j, we create a node representing the match (i, j) and link it to the node corresponding to the match that determined THRESH\[k\]. This forms a set of linked chains. At the end of the algorithm, the last valid entry in the THRESH array points to the end of the LCS. We traverse these pointers backwards to reconstruct the sequence.

### **2.4 Complexity Analysis**

The complexity of Hunt–Szymanski is output-sensitive, depending heavily on the sparsity of matches.

* **Total Matches (r):** Let r be the total number of pairs (i, j) such that A\[i\] \= B\[j\].  
* **Time Complexity:**  
  * Preprocessing (Match List): Requires iterating over both sequences and potentially sorting. This is O(n \\log n) or O(n) depending on the implementation (hashing vs. sorting).  
  * Processing Matches: The core loop iterates over every matching pair in r. For each match, a binary search is performed on the THRESH array, which has a maximum size of n. This yields O(r \\log n).  
  * Total Time: **O((r \+ n) \\log n)**.  
* **Space Complexity:**  
  * The MATCHLIST must store r indices.  
  * The THRESH array takes O(n).  
  * Total Space: **O(r \+ n)**.

**The Worst-Case Scenario:** The variable r is the critical factor.

* **Sparse Case:** If A and B share very few common symbols (e.g., random strings over a large alphabet), r is small (r \\ll n^2). The algorithm performs near O(n \\log n).  
* **Dense/Repetitive Case:** If A and B consist of the same repeated symbol (e.g., "aaaaa" vs "aaaaa"), every position in A matches every position in B. Here, r \= m \\times n \\approx n^2.  
  * The time complexity degrades to **O(n^2 \\log n)**.  
  * This is theoretically *worse* than the naive dynamic programming approach of O(n^2).  
  * **Implication for Spreadsheets:** This worst-case behavior is a significant risk for spreadsheet data, which often contains columns of identical values (e.g., "0", "NULL", "true") or empty rows.

## **3\. Plain-English Explanation and Intuition**

### **3.1 The "Stepping Stone" Analogy**

Imagine two sequences, A and B, as the banks of a river. Sequence A is the left bank, and Sequence B is the right bank. You want to cross the river by stepping on stones. A "stone" exists only where a character on the left bank matches a character on the right bank.

* **Standard LCS (Dynamic Programming):** This is like mapping out the entire riverbed, grid square by grid square, to check for stones and calculating the best path at every single coordinate. Even if the river is mostly empty water, you still do the work for every inch.  
* **Hunt–Szymanski:** This approach is like a surveyor who first scans the river and makes a list of *only* the coordinates where stones actually exist. This is the **Match List**. If there are only a few stones (sparse matches), the surveyor has very little work to do.

### **3.2 The "Leaderboard" (Threshold Array)**

Once the surveyor has the list of stones, the goal is to hop across as many stones as possible while always moving forward (increasing index in A and B). The algorithm processes the stones row by row (stepping down the left bank).  
As you visit stones, you maintain a **Leaderboard** (the Threshold Array). This leaderboard tracks the "best known finish lines" for paths of different lengths.

* Entry \#1 on the leaderboard says: "The earliest we can finish a path of 1 stone is at index 5 on the right bank."  
* Entry \#2 says: "The earliest we can finish a path of 2 stones is at index 12 on the right bank."

When you find a new stone at position (Row A: 10, Col B: 8), you check the leaderboard. You ask: "Can I use this stone to make a path of length 1?" Yes, and it ends at 8\. Is 8 better (earlier) than the current best for length 1 (which was 5)? No. Then you ask: "Can I use this stone to extend a path?" If the best path of length 1 ended at 5, and your new stone is at 8, you can extend it\! You now have a path of length 2 ending at 8\. If the previous best path of length 2 ended at 12, you update the leaderboard: "New record\! Best path of length 2 now ends at 8."  
By strictly maintaining this leaderboard, you ensure that you are always keeping the "tightest" possible chains of matches, which maximizes your ability to add more matches later.

### **3.3 Intuition for the Descending Sort**

Why do we process matches in reverse order for a single row? Imagine Row 10 of Sequence A has the letter 'X'. Sequence B has 'X' at positions 20, 30, and 40\. If we processed them in order (20, then 30, then 40):

1. We use 20 to extend a path of length k to k+1. We update the leaderboard for k+1.  
2. We look at 30\. We see the leaderboard entry for k+1 (which we just updated\!) and think we can extend *that* one to k+2.  
3. We end up counting the single 'X' in Row 10 three times for the same path. This is illegal; one character in A can only match one character in B.

By processing them as 40, then 30, then 20:

1. We use 40 to update the leaderboard.  
2. When we look at 30, we compare it against the *old* leaderboard values (because 40 is larger than 30, it won't affect the check for 30). This clever sorting trick prevents "double-counting" a node within the same step.

## **4\. Variants, Implementation Patterns, and Practical Considerations**

### **4.1 The Kuo-Cross Variant (1989)**

While Hunt–Szymanski is efficient, the requirement to process matches in descending order and perform binary searches for every match can be suboptimal. The **Kuo-Cross** algorithm is a significant optimization that addresses the stability of the Threshold array.

* **Ascending Order:** Kuo and Cross proved that by processing matches in **ascending order** and utilizing a specific update strategy, one can avoid the "redundant" binary searches that occur in the original algorithm.  
* **Eliminating Redundant Updates:** In the original HS, the Threshold array might be updated multiple times for the same length k as we process a row. Kuo-Cross uses a temporary structure to ensure that for each row, we calculate the updates but apply them in a way that avoids overwriting useful information prematurely.  
* **Benefit:** This variant is generally preferred for production implementations as it exhibits better cache locality (ascending memory access) and reduces the constant factors in the runtime complexity.

### **4.2 Sparse Dynamic Programming (Apostolico-Guerra)**

Another notable variant is the Apostolico-Guerra algorithm, often referred to as "Sparse LCS". This approach builds on the Hunt–Szymanski paradigm but incorporates more sophisticated data structures (like balanced trees or persistent trees) to manage the threshold values.

* **Relevance:** This is particularly useful when the number of matches r is neither very small nor very large, but intermediate. It bridges the gap between the log-linear performance of HS and the quadratic performance of standard DP.

### **4.3 Practical Consideration: Memory Layout in WASM**

Implementing these algorithms in a browser-based WebAssembly (WASM) environment requires careful attention to memory layout. WASM uses a linear memory model.

* **Linked Lists vs. Flat Arrays:** The classic description of HS uses a linked list for the MATCHLIST. In WASM, allocating thousands of small linked-list nodes leads to memory fragmentation and poor cache locality.  
  * *Optimization:* Use a **Compressed Sparse Row (CSR)** format.  
  * Allocate two flat Int32Arrays: MatchIndices (stores all match coordinates contiguously) and RowStart (stores the starting index in MatchIndices for each row in A).  
  * This layout is extremely compact and allows the CPU prefetcher to efficiently load match candidates.  
* **Garbage Collection:** By using strictly typed arrays (Int32Array) in JavaScript/WASM rather than arrays of Objects (\[{row: 1, col: 2},...\]), you completely avoid Garbage Collection (GC) pauses, which is critical for maintaining UI responsiveness during large diffs.

### **4.4 The "K-Candidate" Pruning Heuristic**

For spreadsheets, a row at index 100 in File A is unlikely to match a row at index 500,000 in File B unless a massive block move occurred.

* **Heuristic:** You can limit the MATCHLIST construction to a "window" around the diagonal (e.g., |i \- j| \< \\text{WindowSize}).  
* **Trade-off:** This reduces r drastically, enforcing the sparse complexity even on repetitive data. However, it prevents the detection of large block moves (e.g., moving a section from the bottom of the sheet to the top). Since LCS is poor at block moves anyway (marking them as delete+insert), this optimization often improves performance with minimal loss in diff "semantic quality".

## **5\. Applications and Use Cases in the Wild**

### **5.1 Version Control Systems (Git, Mercurial, SVN)**

The history of version control is the history of diff algorithms.

* **Original Unix Diff:** Used Hunt–Szymanski (McIlroy's variation). It worked well because source code lines are typically distinct (variable declarations, function calls).  
* **Git:** The default engine uses **Myers' algorithm** (O(ND)). The Git maintainers found that Myers generally produces "cleaner" diffs (fewer split blocks) for code and handles dense changes better.  
* **Git "Patience" and "Histogram":** Git includes the \--patience and \--histogram options. These are heavily influenced by the philosophy of Hunt–Szymanski. They identify "unique" lines first (anchors) and perform alignment based on them. This is effectively a two-stage HS approach designed to prevent "match noise" (e.g., matching a closing brace } in function A with a closing brace } in function B).

### **5.2 Bioinformatics**

In DNA sequencing, the problem is LCS, but the data shape is the opposite of source code. DNA has a tiny alphabet (A, C, T, G) and massive length.

* **Impact:** Pure Hunt–Szymanski is rarely used because r is massive (\~n^2/4).  
* **Adaptation:** Bioinformatics uses **k-mer** matching. Instead of matching single characters (A=A), they match sequences of length k (e.g., "ACGT" \= "ACGT"). This artificially increases the "alphabet size" and makes the match matrix sparse again, allowing Hunt–Szymanski-like "chaining" algorithms to be used efficiently.

### **5.3 Wiki Engines**

Wiki engines (MediaWiki, DokuWiki) often implement diffing in PHP or Python.

* **Library Support:** Python's difflib uses the Ratcliff-Obershelp algorithm (Gestalt Pattern Matching), not HS or Myers. This is closer to a recursive "longest common substring" approach.  
* **Performance:** For very large wiki pages, these heuristic algorithms can be slow. High-performance implementations often shell out to GNU diff or use native C extensions implementing Myers.

## **6\. Comparison with Alternative Diff/Sequence Algorithms**

For your high-precision spreadsheet diff engine, the choice of algorithm dictates the user experience.

### **6.1 Myers Algorithm (O(ND))**

* **Mechanism:** Myers views the diff problem as finding the shortest path in an "Edit Graph". It searches from the top-left to bottom-right, exploring paths based on the number of edits (D).  
* **Pros:** It is optimal for sequences that are *similar*. If File A and File B differ by only 1 row, Myers finds the diff in O(N) time.  
* **Cons:** It consumes O(N) space (linear) but can degenerate to O(N^2) time if the files are very different. It does not inherently respect the "structure" of the data, sometimes aligning unrelated "}" braces simply to minimize edit count.  
* **Spreadsheet Fit:** Good for cell-level diffing (e.g., "text" vs "test"), but risky for full row alignment on large sheets due to performance cliffs on dissimilar files.

### **6.2 Patience Diff**

* **Mechanism:** Patience Diff (Bram Cohen) first finds all **unique** lines that appear in both files. It computes the LCS of these unique lines (using an algorithm similar to HS/LIS). It then recurses into the "gaps" between these anchors.  
* **Pros:** It produces semantically superior diffs. It avoids matching unrelated common lines (like empty rows).  
* **Cons:** It can fail to find a match if there are *no* unique lines.  
* **Spreadsheet Fit:** **Excellent.** Spreadsheets often have unique identifiers (IDs, timestamps, headers). Aligning these first breaks the massive grid into manageable chunks.

### **6.3 Paul Heckel’s Algorithm**

* **Mechanism:** Published in 1978 (shortly after HS), this algorithm runs in linear time O(N). It uses a symbol table to count occurrences of each line.  
  * Pass 1 & 2: Count occurrences in New and Old files.  
  * Pass 3: Identify lines that appear exactly once in both (Unique Anchors).  
  * Pass 4 & 5: Expand matches from anchors to neighbors (e.g., if Line 10 matches, and Line 11 is identical to Line 11 in the other file, match them too).  
* **Pros:** Extremely fast (O(N)). Handles **block moves** implicitly (if a unique block moves, the anchor moves, and the neighbors follow).  
* **Cons:** It is a heuristic. It does not guarantee the *Longest* Common Subsequence. It might miss matches that strictly finding the LCS would catch.  
* **Spreadsheet Fit:** **Very High.** This is the algorithm behind daff and many tabular diff tools because it handles the "row identity" problem better than LCS.

### **6.4 Histogram Diff**

* **Mechanism:** An optimized version of Patience Diff that uses histograms to find low-frequency matches, not just unique ones.  
* **Pros:** Robust against "semi-repetitive" data.  
* **Cons:** More complex to implement.

## **7\. Fit for Your Product and Data Shapes**

### **7.1 The "Two-Dimensional" Challenge**

Standard diff algorithms are 1D (sequences). Spreadsheets are 2D grids.

* **Row Alignment:** You need to match Row i in A to Row j in B. This is a 1D problem if you treat the entire row as a string.  
* **Column Alignment:** You also need to detect if "Column C" was inserted.  
* **Cell Alignment:** Even if Row 1 matches Row 1, individual cells might have changed.

**Evaluation:** Hunt–Szymanski is a strong candidate for **Row Alignment**, provided you hash the rows first. It naturally handles the case where users insert/delete rows.

### **7.2 The "Repetitive Data" Risk**

Spreadsheets are notorious for repetition.

* **Scenario:** A financial model with 1,000 blank rows for future entry.  
* **Impact on HS:** These blank rows create a dense match matrix (1000 \\times 1000 matches). Hunt–Szymanski will explode to O(N^2 \\log N), freezing the browser.  
* **Impact on Myers:** Myers will struggle to find the "shortest" path through this ocean of identical matches, potentially creating a "snake" path that zig-zags unnaturally.

### **7.3 Constraints: Memory & WASM**

* **Input Size:** 100MB+ files.  
* **Memory:** If you use pure HS with full match lists for dense data, you will exhaust WASM memory (typically limited to 2GB or 4GB, but practical limits are lower).  
* **Latency:** Users expect diffs in seconds. O(N^2) is unacceptable.

## **8\. Recommendations and Design Guidance**

### **8.1 The Hybrid "Anchor-First" Architecture**

Do not rely on a single algorithm. Construct a pipeline that adapts to the data.  
**Step 1: Row Hashing & Deduplication**

* Compute a strong 64-bit hash (e.g., XXHash) for every row content.  
* **Optimization:** Ignore whitespace and styling; diff logic/content only.

**Step 2: Paul Heckel / Patience Pass (The Anchors)**

* Implement a **Patience Diff** or **Heckel-style** pass first.  
* Identify rows that are **unique** in both files.  
* Lock these matches in. They are your "Anchors."  
* *Why?* This prevents the "blank row" catastrophe. Unique rows (headers, IDs, totals) will segment the spreadsheet into small "gaps."

**Step 3: Gap Filling with Hybrid LCS**

* Examine the gaps between anchors.  
* **Small Gaps (\< 1000 rows):** Use **Myers Algorithm**. It finds the minimal edits efficiently for small inputs and is robust.  
* **Large Gaps (\> 1000 rows):**  
  * Check match density. If the gap consists of highly repetitive rows (e.g., all blank), use a **Heuristic Linear Scan** (match top-down until mismatch).  
  * If the gap is random/sparse data, use **Hunt–Szymanski**. Its O(N \\log N) performance will shine here.

**Step 4: Block Move Detection (Post-Processing)**

* LCS algorithms (HS/Myers) identify "Delete Block A" and "Insert Block B".  
* Add a post-processing pass: Compare the hash of "Deleted Block A" with "Inserted Block B".  
* If Hash(A) \== Hash(B), convert the operation to a **"Move"** operation.  
* Visualizing moves is a "killer feature" for spreadsheet diffs that standard text diffs miss.

### **8.2 Handling "Logical Sequences" (Formulas)**

For your requirement to diff "M queries" or "formulas":

* **Do not use string diff.** SUM(A1:A10) vs SUM(A1:A11) is a semantic change.  
* **Tokenization:** Tokenize the formula (Function, OpenParen, Ref, Colon, Ref, CloseParen).  
* **Tree Diff:** For complex logic (M queries), parse into an Abstract Syntax Tree (AST).  
* **Algorithm:** Use **GumTree** or a simplified **Tree Edit Distance** algorithm. However, for a lightweight approach, linearizing the AST tokens and running **Hunt–Szymanski on the token stream** is a highly effective middle ground. It aligns the logical structure better than character-based diff.

### **8.3 Data Structures for WASM**

* **Hash Maps:** Use a high-performance open-addressing hash map (e.g., hashbrown in Rust) to map Row Hashes to IDs.  
* **Match Lists:** As recommended, use flat Int32Arrays in WASM linear memory.  
  * counts: Array of size NumUniqueHashes \+ 1\. Perform a cumulative sum (prefix sum) to determine start indices.  
  * indices: Array of size TotalMatches. Fill this by iterating rows and placing indices into the slots calculated by counts.  
  * This construction is O(N) and extremely cache-friendly.

### **8.4 Summary Recommendation**

**Hunt–Szymanski is a specialized tool, not a silver bullet.** For your product, it should be deployed specifically for **Row Alignment within large, sparse gaps** where unique anchors have already been established. It is superior to Myers for finding matches in shuffled data (which often happens in spreadsheets) but must be guarded against repetitive data pathologies using a Patience/Heckel pre-pass. By implementing this hybrid pipeline in Rust/WASM with flat memory structures, you can achieve sub-second diffs on 100MB+ workbooks while robustly handling the unique structure of tabular data.

### **8.5 Detailed Comparison Matrix**

| Feature | Hunt–Szymanski | Myers (Git Default) | Patience / Heckel | Hybrid (Recommended) |
| :---- | :---- | :---- | :---- | :---- |
| **Core Metric** | Maximize Matches (r) | Minimize Edits (D) | Unique Anchors | Anchors \+ LCS |
| **Complexity** | O((r+n)\\log n) | O(ND) | O(n \\log n) | **Adaptive** |
| **Sparse Data** | 🚀 **Excellent** | 🐢 Slower (D is large) | 🚀 **Excellent** | 🚀 **Excellent** |
| **Dense Data** | 💣 **Catastrophic** | 😐 Acceptable | 😐 Skips non-unique | ✅ **Protected** |
| **Row Alignment** | Good (finds matching rows) | Risk of "snake" alignment | Best for structure | Best of both |
| **Block Moves** | Fails (Del/Ins) | Fails (Del/Ins) | Fails (Del/Ins) | **detects via Post-Process** |
| **Memory** | High (O(r)) | Low (O(n)) | Low (O(n)) | **Optimized (O(n))** |

This architecture ensures you meet your latency budgets (via sparse optimization), memory budgets (via flat arrays), and functional requirements (row alignment and logical sequence diffing).