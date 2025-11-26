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

