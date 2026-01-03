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
