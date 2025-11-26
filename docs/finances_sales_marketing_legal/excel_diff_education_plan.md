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