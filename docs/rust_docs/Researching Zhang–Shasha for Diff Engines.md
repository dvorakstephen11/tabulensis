# **Algorithmic Foundations of Hierarchical Differencing: A Comprehensive Analysis of Zhang-Shasha and Modern Tree Edit Distance for High-Precision Data Engines**

## **1\. Introduction: The Imperative of Semantic Differencing in Structured Data**

In the contemporary landscape of data engineering and software development, the ability to discern and articulate changes between two versions of a digital artifact is a foundational capability. For decades, the industry has relied on linear, line-based differencing tools—epitomized by the Unix diff utility—to manage source code and configuration files. These tools operate on the abstraction of text as a sequence of lines, applying algorithms like the Longest Common Subsequence (LCS) to identify insertions and deletions. While effective for unstructured text or simple codebases where formatting is consistent, this linear abstraction collapses when applied to highly structured, hierarchical data such as spreadsheet models, Abstract Syntax Trees (ASTs) of formula languages, and complex metadata graphs.  
The challenge you face in building a high-precision diff engine for spreadsheet-like data is that your artifacts—Excel workbooks, Power Query (M) scripts, and DAX data models—are not sequences; they are trees and forests. A spreadsheet is not merely a grid of values; it is a hierarchical object graph containing workbooks, sheets, tables, named ranges, and interdependent formulas. An M query is not a list of text lines; it is a sequence of functional steps where the order and nesting of operations define the semantic meaning. A DAX measure is a deeply nested expression tree where wrapping a calculation in a CALCULATE function represents a fundamental logic change, not merely the insertion of text on a new line.  
When linear diffing algorithms are applied to these structures, they fail to capture the *intent* of the user. A block of code moved from one section to another is reported as a massive "delete" followed by an unrelated "insert," destroying the history of the refactoring. A simplified formula that removes a redundant wrapper function is reported as a total replacement of the line, obscuring the fact that the inner logic remains unchanged. For a product aiming for "high precision," this semantic gap is unacceptable. The engine must "see" the tree structure. It must recognize that Table1 was renamed to SalesTable, not that Table1 was deleted and SalesTable created. It must recognize that a column was moved, not destroyed and recreated.  
This report provides an exhaustive, practitioner-focused analysis of the **Zhang-Shasha algorithm**, the seminal method for computing the Tree Edit Distance (TED), and evaluates its suitability as the core engine for your product. However, the landscape of tree comparison has evolved significantly since Zhang and Shasha's publication in 1989\. To architect a solution capable of handling 100MB+ files within the constraints of a browser-based WebAssembly environment, one must look beyond the classical algorithms to the modern optimizations that supersede them.  
This document is structured to guide a senior system architect from the theoretical underpinnings of ordered tree comparison to the concrete design decisions required for implementation. We will dissect the mathematical mechanics of dynamic programming on trees, analyze the "relevant subproblem" constraints that define computational complexity, and contrast the exactness of Zhang-Shasha with modern alternatives like **APTED (All-Path Tree Edit Distance)** and heuristic giants like **GumTree**. Ultimately, we will synthesize these findings into a hybrid architectural recommendation tailored specifically to the "forest" structure of Excel artifacts and the performance rigors of large-scale data processing.

## **2\. Historical Background and Primary Sources**

To truly master the domain of Tree Edit Distance, one must trace the intellectual lineage of the problem. The field has been driven by a single, relentless objective: to lower the computational complexity of comparing hierarchical structures while maintaining the semantic correctness of the edit script.

### **2.1 The Pre-Zhang-Shasha Era: The Complexity Barrier**

The concept of "Edit Distance" originated in string processing, famously formalized by Vladimir Levenshtein in 1965\. The Levenshtein distance measures the dissimilarity between two strings by counting the minimum number of operations (insertions, deletions, substitutions) required to transform one string into the other. As computer science moved from processing flat text to structured data (parse trees in compilers, RNA structures in bioinformatics), the need to generalize this metric to trees became apparent.  
The formal definition of the Tree Edit Distance (TED) problem emerged in the late 1970s. The first non-exponential solution was proposed by **K.C. Tai** in his 1979 paper, "The Tree-to-Tree Correction Problem". Tai’s algorithm was a direct conceptual extension of the string edit distance dynamic programming approach. However, trees introduce a dimension of complexity that strings do not possess: the parent-child relationship. Tai’s algorithm had a time complexity of O(m^3 n^3) and space complexity of O(m^2 n^2), where m and n are the number of nodes in the two trees. While a theoretical breakthrough, proving that the problem was solvable in polynomial time, the O(n^6) complexity rendered it practically useless for any real-world application involving trees with more than a few dozen nodes.

### **2.2 The Seminal Contribution: Zhang and Shasha (1989)**

The field remained stagnant for a decade until 1989, when Kaizhong Zhang and Dennis Shasha published "Simple Fast Algorithms for the Editing Distance Between Trees and Related Problems" in the *SIAM Journal on Computing*. This paper is the primary source for your investigation and represents the inflection point in structural differencing.  
Zhang and Shasha’s critical insight was geometric. They observed that the naive dynamic programming approach (Tai's algorithm) was computing the distance between *every* possible pair of subtrees, many of which were redundant. They proved that for ordered trees, one only needs to compute the edit distance for subtrees rooted at specific nodes, which they termed **Key Roots**. By restricting the recurrence relation to these Key Roots, they drastically pruned the search space.  
The algorithm they proposed achieved a worst-case time complexity of O(m^2 n^2) and, perhaps more importantly for implementation, a space complexity of O(mn). For balanced trees (like typical binary search trees or well-structured heaps), the time complexity drops to O(n^2 \\log^2 n), making it computationally feasible for trees with thousands of nodes. This algorithm became the gold standard—the "textbook" algorithm—for the next twenty years. It balanced exactness (guaranteeing the minimal edit script) with efficiency for standard tree shapes.  
However, the "worst-case" scenario for Zhang-Shasha—deep, linear trees (essentially linked lists)—remained O(n^4). In the context of your product, an M query step chain or a deeply nested DAX expression could resemble this pathological shape, posing a risk to latency guarantees.

### **2.3 The Modern Era: Breaking the Cubic Barrier**

Following Zhang-Shasha, the academic community raced to break the asymptotic barriers, driven largely by the needs of the bioinformatics community (comparing RNA secondary structures) and the XML database community.

* **Klein (1998):** Philip Klein introduced "Heavy Path Decomposition" in his paper "Computing the Edit-Distance Between Unrooted Ordered Trees". Klein realized that if the recursion favored the "heaviest" child (the child with the most descendants), the depth of the recursion could be bounded logarithmically. This reduced the worst-case time complexity to O(n^3 \\log n), a significant improvement over Zhang-Shasha's O(n^4) for skewed trees.  
* **Demaine et al. (2009):** Erik Demaine and colleagues proposed an optimal decomposition strategy that achieved O(n^3) worst-case time. This was a theoretical optimal for the specific class of decomposition strategies they analyzed.  
* **RTED (2011) & APTED (2016):** The current state-of-the-art was established by Mateusz Pawlik and Nikolaus Augsten. They first introduced **RTED (Robust Tree Edit Distance)**, which was the first algorithm to effectively "hybridize" the strategies of Zhang-Shasha (efficient for balanced trees) and Klein/Demaine (efficient for linear trees). They later refined this into **APTED (All-Path Tree Edit Distance)**. APTED dynamically selects the optimal decomposition strategy (Left, Right, or Heavy path) for *each* subproblem instance during execution. It guarantees O(n^3) time and O(n^2) space, effectively superseding Zhang-Shasha for all exact matching use cases.

### **2.4 The Heuristic Turn: GumTree (2014)**

While exact algorithms pushed the boundaries of polynomial time, software engineering researchers realized that for source code, "exactness" (finding the mathematically minimal edit script) was sometimes less important than "naturalness" (finding an edit script that matches human intuition). In 2014, Falleri et al. published "Fine-grained and Accurate Source Code Differencing," introducing **GumTree**.  
GumTree abandoned the strict dynamic programming requirement in favor of a two-phase heuristic approach:

1. **Top-Down Anchoring:** Greedily finding the largest identical subtrees to serve as "anchors."  
2. **Bottom-Up Propagation:** Extending those matches to parents if enough children match.

This approach allows GumTree to handle massive ASTs (tens of thousands of nodes) in near-linear time (O(n^2) worst case but often O(n) practical), and critically, it introduced the ability to detect **Move** operations—something the classical Zhang-Shasha algorithm does not natively support.  
For your product, understanding this historical progression is vital. Zhang-Shasha is the foundation, APTED is the modern exact engine, and GumTree is the scalable, semantic wrapper.

## **3\. Formal Description and Complexity**

To engineer a diff engine, one must move beyond the abstract idea of "comparison" to the rigorous mathematical definition of distance. The Zhang-Shasha algorithm is defined on **rooted, ordered, labeled trees**.

* **Rooted:** There is a single node (the root) from which all other nodes descend.  
* **Ordered:** The order of siblings is significant. In a spreadsheet formula, SUM(A1, B1) is structurally different from SUM(B1, A1) if we strictly respect argument order (though semantically they may be equivalent, the ASTs differ).  
* **Labeled:** Each node has a label (e.g., Function: SUM, Cell: A1, Operator: \+).

### **3.1 The Edit Operations and Cost Model**

The Tree Edit Distance \\delta(T\_1, T\_2) is formally defined as the minimum total cost of a sequence of edit operations that transforms tree T\_1 into tree T\_2. The standard set of operations allows for identifying structural changes with precision:

1. **Rename (Change Label):** Transform a node u with label L\_1 into a node u with label L\_2.  
   * *Notation:* u \\to v  
   * *Cost:* \\gamma(u \\to v). Typically, if L\_1 \= L\_2, the cost is 0; otherwise, it is 1 (or a weighted value based on label similarity).  
   * *Product Relevance:* Essential for detecting when a variable name changes in M code or a measure name changes in DAX.  
2. **Delete Node:** Remove a node u from the tree.  
   * *Notation:* u \\to \\Lambda  
   * *Mechanism:* Unlike deleting a file in a directory, deleting a node in TED **promotes its children**. If node u has children c\_1, c\_2,..., c\_k, deleting u causes c\_1...c\_k to become children of u's parent, inserted in place of u.  
   * *Cost:* \\gamma(u \\to \\Lambda).  
   * *Product Relevance:* This semantic is crucial for "unwrapping." If a user changes CALCULATE(SUM(Sales)) to SUM(Sales), they have "deleted" the CALCULATE node. The SUM node (the child) is promoted. A linear diff would see this as replacing the whole line; TED sees it as a structural simplification.  
3. **Insert Node:** Insert a new node v into the tree.  
   * *Notation:* \\Lambda \\to v  
   * *Mechanism:* Node v is inserted as a child of some node p. v can adopt a consecutive subsequence of p's existing children as its own children.  
   * *Cost:* \\gamma(\\Lambda \\to v).  
   * *Product Relevance:* Crucial for "wrapping." Changing SUM(Sales) to CALCULATE(SUM(Sales)) involves inserting CALCULATE and making SUM its child.

### **3.2 The Zhang-Shasha Recurrence Relation**

The computational heart of the algorithm is a dynamic programming recurrence. To understand it, we must first define the indexing scheme. The algorithm uses a **post-order traversal** to number the nodes from 1 to |T|.

* Let T\[i\] be the i-th node in the post-order traversal of tree T.  
* Let l(i) be the index of the **leftmost leaf** of the subtree rooted at T\[i\].  
* Let T\[i..j\] denote the ordered subforest consisting of nodes with post-order indices between i and j.  
* Let F(i) be the forest induced by the nodes in the subtree rooted at T\[i\].

The goal is to compute the distance between the forest rooted at i in T\_1 and the forest rooted at j in T\_2. The recurrence relations for the distance d(T\_1\[l(i)..i\], T\_2\[l(j)..j\]) are as follows:  
The third case (Match/Rename) is where the tree structure is handled. If T\_1\[i\] and T\_2\[j\] are mapped to each other, we must account for the cost of renaming them (if labels differ) plus the cost of transforming the forest of T\_1\[i\]'s children into the forest of T\_2\[j\]'s children.

### **3.3 The "Key Root" Optimization**

Solving the recurrence for *all* pairs of subforests would lead to the O(m^3 n^3) complexity of Tai's algorithm. Zhang and Shasha's breakthrough was identifying that we do not need to solve the general forest problem for every node. We only need to solve it for **Key Roots**.  
**Definition:** A node is a **Key Root** if it is the root of the tree OR if it has a left sibling.

* Formally: KR(T) \= \\{k \\in T \\mid \\text{parent}(k) \\text{ exists implies } k \\text{ is not the leftmost child} \\} \\cup \\{ \\text{root}(T) \\}.

**Intuition:** If a node x is the leftmost child of its parent p, then the forest formed by the subtree of x is a proper prefix of the forest formed by the subtree of p. The recursion can "slide" up the left edge without starting a new dynamic programming table. The calculation only needs to "reset" (start a new sub-problem) when we encounter a node that branches off—i.e., a node that has a left sibling.  
The algorithm proceeds by:

1. Identifying all Key Roots in T\_1 and T\_2.  
2. Iterating through each pair of Key Roots (k\_1, k\_2).  
3. For each pair, computing the edit distance between the tree rooted at k\_1 and the tree rooted at k\_2 using a constrained DP table.

### **3.4 Complexity Analysis**

* **Time Complexity:** The time complexity is proportional to the product of the number of Key Roots and the sizes of the trees. More tightly, it is bounded by:  
  * **Worst Case:** For a linear tree (a linked list), every node is a key root (since the root is a key root, but wait—in a linear tree, only the root is a key root? No. In a linear tree A \\to B \\to C, B is the leftmost child of A, so it is *not* a key root. C is the leftmost child of B, so it is *not* a key root. Wait. If the tree is a single line, only the global root is a Key Root. The worst case is actually a specific "zigzag" or balanced structure where the number of Key Roots is maximized relative to depth? Actually, the standard analysis states that for a complete binary tree, |KR| \\approx |T|/2, yielding O(n^4)? No, for balanced trees it is O(n^2 \\log^2 n). The worst case O(n^4) occurs for specific tree shapes that defeat the left-path optimization, often cited as "standard" trees in some contexts, but strictly speaking, the worst case is when the number of key roots is O(n) and the depth is O(n), which implies a dense branching structure that isn't a simple line).  
  * Let's correct the intuition: The algorithm is efficient (O(n^2)) when the number of Key Roots is small (like a line) or when the depth is small (like a flat fan-out). The complexity spikes when *both* the number of key roots and the depth are significant.  
* **Space Complexity:** O(|T\_1| \\cdot |T\_2|).  
  * This is the **primary bottleneck** for your 100MB constraint. A 100MB file could map to 10^6 nodes. The DP table would require 10^{12} entries. Assuming 4 bytes per integer, that is 4 Terabytes of RAM.  
  * **Conclusion:** You **cannot** run Zhang-Shasha on the global tree of a massive workbook. You must segment the problem.

## **4\. Plain-English Explanation and Intuition**

For the engineering team and stakeholders who may not be algorithmists, it is essential to build an intuition for *how* the algorithm "sees" data.

### **4.1 The Directory Analogy**

Imagine you are comparing two file system directories, Folder A and Folder B. Your goal is to write a script of shell commands (rm, mkdir, mv, rename) to transform A into B.

* **String Diff Approach:** A linear diff tool (like diff \-r) walks the directory listing alphabetically. If Folder A contains file1.txt and Folder B contains file2.txt, it says "Delete file1, Insert file2." It doesn't "understand" that file1 might have been renamed to file2 or moved into a subfolder.  
* **Tree Edit Distance Approach:** TED looks at the structure. It asks: "Is the cost of deleting file1 and creating file2 cheaper than the cost of renaming file1 to file2?"  
  * If file1 is a huge folder with 1,000 children, and file2 has the same 1,000 children, the cost of "Delete \+ Insert" is massive (you have to delete 1,000 children and re-insert them).  
  * The cost of "Rename" is tiny (1 operation).  
  * Therefore, the algorithm chooses "Rename." It "sees" the stability of the sub-structure.

### **4.2 Visualizing the Recursion**

Think of the algorithm as a "matcher" trying to overlay Tree A onto Tree B.

1. It starts at the leaves (the bottom). It calculates the cost to match leaf A1 to leaf B1.  
2. It moves up to the parents. When considering Parent A and Parent B, it looks at the costs it already calculated for the children.  
3. It checks three options:  
   * **Can I match Parent A to Parent B?** If so, I pay the rename cost, plus the cost of aligning their children (which I just calculated).  
   * **Should I delete Parent A?** If I do, its children (Child A\_1, Child A\_2) are now "orphans" that need to be matched against Parent B's children.  
   * **Should I insert Parent B?** If I do, I need to match Parent A's children against the *grandchildren* of Parent B (or an empty set).

### **4.3 The "Islands of Stability"**

The "Key Root" concept can be visualized as identifying the "Islands of Stability" in the tree.

* A long linear chain of nodes (e.g., A \-\> B \-\> C \-\> D) is structurally simple. It's just a string. The algorithm processes this quickly, effectively performing a string diff.  
* The complexity happens at **branching points** (e.g., an IF statement in M code, or a Folder with multiple subfolders). These are the "Key Roots."  
* The algorithm essentially says: "I will only perform the expensive, full-matrix comparison at these chaotic branching points. For the linear roads between them, I'll just carry the values forward."

### **4.4 Why this matters for Excel**

Consider a User entering a formula: SUM(A1:A10). Later, they realize they need to filter it: CALCULATE(SUM(A1:A10), Filter(Table, \[Col\]\>5)).

* **Cognitive Model:** The user *wrapped* the SUM. They didn't delete it.  
* **Tree Diff:** Recognizes that the SUM subtree is identical. It identifies that CALCULATE was inserted as the new parent. The diff output can explicitly say: "Wrapped SUM(...) in CALCULATE."  
* **Value:** This preserves the user's mental model of the change, which is critical for reviewing complex financial models.

## **5\. Variants, Implementation Patterns, and Practical Considerations**

While Zhang-Shasha provides the theoretical foundation, practically implementing a diff engine for 100MB files requires navigating a landscape of optimizations and modern variants.

### **5.1 APTED: The Robust Successor**

**APTED (All-Path Tree Edit Distance)** is the algorithm you should likely implement for your "exact diff" kernel, rather than vanilla Zhang-Shasha.

* *The Flaw in Zhang-Shasha:* The algorithm is statically biased toward the "Left" path decomposition. It always peels off the leftmost leaf. If the tree is "Right-heavy" (most nodes are right children), this is efficient. If the tree is "Left-heavy," it degrades to O(n^4).  
* **The APTED Innovation:** APTED generalizes the decomposition. For every subproblem (comparing forest F and forest G), it calculates the size of the recursion tree for all possible strategies:  
  1. Decompose along the **Left** path.  
  2. Decompose along the **Right** path.  
  3. Decompose along the **Heavy** path (the child with the most descendants).  
* **Performance:** By choosing the optimal strategy at each step, APTED guarantees O(n^3) time complexity and O(n^2) space for *any* tree shape.  
* **Why it matters for you:** User-generated Excel formulas and M queries are unpredictable. A deeply nested IF(IF(IF...)) chain can create pathological left-heavy or right-heavy trees. APTED protects your engine from worst-case latency spikes that would freeze the browser.

### **5.2 RTED (Robust Tree Edit Distance)**

RTED was the predecessor to APTED. It computes the optimal strategy *before* running the recurrence. APTED improved on this by managing memory more efficiently during the strategy execution. **Recommendation:** Skip RTED and go straight to APTED.

### **5.3 Memory Optimization: Linearized Trees**

The O(n^2) space constraint is severe. A key optimization pattern for systems programming (Rust/C++) is **Tree Linearization**. Instead of representing the tree as a graph of heap-allocated pointers (struct Node { children: Vec\<Box\<Node\>\> }), which causes memory fragmentation and poor cache locality:

1. **Pre-order Array:** Flatten the tree into a single array of nodes in pre-order traversal.  
2. **Euler String:** Represent the tree as a string of labels including backtracking tokens (e.g., A, B, $, C, $).  
3. **Implementation Benefit:** APTED and ZS can be rewritten to operate on integer indices into these arrays rather than pointers. This allows the entire tree to reside in a single contiguous block of WebAssembly memory (an "Arena"), significantly reducing overhead and allowing for O(1) serialization/deserialization when passing data between the JS thread and Wasm worker.

### **5.4 Heuristic: PQ-Gram**

For scenarios where exact edit scripts are not required—such as detecting "fuzzy duplicate" sheets or finding similar formulas across the workbook—**PQ-Gram** is a valuable tool.

* **Mechanism:** It generates a "fingerprint" for a tree by decomposing it into small subtrees of shape (p, q) (where p is ancestor depth and q is sibling width).  
* **Comparison:** It computes the set intersection of these fingerprints (like Jaccard distance).  
* **Complexity:** O(n \\log n).  
* **Application:** Use this for a "Quick Diff" overview or to detect moved worksheets where the content has changed slightly. It cannot generate a precise edit script, but it acts as a high-speed filter.

## **6\. Applications and Use Cases in the Wild**

Validating the architectural approach requires examining how established tools solve similar problems.

### **6.1 GumTree: The Industry Standard for Code**

**GumTree** is the algorithm behind widely used semantic diff tools in the software industry (e.g., GitHub's semantic diffs). It addresses the two main failings of exact TED algorithms: quadratic complexity and lack of "Move" detection.  
**The GumTree Workflow:**

1. **Greedy Top-Down (Anchoring):** It scans both trees to find the largest subtrees that are structurally identical (isomorphic) and have the same hash. These are "anchored." (Time: O(n) with hashing).  
   * *Insight:* If a large block of code (or a large Table in Excel) is unchanged, GumTree locks it instantly. Zhang-Shasha would waste millions of cycles proving it is unchanged node-by-node.  
2. **Bottom-Up (Propagation):** It iterates through the anchors. If the parents of two anchored nodes are not yet matched, but they share a significant percentage of matching children (e.g., \>50% Dice coefficient), it matches the parents.  
   * *Insight:* This detects **Renames**. If FunctionOld(A, B) becomes FunctionNew(A, B), the arguments A and B are anchors. The parents match because they share the same children.  
3. **Recovery Phase:** For the "holes" between anchors (the actually changed parts), it runs a precise difference. Originally, it used an optimal TED here; modern versions often use simplified list-diffs.  
   * *Insight:* This is the pattern for your engine. Use GumTree to handle the 100MB file, breaking it down into small "changed zones," then run APTED only on those zones.

### **6.2 XML Database Differencing (X-Diff)**

In the early 2000s, XML databases faced the same problem. Tools like **X-Diff** introduced the concept of "unordered" tree matching for data-centric XML (where child order doesn't matter) vs. "ordered" matching for document-centric XML.

* **Relevance:** Excel tables are often "unordered" (row order might not matter if there is a primary key). However, formulas are strictly "ordered." Your engine must support both modes: Ordered TED for formulas/M, and Unordered matching (likely Key-based hashing) for data tables.

### **6.3 Applications in Bioinformatics**

Bioinformatics heavily uses TED for comparing RNA secondary structures. However, they deal with trees of limited depth. Their optimizations often focus on parallelization (GPU-based TED). While interesting, the constraint in a browser (Wasm single-threaded performance limits) makes the algorithmic efficiency of APTED/GumTree more valuable than raw parallel throughput.

## **7\. Comparison with Other Tree and Sequence Diff Algorithms**

To justify the selection of Zhang-Shasha/APTED, we must contrast it with the alternatives.

| Algorithm | Time Complexity | Space Complexity | Move Support | Use Case | Strengths | Weaknesses |
| :---- | :---- | :---- | :---- | :---- | :---- | :---- |
| **Zhang-Shasha** | O(n^4) (worst) | O(n^2) | Implicit | General Tree Diff | Exact minimal edit script. | Performance unstable on deep trees. |
| **APTED** | **O(n^3)** | **O(n^2)** | Implicit | **Formula Precision** | **State-of-the-art exactness. Stable.** | Quadratic space limits max input size (\~2k nodes). |
| **GumTree** | O(n^2) (typ. linear) | O(n) | **Explicit** | **Global Structure** | **Scales to huge files. Detects Moves.** | Heuristic. Can miss optimal edits in small dense areas. |
| **PQ-Gram** | O(n \\log n) | O(n) | No | Similarity / Clustering | Extremely fast approximation. | No edit script. Just a score. |
| **Myers (Linear)** | O(ND) | O(N) | No | Text / Lists | Standard for text. | Semantic blindness. Fails on wrapped code. |

**The Case Against Linear Diff:** Applying Myers' algorithm (standard git diff) to M code or DAX frequently results in "conflicts" that aren't real conflicts. A reformatting of M code (adding newlines) causes linear diff to mark the whole file as changed. A Tree Diff (ZS/APTED) sees that the structure is identical (distance 0\) despite the whitespace, eliminating false positives.  
**The Case for "Move" Support:** Exact TED algorithms (ZS/APTED) define distance via Insert/Delete/Rename. They mathematically simulate a "Move" as deleting a subtree here and inserting it there. This is frustrating for users who want to see "Moved Table1 to Group B." GumTree is the only algorithm in this list that explicitly models moves as a first-class citizen by linking the source and destination subtrees.

## **8\. Fit for Product and Data Shapes**

Your specific data context (Excel/Power BI) requires a nuanced application of these algorithms. We must classify the data shapes into three tiers.

### **8.1 Tier 1: The Forest (Workbook Structure)**

* **Data:** Workbook \\to Sheets \\to Tables \\to Columns.  
* **Shape:** Broad, shallow tree. High fan-out (100 sheets), low depth (4-5 levels).  
* **Behavior:** Users reorder sheets, rename tables, move columns.  
* **Algorithm Fit:** **GumTree**.  
  * *Why:* The graph is large enough that O(n^3) might be slow if there are thousands of columns. More importantly, the "Move" detection is the primary requirement. If a user drags "Sheet1" to the end, GumTree detects Move(Sheet1, Index: 99). ZS would report Delete(Sheet1) at index 0 and Insert(Sheet1) at index 99, losing the identity of the sheet.

### **8.2 Tier 2: The Logic (Formulas, M, DAX)**

* **Data:** ASTs of expressions. CALCULATE(SUM(),...)  
* **Shape:** Deep, narrow trees. Recursive function calls. Typically \< 1,000 nodes per formula.  
* **Behavior:** Users wrap functions, change parameters, rename variables.  
* **Algorithm Fit:** **APTED**.  
  * *Why:* Precision is king. The trees are small enough that O(n^3) is effectively instantaneous (\< 10ms). You need the exact edit script to show "Wrapped SUM in CALCULATE." Heuristics like GumTree might misinterpret subtle logic changes in dense formulas.  
  * *Constraint:* You must implement the parser (using a tool like tree-sitter or a custom recursive descent parser) to convert the text code into the labeled ordered tree required by APTED.

### **8.3 Tier 3: The Grid (Cell Values)**

* **Data:** 2D matrix of values (100k \- 1M cells).  
* **Shape:** Technically a tree (Sheet \\to Row \\to Cell), but functionally a dense matrix.  
* **Constraint:** **DO NOT USE TREE EDIT DISTANCE.**  
  * Running O(n^3) on 100,000 cells is 10^{15} operations. It will never finish.  
* **Algorithm Fit:** **Sequence Diff / Hashing**.  
  * Treat rows as atomic units. Hash each row. Run standard Myers diff (LCS) on the sequence of row hashes.  
  * If a row hash differs, run a column-wise sequence diff on the cells within that row.  
  * Only promote to Tier 2 (APTED) if the *content* of a specific cell is a complex formula that changed.

### **8.4 WebAssembly Constraints**

* **Memory:** Wasm has a hard memory limit (typically 4GB, but practically you want to stay under 500MB to avoid crashing the tab).  
* **ZS/APTED Matrix:** An APTED DP table for 2,000 nodes is \\approx 16MB. This is safe. For 20,000 nodes, it is 1.6GB.  
* **Design Rule:** You must enforce a "Bail-out" threshold. If a formula AST exceeds 2,000 nodes (rare, but generated M code can be huge), fallback to a faster, linear diff or a GumTree approximation. Do not attempt exact TED on massive generated code.

## **9\. Recommendations and Design Guidance**

Based on this research, I propose a **Hybrid-Tiered Architecture** for your diff engine. Do not rely on a single algorithm; dispatch the artifact to the correct solver based on its structural characteristics.

### **9.1 Recommended Architecture**

1. **The "GumTree" Global Matcher (Rust/Wasm):**  
   * Implement a custom version of the GumTree algorithm to act as the entry point.  
   * **Input:** The high-level object graph (Sheets, Tables, Named Ranges, M Query names).  
   * **Phase 1 (Top-Down):** Hash every subtree. Match identical hashes. This instantly aligns 90% of the workbook that hasn't changed.  
   * **Phase 2 (Bottom-Up):** For modified objects, check their children. If \>60% of children match (using unique IDs or content hashes), link the parents as a "Rename" or "Modification."  
   * **Output:** A set of "Matched Nodes" and a set of "Changed Subtrees."  
2. **The "APTED" Micro-Solver:**  
   * For every pair of "Changed Subtrees" identified above (e.g., Query1 in V1 vs Query1 in V2), check the node count.  
   * **If count \< 2,000:** Pass the subtrees to the **APTED** engine.  
     * Implement APTED using **linearized tree arrays** (pre-order traversal vectors) to minimize Wasm memory overhead.  
     * Output the exact edit script (Insert, Delete, Rename).  
   * **If count \> 2,000:** Fallback to a linear diff on the pretty-printed text of the code/formula. It is better to give a fast text diff than to hang the browser trying to compute a tree diff for a massive generated query.  
3. **The "Sequence" Grid Solver:**  
   * For Sheet content, perform a Row-Level Hash Diff.  
   * Do not build a tree of cells. Keep the grid as a flat vector of rows.  
   * Identify inserted/deleted rows first.  
   * For modified rows, identify inserted/deleted columns.

### **9.2 Implementation Roadmap**

1. **Data Structure Design:** Define a Tree struct in Rust that uses Vec\<Node\> and integer indices (u32) instead of pointers. This is critical for serialization performance between JS and Wasm.  
2. **Parser Integration:** Build or integrate parsers for M and DAX that output directly into this Tree struct. Ensure "Key Roots" and "Leftmost Leaf" indices are calculated *during parsing* to avoid an extra O(n) pass later.  
3. **APTED Implementation:** Port the Java reference implementation of APTED to Rust. Focus on the "All-Path" strategy selection logic.  
4. **Heuristic Layer:** Build the GumTree matching logic on top. The ability to detect "Moves" is the killer feature that will distinguish your product from basic diff tools.

### **9.3 Conclusion**

Zhang-Shasha provides the fundamental logic for your engine, but it is the **APTED** algorithm that makes it robust enough for production, and the **GumTree** heuristics that make it scalable enough for 100MB files. By treating the workbook as a forest of different tree types—applying heuristics to the macro-structure and exact algorithms to the micro-logic—you can achieve the "high precision" goal without violating the memory and latency constraints of the browser.

#### **Works cited**

1\. Tree Edit Distance, http://tree-edit-distance.dbresearch.uni-salzburg.at/ 2\. Revisiting the tree edit distance and its backtracing: A tutorial \- arXiv, https://arxiv.org/pdf/1805.06869 3\. Simple Fast Algorithms for the Editing Distance Between Trees and Related Problems, https://www.researchgate.net/publication/220618233\_Simple\_Fast\_Algorithms\_for\_the\_Editing\_Distance\_Between\_Trees\_and\_Related\_Problems 4\. Simple Fast Algorithms for the Editing Distance between Trees and Related Problems | SIAM Journal on Computing, https://epubs.siam.org/doi/10.1137/0218082 5\. Computing the Edit-Distance Between Unrooted Ordered Trees, http://128.148.32.110/research/pubs/pdfs/1998/Klein-1998-CED.pdf 6\. RTED: A Robust Algorithm for the Tree Edit Distance \- arXiv, https://arxiv.org/pdf/1201.0230 7\. RTED: A Robust Algorithm for the Tree Edit Distance \- VLDB ..., https://vldb.org/pvldb/vol5/p334\_mateuszpawlik\_vldb2012.pdf 8\. Tree edit distance: Robust and memory-efficient \- ResearchGate, https://www.researchgate.net/publication/281324160\_Tree\_edit\_distance\_Robust\_and\_memory-efficient 9\. Fine-grained and Accurate Source Code Differencing: GumTree \- Computer Science (CS), https://courses.cs.vt.edu/cs6704/spring17/slides\_by\_students/CS6704\_gumtree\_Kijin\_AN\_Feb15.pdf 10\. \[PDF\] Fine-grained and accurate source code differencing \- Semantic Scholar, https://www.semanticscholar.org/paper/Fine-grained-and-accurate-source-code-differencing-Falleri-Morandat/099c9fa379eeb10222969ce0073968ea47417768 11\. The pq-gram distance between ordered labeled trees \- Semantic Scholar, https://www.semanticscholar.org/paper/The-pq-gram-distance-between-ordered-labeled-trees-Augsten-B%C3%B6hlen/1cf982a65c9242c9e7da2ff6e83fdc5d052303e9 12\. Algorithms for Web Scraping \- DTU Informatics, https://www2.imm.dtu.dk/pubdb/edoc/imm6183.pdf 13\. Fine-grained and Accurate Source Code Differencing \- LaBRI, https://www.labri.fr/perso/falleri/img/slides/ase14.pdf