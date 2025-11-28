

# **Algorithmic Foundations of High-Fidelity Diff Engines: A Comprehensive Analysis of the Hungarian Method and Linear Assignment Problems**

## **1\. Introduction: The Intersection of Combinatorial Optimization and Version Control**

The construction of high-quality difference (diff) engines represents a unique challenge at the convergence of software engineering, information theory, and combinatorial optimization. For decades, the pervasive Myers algorithm and its variants have served as the backbone of text-based version control systems like Git, Subversion, and Mercurial. These algorithms fundamentally operate on the principle of the Longest Common Subsequence (LCS), a method optimized for characterizing insertions and deletions in sequential data streams.1 While robust for simple text editing, the LCS approach falters significantly when addressing the complexities of moved blocks of code, reordered data rows in large spreadsheets, or structural refactoring in hierarchical documents. The inability of traditional sequential algorithms to distinguish between a "move" and a "delete-then-insert" operation results in "noisy" diffs that obscure developer intent and complicate code review, merging, and auditing processes.3

To transcend these limitations and build the next generation of "semantic" or "structure-aware" diff engines, practitioners must look towards the Linear Assignment Problem (LAP) and its canonical solution: the Hungarian algorithm, also known as the Kuhn-Munkres algorithm.5 Unlike the classic LCS approach, which seeks to maximize the length of a preserved sequence, the assignment problem seeks to minimize the total cost of transforming one dataset into another, allowing for the detection of non-sequential correspondences. This distinction is critical for "move detection"—the holy grail of modern differencing tools—where a block of code moved from the top of a file to the bottom should be recognized as a movement operation rather than a disjointed deletion and insertion.7

This report provides an exhaustive examination of the Hungarian algorithm and its modern successors, specifically tailored for engineers and researchers tasked with detecting optimal matching in complex datasets. We will traverse the historical and mathematical landscape of the Hungarian method, dissect its computational complexity, and rigorously evaluate its performance against modern alternatives like the Jonker-Volgenant (LAPJV) algorithm.8 Furthermore, we will explore the practical application of these algorithms in differencing engines, addressing the formidable challenges of cost matrix construction, sparse approximation, and scaling to large datasets using heuristics like Locality-Sensitive Hashing (LSH).10 By bridging the gap between abstract optimization theory and concrete software implementation, this report aims to serve as a definitive guide for architecting diff engines that are not only mathematically optimal but also computationally feasible for real-world scale.

## **2\. The Hungarian Algorithm: Theoretical Underpinnings and Mechanics**

The Hungarian algorithm stands as a monumental achievement in the history of combinatorial optimization. Developed by Harold Kuhn in 1955 and later refined by James Munkres in 1957, it provided the first polynomial-time solution to the assignment problem.5 Its name pays homage to the Hungarian mathematicians Dénes Kőnig and Jenő Egerváry, whose earlier work on bipartite matching formed the theoretical bedrock upon which Kuhn built his method.5 It is worth noting that while Kuhn formalized the method in 1955, historical analysis has revealed that Carl Gustav Jacobi had solved the assignment problem in the 19th century, though his work was published posthumously in Latin and remained largely unnoticed by the optimization community until 2006\.6

### **2.1 The Assignment Problem Formulation**

To understand the algorithm, one must first rigorously define the problem it solves. The Linear Assignment Problem (LAP) can be conceptualized in two primary ways: the matrix formulation and the bipartite graph formulation. In the context of a diff engine, this mathematical abstraction translates directly to the problem of aligning elements (rows, lines, or nodes) between two versions of a file.

The Matrix Formulation:  
Given two sets of equal size $N$, often referred to as "workers" and "jobs" (or in the context of diffing, "original lines" and "new lines"), we construct an $N \\times N$ cost matrix $C$. The element $C\_{ij}$ represents the cost of assigning worker $i$ to job $j$. The objective is to find a permutation $P$ of the jobs such that the total cost is minimized:

$$\\min \\sum\_{i=1}^{N} C\_{i, P(i)}$$  
This formulation is intuitive for diff engines: the cost $C\_{ij}$ is a measure of dissimilarity between line $i$ of the old file and line $j$ of the new file. A perfect match might have a cost of 0, while a completely different line has a high cost (potentially infinite if matching is forbidden).6 If the sets are of unequal size (e.g., lines were added or removed), the matrix requires padding with dummy rows or columns containing high costs (or zeros, depending on the maximization/minimization goal) to become square, allowing the algorithm to proceed.11

The Bipartite Graph Formulation:  
Alternatively, the problem can be modeled as finding a minimum-weight perfect matching in a weighted bipartite graph $G \= (U, V, E)$. Here, $U$ represents the set of original items, $V$ represents the set of new items, and the edges $E$ carry weights corresponding to the costs. The goal is to select a subset of edges such that every vertex in $U$ is connected to exactly one vertex in $V$, and the sum of the weights of these edges is minimized.13 This graph-theoretic perspective is particularly useful when implementing sparse approximations, as edges with infinite costs (representing totally dissimilar lines) can simply be omitted from the graph structure, saving memory and computation.12

### **2.2 Algorithmic Mechanics and Complexity**

The Hungarian algorithm relies on the concept of "potential" functions (dual variables) and the principle of complementary slackness. It is a primal-dual method that iteratively improves a feasible dual solution to find an optimal primal solution. The algorithm fundamentally works by transforming the cost matrix to equivalent matrices (where the optimal assignment remains the same) until the solution becomes obvious—specifically, when it is possible to select $N$ independent zeros (one in each row and column).15

The Original $O(n^4)$ Approach:  
Kuhn's original publication presented an algorithm that, while polynomial, operated with a complexity of $O(n^4)$.6 The process involves a sequence of reduction and covering steps:

1. **Row/Column Reduction**: The algorithm begins by subtracting the minimum value from each row and then from each column. This ensures that every row and column contains at least one zero. Since subtracting a constant from a row or column affects the cost of all potential assignments involving that row or column equally, the optimal assignment for the reduced matrix is identical to that of the original matrix.  
2. **Zero Covering**: The core iterative step involves attempting to "cover" all zeros in the resulting matrix with the minimum possible number of lines (rows or columns). This is related to Kőnig's theorem, which states that in any bipartite graph, the number of edges in a maximum matching equals the number of vertices in a minimum vertex cover.  
3. **Optimality Check**: If the minimum number of lines required to cover all zeros is equal to $N$, then an optimal assignment exists among the zeros. The algorithm terminates, and the assignment is constructed.  
4. **Matrix Adjustment**: If fewer than $N$ lines are needed, the matrix is modified to create more zeros. The algorithm finds the smallest uncovered value, subtracts it from all uncovered elements, and adds it to elements at the intersection of two covering lines. This preserves the non-negativity of the matrix (conceptually) and shifts the zeros to new positions, allowing the algorithm to proceed.6

The $O(n^3)$ Refinement:  
It was later observed by Edmonds, Karp, and Tomizawa that the algorithm could be implemented in $O(n^3)$ time.6 This improvement is not merely an implementation detail but a fundamental structural change in how the augmentation step is handled. The $O(n^4)$ complexity arises because, in the naive implementation, finding the minimum uncovered element requires scanning the entire matrix in each step.  
The $O(n^3)$ refinement is achieved by maintaining the dual variables and using a technique similar to Dijkstra's algorithm to find the shortest augmenting path in the residual graph during each stage of the assignment augmentation.17 Specifically, instead of scanning the entire matrix to find the minimum uncovered element (which contributes to the $O(n^4)$ behavior), one can maintain an array of "slack" values. This slack vector tracks the minimum distance of each uncovered column to the tree of alternating paths rooted at the current unassigned row. By updating this slack vector incrementally, the cost of the adjustment step is reduced from $O(n^2)$ to $O(n)$, bringing the total complexity of the algorithm (which iterates $n$ times) to $O(n^3)$.6

This distinction is vital for diff engine developers. An $O(n^4)$ implementation will collapse under the weight of even moderately sized files (e.g., 500 lines), taking significantly longer than a user is willing to wait. An optimized $O(n^3)$ implementation, however, pushes the feasibility boundary to files with several thousand lines, especially when implemented in performant languages like C++ or Rust.18

### **2.3 Significance for Diff Engines**

For a diff engine, the Hungarian algorithm guarantees the *globally optimal* alignment of elements. If one treats the "cost" as the Levenshtein distance or Jaccard dissimilarity between lines, the Hungarian algorithm will find the unique mapping of lines that minimizes the total edit distance.12

This is a profound improvement over greedy heuristics. Consider a scenario where a function calculateTotal() is moved from the top of a file to the bottom, and a very similar function calculateSubtotal() is introduced where the original used to be. A greedy algorithm or a simple nearest-neighbor approach might incorrectly match the new calculateSubtotal() to the old calculateTotal() because they are spatially close and textually similar. The Hungarian algorithm, however, would evaluate the global cost and likely determine that matching the moved calculateTotal() to its new location at the bottom yields a lower total system cost, effectively recognizing the move.4 This capability essentially solves the "block move" problem that plagues LCS-based diffs, allowing the engine to report "Function moved" rather than "Function deleted" and "New function added."

However, the $O(n^3)$ complexity is a double-edged sword. While polynomial, it is expensive. For a file with 1,000 lines, $1000^3$ is $10^9$ operations—manageable on modern CPUs. But for a file with 10,000 lines, $10^{12}$ operations pushes the runtime into minutes or hours, making it infeasible for real-time diffing in IDEs or pull requests without significant optimization or heuristics.6

## **3\. Beyond Munkres: The Jonker-Volgenant Algorithm (LAPJV)**

While the Hungarian algorithm provides the theoretical foundation, modern high-performance libraries rarely use Kuhn's original implementation. The standard for dense linear assignment problems is the algorithm proposed by Roy Jonker and Anton Volgenant in 1987, commonly referred to as LAPJV.9

### **3.1 Algorithmic Improvements over Munkres**

The Jonker-Volgenant algorithm is functionally equivalent to the Hungarian method in that it solves the exact linear assignment problem, but it employs sophisticated heuristics and structural changes that significantly improve practical performance, often by a factor of 10 or more.8 The core innovation of LAPJV lies in its initialization and augmentation phases.

Shortest Augmenting Path with Augmentation Heuristics:  
Like the optimized $O(n^3)$ Hungarian method, LAPJV uses a shortest augmenting path approach. However, it precedes the rigorous pathfinding phase with substantially more aggressive initialization heuristics.

1. **Column Reduction Phase**: It performs an initial scan to make cheap assignments greedily. This phase aims to solve the easy parts of the assignment problem instantly, reducing the number of nodes that require the expensive pathfinding logic.  
2. **Reduction Transfer**: It attempts to improve the dual variables (potentials) rapidly before engaging in the costly path augmentation. This helps in creating more zeros or low-cost edges in the reduced cost matrix.  
3. **Augmenting Row Reduction**: This is a specific phase that effectively combines the search for an augmenting path with row reduction operations. It is conceptually similar to "auction" phases where unassigned rows "bid" for columns, allowing for multiple assignments to be updated simultaneously rather than one by one.21

These heuristics do not change the worst-case complexity, which remains $O(n^3)$, but they dramatically lower the constant factors and improve the average-case behavior. In many real-world diffing scenarios, files have large sections of identical or nearly identical lines. LAPJV's initialization phases can match these lines almost instantly, leaving only the truly changed or moved lines for the heavier pathfinding logic.

### **3.2 Benchmarks and Real-World Performance**

Empirical benchmarks consistently show LAPJV outperforming standard Munkres implementations. In comparisons involving random cost matrices, lapjv implementations (such as those wrapped in Python or native C++) consistently clock significantly lower execution times than scipy.optimize.linear\_sum\_assignment (which historically used Munkres but has since moved to a modified JV or similar shortest-path implementation).22

For example, on a $1000 \\times 1000$ matrix, LAPJV might finish in milliseconds, whereas a naive Munkres implementation could take seconds. Benchmarks on random cost matrices of size $2048 \\times 2048$ show lapjv completing in fractions of a second, while pure Python implementations of Munkres struggle or time out.23 This performance gap becomes even more pronounced as $N$ increases.

It is crucial to note that while scipy.optimize.linear\_sum\_assignment has improved significantly in recent versions (SciPy 1.4+) by adopting a C++ implementation of the modified Jonker-Volgenant algorithm (specifically the rectangular variant), specialized libraries like lap (Python) or lapjv (Rust) often still hold a performance edge due to fewer overheads and more aggressive optimizations for dense integer matrices.24

### **3.3 Implementations in the Wild**

Practitioners looking to integrate this into a diff engine have several robust options:

* **Python**: The lap and lapjv packages wrap C++ implementations of the Jonker-Volgenant algorithm. scipy.optimize.linear\_sum\_assignment is the standard, accessible option, utilizing a modified shortest augmenting path algorithm that offers comparable performance to JV in modern versions.24  
* **Rust**: For systems requiring maximum performance and safety, the lapjv crate provides a native Rust implementation of the algorithm.20 This is particularly relevant for diff engines that might be compiled to WebAssembly (Wasm) for client-side execution in browsers, enabling high-performance diffing directly in web-based code review tools.  
* **Matlab**: The algorithm has a long history in the Matlab community, with efficient MEX implementations available for large-scale tracking problems.9

The dominance of JV in these benchmarks suggests that any "production-grade" diff engine aiming for optimal block move detection should prefer LAPJV (or the specialized sparse variant LAPJVsp) over a textbook implementation of the Hungarian algorithm.

## **4\. The Diffing Challenge: From Text Streams to Structural Moves**

To apply assignment algorithms effectively, one must bridge the gap between the raw data (text, spreadsheets, ASTs) and the mathematical abstraction of the cost matrix. This involves translating the semantic question "how similar are these two lines?" into a numerical cost.

### **4.1 The Limitations of LCS and Myers**

The standard diff algorithm, popularized by the Unix diff utility and Git, utilizes the Myers $O(ND)$ algorithm (or variations like Patience Diff or Histogram Diff).27 Myers' algorithm transforms the problem of finding the shortest edit script into finding the shortest path in an edit graph. It is optimized for the case where differences are small and sequential.29

However, Myers is fundamentally **local** and **sequential**. It assumes that the order of lines matters implicitly. If a paragraph is moved from the beginning of a document to the end, Myers will see this as deleting the paragraph at the top and inserting a new, identical paragraph at the bottom. It has no concept of "identity" for data blocks that transcends their position. This results in "noisy" diffs that obscure the user's intent—a simple refactor becomes a massive churn of deletions and additions.4 Furthermore, Myers is sensitive to "alignment noise." A single inserted line can misalign subsequent matches if not handled by heuristics like Patience Diff, which uses unique lines as anchors.

### **4.2 The Block Move Problem**

Detecting block moves is, in the general case, NP-hard. It involves finding a set of non-overlapping common substrings (blocks) and determining a mapping between them. This is where the assignment problem becomes a powerful heuristic tool.

By relaxing the strict sequential constraint of LCS, we can treat the diff problem as a bipartite matching problem. Each "item" (line, sentence, or code block) in file A is a node in partition $U$, and each item in file B is a node in partition $V$. The cost of an edge $(u, v)$ is defined by the dissimilarity between the content of $u$ and $v$. Solving this assignment problem allows the engine to "find" the moved block at its new location because the cost of matching identical (or nearly identical) blocks is practically zero, regardless of their positional offset.

### **4.3 Constructing the Cost Matrix**

The quality of the assignment—and thus the quality of the diff—depends entirely on the cost matrix. If the cost matrix fails to capture the true semantic similarity between lines, the Hungarian algorithm will produce a mathematically optimal but practically nonsensical matching.

Metrics for Similarity:  
The choice of similarity metric determines the "resolution" of the diff.

1. **Exact Match**: Cost is 0 if strings are identical, $\\infty$ (or a very large number) otherwise. This reduces the problem to finding the maximum number of identical lines that can be paired one-to-one. This is fast but brittle; a single character change (like a semicolon insertion) prevents the match.  
2. **Levenshtein / Edit Distance**: Cost is the number of character edits required to transform line $A$ into line $B$. This allows for "fuzzy" matching, detecting lines that were moved *and* slightly modified (e.g., a variable rename in a moved function). However, calculating Levenshtein distance for every pair of lines is computationally expensive ($O(L\_1 L\_2)$ per pair).32  
3. **Jaccard Similarity**: Useful for token-based comparison. Cost \= $1 \- J(A, B)$, where $J$ is the intersection over union of the set of tokens (words) in the lines.33 This is robust to word reordering but ignores structure.  
4. **Euclidean/Cosine Distance**: If lines are embedded into a vector space (using TF-IDF or dense embeddings like BERT), the cost can be the geometric distance between vectors. This captures semantic similarity even if wording changes completely, but computing high-dimensional embeddings for every line is resource-intensive.34

The Challenge of Dense Matrices:  
For a file with $N=10,000$ lines, a full cost matrix requires $10^8$ entries. If computing each entry involves a Levenshtein distance calculation, the preprocessing step alone becomes prohibitively slow. This necessitates sparse candidate generation, where we only compute costs for pairs that are "likely" to match (e.g., share at least one rare token or have high Jaccard similarity via MinHash).10 The Hungarian algorithm is then run only on this sparse graph, effectively pruning the $N^2$ search space.

## **5\. Optimizing for Scale: Sparse Matrices and Approximation**

The $O(n^3)$ barrier of the Hungarian/JV algorithms is the primary bottleneck for large-scale diffing. To build a high-quality engine that scales, one must abandon dense matrices in favor of sparse representations and approximation techniques.

### **5.1 Sparse Linear Assignment Solvers**

In many diffing scenarios, a line in File A has zero probability of matching 99% of the lines in File B. It is wasteful to store and process these high-cost edges. Algorithms like the **Jonker-Volgenant Sparse (LAPJVsp)** or solvers based on **Successive Shortest Path with Dijkstra** using potentials are designed exactly for this.

scipy.sparse.csgraph.min\_weight\_full\_bipartite\_matching implements such a sparse solver.37 By inputting a sparse matrix (CSR/CSC format) where only plausible matches (e.g., lines sharing rare trigrams) have entries, the solver can skip the vast majority of calculations. The complexity shifts closer to $O(k \\cdot n \\log n)$ or $O(m \+ n \\log n)$ depending on the number of edges $m$, which is significantly faster than $O(n^3)$ when the graph is sparse.38

### **5.2 The Auction Algorithm**

For extremely large or parallelized environments, **Auction Algorithms** (developed by Bertsekas) offer an alternative. They view the assignment problem as an economic market where persons (rows) bid for objects (columns). The prices of objects rise as they receive more bids, eventually reaching an equilibrium that corresponds to the optimal assignment.22

Auction algorithms are particularly attractive because they are naturally parallelizable and can be robust for sparse problems. Implementations in Rust (e.g., sparse\_linear\_assignment) utilizing forward/reverse auction mechanisms with $\\epsilon$-scaling can rival or outperform sequential JV implementations on very large, sparse graphs.40 However, they can suffer from convergence issues (infinite loops) if a perfect matching is not feasible, requiring careful handling of unassigned nodes and robust termination conditions.40 Unlike the Hungarian method, the auction algorithm does not strictly improve the primal objective at every step, but rather monotonically improves the dual objective, which allows it to escape local optima in a distributed fashion.

### **5.3 Heuristic Pruning and Blocking**

Instead of solving one giant $N \\times N$ assignment problem, practical engines often use "blocking" or "windowing" strategies to decompose the problem:

1. **Anchor Points**: Use LCS or Heckel's algorithm to find unique, identical lines that serve as "anchors." These lines are assumed to be correctly matched and are removed from the problem space. The file is then divided into smaller blocks of unmatched lines between these anchors.27  
2. **Divide and Conquer**: Solve the assignment problem only for the unaligned regions between anchors. If a 10,000-line file has 9,000 identical lines, the remaining 1,000 lines might form several small clusters (e.g., ten $100 \\times 100$ problems). Solving ten $100 \\times 100$ problems is vastly faster than solving one $1000 \\times 1000$ problem due to the cubic complexity.43  
3. **Windowing**: Only consider matching line $i$ in File A with lines in the range $$ in File B. This enforces a "locality" constraint, assuming that moves are usually local (e.g., swapping two adjacent functions). However, this defeats the purpose of detecting global moves (e.g., moving a class to the bottom of the file) and is generally discouraged for "move-aware" engines unless $W$ is very large.

## **6\. Architecture of a Block-Move Diff Engine**

Based on the research, a state-of-the-art diff engine should employ a hybrid architecture that layers different algorithms to balance speed and precision. A pure Hungarian implementation is too slow; a pure Myers implementation is too dumb. The optimal architecture is a pipeline.

### **6.1 Phase 1: Preprocessing and Hashing**

The engine begins by tokenizing the input files. Every line is hashed to allow for $O(1)$ comparisons.

* **Hash Map**: Construct a map of Hash \-\> \[Line Indices\] for both files.  
* **Heckel’s Pass**: Identify "unique" lines (lines that appear exactly once in both files). These are high-confidence anchors. Paul Heckel's algorithm is linear $O(N)$ and provides the skeleton of the matching.27 This step aligns the "easy" parts of the file instantly.

### **6.2 Phase 2: Block Detection and Matching**

Instead of diffing individual lines immediately, adjacent identical lines are grouped into "blocks." The problem is then elevated to matching blocks.

* If blocks are identical and anchored by unique lines, they are matched instantly.  
* For modified blocks (e.g., a function that was moved and had one line changed), we generate candidate matches using **MinHash LSH**.  
  * **MinHash**: Generates a compact signature for each block based on its constituent tokens. This signature allows for estimating Jaccard similarity without full pair-wise comparison.  
  * **LSH**: Buckets these signatures to find blocks that are "similar" (low Jaccard distance) without comparing every pair.10 This reduces the candidate set from $N^2$ to a manageable subset.

### **6.3 Phase 3: The Solver (Hungarian/JV)**

For the remaining unmatched blocks (or lines within matched blocks that differ), we construct a sparse cost matrix using the candidates from Phase 2\.

* **Nodes**: Unmatched lines from File A and File B.  
* **Edges**: Created only if the similarity score (Levenshtein/Cosine) exceeds a threshold.  
* **Solver**: Apply **LAPJV** (via a library like lapjv-rust or SciPy) to finding the optimal set of moves and edits.24 This step rigorously solves the assignment problem for the difficult cases that heuristics missed.  
* **Outcome**: This results in a set of "Moves" (assignments with large positional displacement), "Edits" (assignments with non-zero cost), and "Deletes/Inserts" (unassigned rows/cols).

### **6.4 Phase 4: Presentation**

The final step is presenting this to the user. Unlike standard diffs which just show Red/Green lines, this engine can render:

* "Moved from Line X to Line Y" indicators.  
* Intra-line diffing (using Myers on the character level) for matched lines that have slight edits.  
* Visual connectors for moved blocks, similar to tools like xltrail or specialized IDE plugins that help users trace code evolution across the file.47

## **7\. Specialized Domain: Spreadsheet and Tabular Diffing**

Diffing spreadsheets (Excel, CSV) introduces a dimension of complexity distinct from text: the 2D grid structure. Rows can be sorted, filtered, or inserted, and columns can be reordered. The "sequence" assumption of Myers is almost always invalid here.

### **7.1 The Row Alignment Problem**

In spreadsheets, "lines" are "rows." The order of rows often carries no semantic meaning (e.g., a list of employees sorted by ID vs sorted by Name). Therefore, sequential diff algorithms like Myers are often useless because a simple sort operation makes the entire file look "different".48

The Role of the Assignment Problem:  
Here, the Hungarian algorithm is not just an optimization; it is essential. We treat the spreadsheet diff as a bipartite matching of rows.

* **Key-based Matching**: If a "Primary Key" (e.g., Employee ID) exists, matching is trivial $O(N)$ (hash join). The engine should attempt to auto-detect keys by checking for columns with unique constraint violations.  
* **Content-based Matching**: Without keys, we must match rows based on content similarity. This is exactly the LAP. The cost $C\_{ij}$ is the distance between Row A and Row B.  
* **Solver**: For small sheets (\< 2000 rows), Hungarian/JV finds the optimal row alignment. For large sheets, heuristics are required.

### **7.2 Heuristics for Large Tabular Data**

For datasets with $100k+$ rows, $O(n^3)$ is impossible.

* **LSH for Rows**: Use MinHash on the row contents to bucket similar rows. Only run exact comparison within buckets.49  
* **Row Clustering**: Cluster rows by statistical properties (e.g., sum of numeric columns, presence of specific categorical values) to narrow the search space.50  
* **Daff / Coopy**: The daff library (and its Haxe/JS/Python implementations) uses a hybrid approach. It attempts to detect keys, align columns first, and then use heuristics to align rows, supporting moves and reorders without a full $N^2$ cost matrix.51

Xltrail and Spreadsheet Compare:  
Tools like xltrail and Microsoft's Spreadsheet Compare wrap these logic layers. They often rely on detecting a "key" column to anchor the diff. If no key is found, they may fall back to rigid line-by-line comparison or costly best-effort matching.53

## **8\. Structural Diffing: Trees and ASTs**

For code, "text" is just a serialization of a tree (Abstract Syntax Tree). Structural diffing engines like **GumTree** operate on the AST nodes rather than lines, representing the frontier of diff technology.

### **8.1 Top-Down vs Bottom-Up Matching**

GumTree employs a two-stage heuristic that mirrors the divide-and-conquer strategy, tailored for trees:

1. **Top-Down (Greedy)**: The algorithm searches for the largest isomorphic subtrees (e.g., a whole class or method that hasn't changed) using a height-priority approach. These matched subtrees serve as "anchors".42  
2. **Bottom-Up (Hungarian-like)**: For nodes that didn't match in the top-down phase, the algorithm matches them based on their descendants. If two function containers share a high percentage (e.g., \> 50%) of their children (statements), they are matched, even if the containers themselves (e.g., function names) changed. This allows for detecting Renames and Moves.

While GumTree doesn't strictly use the $O(n^3)$ Hungarian algorithm for the entire tree (which would be essentially the Tree Edit Distance problem, $O(N^3)$ or worse), it uses optimized mapping heuristics that approximate the assignment problem to keep runtime feasible for large codebases.55 The "bottom-up" phase effectively solves a localized assignment problem between the children of matched parents.

## **9\. Ecosystem and Implementation Recommendations**

For the practitioner building a diff engine today, the landscape offers several distinct paths depending on the language and performance requirements.

### **9.1 Rust: The Performance King**

Rust is the ideal language for this task due to its memory safety and performance, plus its ability to compile to Wasm for web-based diff viewers.

* **lapjv crate**: A direct port of the Jonker-Volgenant algorithm. It is fast, standard, and effective for dense matrices. This is the recommended crate for exact LAP solutions.9  
* **pathfinding crate**: Contains a kuhn\_munkres implementation. While excellent for general use and pedagogical clarity, benchmarks suggest specialized LAP solvers (like lapjv) may edge it out in raw speed for dense arithmetic matrices due to specific matrix optimizations.56  
* **sheets-diff**: A specialized crate for Excel files, though likely relies on simpler key-based or sequential heuristics rather than full semantic assignment for large files.58

### **9.2 Python: The Prototyping Powerhouse**

* **SciPy**: scipy.optimize.linear\_sum\_assignment is robust and uses a high-performance C++ backend. It is the gold standard for correctness and is widely available.59  
* **lap**: A specialized library often faster than SciPy for very dense integer matrices, wrapping C++ JV code. It is less maintained but highly performant.

### **9.3 The Verdict on "Practitioner-Focused"**

If building a generic text diff engine:

1. **Start with Myers** for basic display and compatibility.  
2. **Layer Heckel's Algorithm** to detect moved blocks quickly (linear time) and establish anchors.  
3. **Use Hungarian (LAPJV)** only on the *residue*—the blocks that were not matched by Myers or Heckel—to find granular sub-block moves or renames. This hybrid approach keeps the "average case" linear while providing "best case" optimality for complex refactors.

For a spreadsheet/data diff engine:

1. **Mandate Keys**: Always attempt to find a unique key column first.  
2. **Sparse LSH**: If no key exists, use MinHash to generate candidate pairs.  
3. **Sparse Assignment**: Feed candidates into a sparse solver (like SciPy's min\_weight\_full\_bipartite\_matching). This allows the engine to scale to millions of rows where a dense $N^3$ approach would fail.

## **10\. Conclusion**

The Hungarian algorithm, while historically significant as the first polynomial-time solution to the assignment problem, is rarely the end-all solution for modern diff engines in its raw form. Its $O(n^3)$ complexity is a hurdle for big data. However, the *assignment problem formulation* itself is the key to unlocking semantic diffs, move detection, and accurate tabular comparisons.

By adopting the **Jonker-Volgenant (LAPJV)** optimization for dense local problems, and leveraging **Sparse Solvers** combined with **Locality Sensitive Hashing (LSH)** for global matching, engineers can construct diff engines that feel "intelligent"—recognizing data movement and structural changes where traditional tools see only chaos. The future of diffing lies not in better string matching, but in smarter assignment optimization.

---

### **Tables and Comparisons**

| Algorithm | Complexity | Best Use Case | Diff Engine Application |
| :---- | :---- | :---- | :---- |
| **Hungarian (Munkres)** | $O(n^3)$ | General assignment, small $N$ | Theoretical baseline; small datasets. |
| **Jonker-Volgenant (LAPJV)** | $O(n^3)$ (Fast Avg) | Dense assignment, $N \< 5000$ | Optimal block move detection in code/text. |
| **Myers (LCS)** | $O(ND)$ | Sequential text, small edits | Standard git diff; character-level edits. |
| **Heckel** | $O(N)$ | Unique line matching | Initial "anchor" pass for move detection. |
| **Auction Algorithm** | Iterative | Parallel / Distributed | Massive datasets; sparse graph matching. |
| **GumTree** | $O(N^2)$ Heuristic | Hierarchical (AST) | Structural code diff (renames, wraps). |

### **Implementation Matrix**

| Language | Library | Algorithm | Notes |
| :---- | :---- | :---- | :---- |
| **Python** | scipy.optimize.linear\_sum\_assignment | Modified JV / Shortest Path | Robust, standard choice. |
| **Python** | lap | Jonker-Volgenant | Extremely fast for dense int matrices. |
| **Rust** | lapjv | Jonker-Volgenant | Native Rust, good for Wasm/Performance. |
| **Rust** | pathfinding::kuhn\_munkres | Hungarian | General purpose, generic types. |
| **C++** | Google OR-Tools | Min-Cost Max-Flow | Overkill for simple AP, but scales well. |

### **Equation Reference**

The Linear Assignment Problem objective function:

$$\\min\_{x} \\sum\_{i \\in U} \\sum\_{j \\in V} C\_{ij} x\_{ij}$$

Subject to:

$$\\sum\_{j \\in V} x\_{ij} \= 1 \\quad \\forall i \\in U$$

$$\\sum\_{i \\in U} x\_{ij} \= 1 \\quad \\forall j \\in V$$

$$x\_{ij} \\in \\{0, 1\\}$$  
This mathematical clarity allows the diff engine to formally prove that the matched "moved block" is indeed the best possible candidate for the move, minimizing the user's cognitive load during code review or data reconciliation.

#### **Works cited**

1. BDiff: Block-aware and Accurate Text-based Code Differencing \- arXiv, accessed November 26, 2025, [https://arxiv.org/html/2510.21094v1](https://arxiv.org/html/2510.21094v1)  
2. Myers diff algorithm vs Hunt–McIlroy algorithm \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/42635889/myers-diff-algorithm-vs-hunt-mcilroy-algorithm](https://stackoverflow.com/questions/42635889/myers-diff-algorithm-vs-hunt-mcilroy-algorithm)  
3. Exactly how the Hungarian Algorithm Works (Self-Driving Cars Example), accessed November 26, 2025, [https://www.thinkautonomous.ai/blog/hungarian-algorithm/](https://www.thinkautonomous.ai/blog/hungarian-algorithm/)  
4. (PDF) BDiff: Block-aware and Accurate Text-based Code Differencing \- ResearchGate, accessed November 26, 2025, [https://www.researchgate.net/publication/396924404\_BDiff\_Block-aware\_and\_Accurate\_Text-based\_Code\_Differencing](https://www.researchgate.net/publication/396924404_BDiff_Block-aware_and_Accurate_Text-based_Code_Differencing)  
5. Hungarian algorithm for solving the assignment problem, accessed November 26, 2025, [https://cp-algorithms.com/graph/hungarian-algorithm.html](https://cp-algorithms.com/graph/hungarian-algorithm.html)  
6. Hungarian algorithm \- Wikipedia, accessed November 26, 2025, [https://en.wikipedia.org/wiki/Hungarian\_algorithm](https://en.wikipedia.org/wiki/Hungarian_algorithm)  
7. The Hungarian Algorithm and Its Applications in Computer Vision | Towards Data Science, accessed November 26, 2025, [https://towardsdatascience.com/hungarian-algorithm-and-its-applications-in-computer-vision/](https://towardsdatascience.com/hungarian-algorithm-and-its-applications-in-computer-vision/)  
8. accessed November 26, 2025, [https://www.mathworks.com/matlabcentral/fileexchange/26836-lapjv-jonker-volgenant-algorithm-for-linear-assignment-problem-v3-0\#:\~:text=The%20Jonker%2DVolgenant%20algorithm%20is,than%20the%20munkres%20code%20(v2.](https://www.mathworks.com/matlabcentral/fileexchange/26836-lapjv-jonker-volgenant-algorithm-for-linear-assignment-problem-v3-0#:~:text=The%20Jonker%2DVolgenant%20algorithm%20is,than%20the%20munkres%20code%20\(v2.)  
9. LAPJV \- Jonker-Volgenant Algorithm for Linear Assignment Problem V3.0 \- File Exchange, accessed November 26, 2025, [https://www.mathworks.com/matlabcentral/fileexchange/26836-lapjv-jonker-volgenant-algorithm-for-linear-assignment-problem-v3-0](https://www.mathworks.com/matlabcentral/fileexchange/26836-lapjv-jonker-volgenant-algorithm-for-linear-assignment-problem-v3-0)  
10. Locality Sensitive Hashing (LSH): The Illustrated Guide \- Pinecone, accessed November 26, 2025, [https://www.pinecone.io/learn/series/faiss/locality-sensitive-hashing/](https://www.pinecone.io/learn/series/faiss/locality-sensitive-hashing/)  
11. Hungarian Algorithm for non square matrix \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/51223398/hungarian-algorithm-for-non-square-matrix](https://stackoverflow.com/questions/51223398/hungarian-algorithm-for-non-square-matrix)  
12. Assignment problem \- Wikipedia, accessed November 26, 2025, [https://en.wikipedia.org/wiki/Assignment\_problem](https://en.wikipedia.org/wiki/Assignment_problem)  
13. The Dynamic Hungarian Algorithm for the Assignment Problem with Changing Costs \- KiltHub @ CMU, accessed November 26, 2025, [https://kilthub.cmu.edu/articles/The\_Dynamic\_Hungarian\_Algorithm\_for\_the\_Assignment\_Problem\_with\_Changing\_Costs/6561212/files/12043517.pdf](https://kilthub.cmu.edu/articles/The_Dynamic_Hungarian_Algorithm_for_the_Assignment_Problem_with_Changing_Costs/6561212/files/12043517.pdf)  
14. What is minimum cost perfect matching problem for general graph?, accessed November 26, 2025, [https://cs.stackexchange.com/questions/115532/what-is-minimum-cost-perfect-matching-problem-for-general-graph](https://cs.stackexchange.com/questions/115532/what-is-minimum-cost-perfect-matching-problem-for-general-graph)  
15. Algorithms for the Assignment and Transportation Problems \- James Munkres \- UC Davis Mathematics, accessed November 26, 2025, [https://www.math.ucdavis.edu/\~saito/data/emd/munkres.pdf](https://www.math.ucdavis.edu/~saito/data/emd/munkres.pdf)  
16. munkres \- Duke Computer Science, accessed November 26, 2025, [https://users.cs.duke.edu/\~brd/Teaching/Bio/asmb/current/Handouts/munkres.html](https://users.cs.duke.edu/~brd/Teaching/Bio/asmb/current/Handouts/munkres.html)  
17. Solving assignment problem using Hungarian method vs min cost max flow problem, accessed November 26, 2025, [https://mathoverflow.net/questions/185632/solving-assignment-problem-using-hungarian-method-vs-min-cost-max-flow-problem](https://mathoverflow.net/questions/185632/solving-assignment-problem-using-hungarian-method-vs-min-cost-max-flow-problem)  
18. munkres \- crates.io: Rust Package Registry, accessed November 26, 2025, [https://crates.io/crates/munkres](https://crates.io/crates/munkres)  
19. Is it still efficient to use the Hungarian algorithm for large values of n? \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/52872539/is-it-still-efficient-to-use-the-hungarian-algorithm-for-large-values-of-n](https://stackoverflow.com/questions/52872539/is-it-still-efficient-to-use-the-hungarian-algorithm-for-large-values-of-n)  
20. lapjv \- Rust \- Docs.rs, accessed November 26, 2025, [https://docs.rs/lapjv](https://docs.rs/lapjv)  
21. src-d/lapjv: Linear Assignmment Problem solver using ... \- GitHub, accessed November 26, 2025, [https://github.com/src-d/lapjv](https://github.com/src-d/lapjv)  
22. Performance comparison of 2D assignment algorithms for assigning truth objects to measured tracks \- ResearchGate, accessed November 26, 2025, [https://www.researchgate.net/publication/252500299\_Performance\_comparison\_of\_2D\_assignment\_algorithms\_for\_assigning\_truth\_objects\_to\_measured\_tracks](https://www.researchgate.net/publication/252500299_Performance_comparison_of_2D_assignment_algorithms_for_assigning_truth_objects_to_measured_tracks)  
23. berhane/LAP-solvers: Benchmarking Linear Assignment Problem Solvers \- GitHub, accessed November 26, 2025, [https://github.com/berhane/LAP-solvers](https://github.com/berhane/LAP-solvers)  
24. The Assignment Problem, a NumPy function? \- python \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/1398822/the-assignment-problem-a-numpy-function](https://stackoverflow.com/questions/1398822/the-assignment-problem-a-numpy-function)  
25. Maximum Weight / Minimum Cost Bipartite Matching Code in Python \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/4426131/maximum-weight-minimum-cost-bipartite-matching-code-in-python](https://stackoverflow.com/questions/4426131/maximum-weight-minimum-cost-bipartite-matching-code-in-python)  
26. Antti/lapjv-rust: Linear Assignment Problem solver using Jonker-Volgenant algorithm, accessed November 26, 2025, [https://github.com/Antti/lapjv-rust](https://github.com/Antti/lapjv-rust)  
27. Paul Heckel's Diff Algorithm \- GitHub Gist, accessed November 26, 2025, [https://gist.github.com/ndarville/3166060](https://gist.github.com/ndarville/3166060)  
28. Tweaked implementation of Paul Heckel's diff algorithm \- GitHub, accessed November 26, 2025, [https://github.com/myndzi/heckel-diff](https://github.com/myndzi/heckel-diff)  
29. An O(ND) difference algorithm and its variations. | Janelia Research Campus, accessed November 26, 2025, [https://www.janelia.org/publication/ond-difference-algorithm-and-its-variations](https://www.janelia.org/publication/ond-difference-algorithm-and-its-variations)  
30. The Myers Difference Algorithm \- Nathaniel W. \- Wroblewski, accessed November 26, 2025, [https://www.nathaniel.ai/myers-diff/](https://www.nathaniel.ai/myers-diff/)  
31. Visualizing Diffs: The Myers difference algorithm (2020) | Hacker News, accessed November 26, 2025, [https://news.ycombinator.com/item?id=33417466](https://news.ycombinator.com/item?id=33417466)  
32. Seeking algo for text diff that detects and can group similar lines \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/2231488/seeking-algo-for-text-diff-that-detects-and-can-group-similar-lines](https://stackoverflow.com/questions/2231488/seeking-algo-for-text-diff-that-detects-and-can-group-similar-lines)  
33. MinHash \- Fast Jaccard Similarity at Scale \- Arpit Bhayani, accessed November 26, 2025, [https://arpitbhayani.me/blogs/jaccard-minhash/](https://arpitbhayani.me/blogs/jaccard-minhash/)  
34. Similarity Metrics for Vector Search \- Zilliz blog, accessed November 26, 2025, [https://zilliz.com/blog/similarity-metrics-for-vector-search](https://zilliz.com/blog/similarity-metrics-for-vector-search)  
35. euclidean distance and similarity \- excel \- Stack Overflow, accessed November 26, 2025, [https://stackoverflow.com/questions/47164817/euclidean-distance-and-similarity](https://stackoverflow.com/questions/47164817/euclidean-distance-and-similarity)  
36. linear\_assignment | OR-Tools \- Google for Developers, accessed November 26, 2025, [https://developers.google.com/optimization/reference/graph/linear\_assignment](https://developers.google.com/optimization/reference/graph/linear_assignment)  
37. min\_weight\_full\_bipartite\_matchi, accessed November 26, 2025, [https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.csgraph.min\_weight\_full\_bipartite\_matching.html](https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.csgraph.min_weight_full_bipartite_matching.html)  
38. Linear Assignment Problems and Extensions ∗ \- TU Graz, accessed November 26, 2025, [https://www.opt.math.tugraz.at/\~cela/papers/lap\_bericht.pdf](https://www.opt.math.tugraz.at/~cela/papers/lap_bericht.pdf)  
39. \[2310.03159\] New Auction Algorithms for the Assignment Problem and Extensions \- arXiv, accessed November 26, 2025, [https://arxiv.org/abs/2310.03159](https://arxiv.org/abs/2310.03159)  
40. sparse\_linear\_assignment \- Rust \- Docs.rs, accessed November 26, 2025, [https://docs.rs/sparse\_linear\_assignment](https://docs.rs/sparse_linear_assignment)  
41. sparse\_linear\_assignment \- crates.io: Rust Package Registry, accessed November 26, 2025, [https://crates.io/crates/sparse\_linear\_assignment](https://crates.io/crates/sparse_linear_assignment)  
42. Fine-grained and Accurate Source Code Differencing: GumTree \- Computer Science (CS), accessed November 26, 2025, [https://courses.cs.vt.edu/cs6704/spring17/slides\_by\_students/CS6704\_gumtree\_Kijin\_AN\_Feb15.pdf](https://courses.cs.vt.edu/cs6704/spring17/slides_by_students/CS6704_gumtree_Kijin_AN_Feb15.pdf)  
43. An O(ND) Difference Algorithm and Its Variations ∗ \- XMail, accessed November 26, 2025, [http://www.xmailserver.org/diff2.pdf](http://www.xmailserver.org/diff2.pdf)  
44. Myers Diff Algorithm \- Code & Interactive Visualization \- Robert Elder, accessed November 26, 2025, [https://blog.robertelder.org/diff-algorithm/](https://blog.robertelder.org/diff-algorithm/)  
45. A technique for isolating differences between files \- Communications of the ACM, accessed November 26, 2025, [https://cacm.acm.org/research/a-technique-for-isolating-differences-between-files/](https://cacm.acm.org/research/a-technique-for-isolating-differences-between-files/)  
46. MinHash Tutorial with Python Code \- Chris McCormick, accessed November 26, 2025, [https://mccormickml.com/2015/06/12/minhash-tutorial-with-python-code/](https://mccormickml.com/2015/06/12/minhash-tutorial-with-python-code/)  
47. xltrail \- Version Control for Excel Spreadsheets \- xltrail is a version control system for Excel workbooks. It tracks changes, compares worksheets and VBA, and provides an audit trail for easy collaboration., accessed November 26, 2025, [https://www.xltrail.com/](https://www.xltrail.com/)  
48. Microsoft Excel \- Moving data within multiple staggered rows, into single row, accessed November 26, 2025, [https://superuser.com/questions/1709220/microsoft-excel-moving-data-within-multiple-staggered-rows-into-single-row](https://superuser.com/questions/1709220/microsoft-excel-moving-data-within-multiple-staggered-rows-into-single-row)  
49. LocalitySensitiveHashing-1.0.1.html \- Purdue College of Engineering, accessed November 26, 2025, [https://engineering.purdue.edu/kak/distLSH/LocalitySensitiveHashing-1.0.1.html/signal.html](https://engineering.purdue.edu/kak/distLSH/LocalitySensitiveHashing-1.0.1.html/signal.html)  
50. Find Clusters in Data \- Tableau Help, accessed November 26, 2025, [https://help.tableau.com/current/pro/desktop/en-us/clustering.htm](https://help.tableau.com/current/pro/desktop/en-us/clustering.htm)  
51. edwindj/daff: Diff, patch and merge for data.frames, see http://paulfitz.github.io/daff \- GitHub, accessed November 26, 2025, [https://github.com/edwindj/daff](https://github.com/edwindj/daff)  
52. Diffing and patching tabular data \- Open Knowledge Labs, accessed November 26, 2025, [https://okfnlabs.org/blog/2013/08/08/diffing-and-patching-data.html](https://okfnlabs.org/blog/2013/08/08/diffing-and-patching-data.html)  
53. Version Control for Excel Spreadsheets \- 3 steps to make ... \- xltrail, accessed November 26, 2025, [https://www.xltrail.com/blog/git-diff-spreadsheetcompare](https://www.xltrail.com/blog/git-diff-spreadsheetcompare)  
54. Generating Accurate and Compact Edit Scripts using Tree Differencing | Xifiggam.eu, accessed November 26, 2025, [https://www.xifiggam.eu/wp-content/uploads/2018/08/GeneratingAccurateandCompactEditScriptsusingTreeDifferencing.pdf](https://www.xifiggam.eu/wp-content/uploads/2018/08/GeneratingAccurateandCompactEditScriptsusingTreeDifferencing.pdf)  
55. Efficiently Optimizing Hyperparameters for the Gumtree Hybrid Code Differencing Algorithm within HyperAST \- TU Delft Research Portal, accessed November 26, 2025, [https://pure.tudelft.nl/admin/files/247344311/Scalable\_Structural\_Code\_Diffs\_Gumtree\_Hybrid\_v2-8-1.pdf](https://pure.tudelft.nl/admin/files/247344311/Scalable_Structural_Code_Diffs_Gumtree_Hybrid_v2-8-1.pdf)  
56. kuhn\_munkres in pathfinding \- Docs.rs, accessed November 26, 2025, [https://docs.rs/pathfinding/latest/pathfinding/kuhn\_munkres/fn.kuhn\_munkres.html](https://docs.rs/pathfinding/latest/pathfinding/kuhn_munkres/fn.kuhn_munkres.html)  
57. Simplified Rust implementation of the Hungarian (or Kuhn–Munkres) algorithm \- GitHub, accessed November 26, 2025, [https://github.com/nwtnni/hungarian](https://github.com/nwtnni/hungarian)  
58. sheets-diff \- crates.io: Rust Package Registry, accessed November 26, 2025, [https://crates.io/crates/sheets-diff](https://crates.io/crates/sheets-diff)  
59. linear\_sum\_assignment — SciPy v1.16.2 Manual, accessed November 26, 2025, [https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.linear\_sum\_assignment.html](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.linear_sum_assignment.html)