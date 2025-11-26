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
fixtures/generated/*.csv
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

