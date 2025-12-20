import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
try:
    from .generators.corrupt import ContainerCorruptGenerator
    from .generators.database import KeyedTableGenerator
    from .generators.grid import (
        AddressSanityGenerator,
        BasicGridGenerator,
        ColumnAlignmentG9Generator,
        ColumnMoveG12Generator,
        EdgeCaseGenerator,
        GridTailDiffGenerator,
        MultiCellDiffGenerator,
        Pg6SheetScenarioGenerator,
        RectBlockMoveG12Generator,
        RowAlignmentG10Generator,
        RowAlignmentG8Generator,
        RowBlockMoveG11Generator,
        RowFuzzyMoveG13Generator,
        SheetCaseRenameGenerator,
        SingleCellDiffGenerator,
        SparseGridGenerator,
        ValueFormulaGenerator,
    )
    from .generators.mashup import (
        MashupCorruptGenerator,
        MashupDuplicateGenerator,
        MashupEncodeGenerator,
        MashupInjectGenerator,
        MashupMultiEmbeddedGenerator,
        MashupOneQueryGenerator,
        MashupPermissionsMetadataGenerator,
    )
    from .generators.objects import ChartsGenerator, CopyTemplateGenerator, NamedRangesGenerator
    from .generators.perf import LargeGridGenerator
except ImportError:
    from generators.corrupt import ContainerCorruptGenerator
    from generators.database import KeyedTableGenerator
    from generators.grid import (
        AddressSanityGenerator,
        BasicGridGenerator,
        ColumnAlignmentG9Generator,
        ColumnMoveG12Generator,
        EdgeCaseGenerator,
        GridTailDiffGenerator,
        MultiCellDiffGenerator,
        Pg6SheetScenarioGenerator,
        RectBlockMoveG12Generator,
        RowAlignmentG10Generator,
        RowAlignmentG8Generator,
        RowBlockMoveG11Generator,
        RowFuzzyMoveG13Generator,
        SheetCaseRenameGenerator,
        SingleCellDiffGenerator,
        SparseGridGenerator,
        ValueFormulaGenerator,
    )
    from generators.mashup import (
        MashupCorruptGenerator,
        MashupDuplicateGenerator,
        MashupEncodeGenerator,
        MashupInjectGenerator,
        MashupMultiEmbeddedGenerator,
        MashupOneQueryGenerator,
        MashupPermissionsMetadataGenerator,
    )
    from generators.objects import ChartsGenerator, CopyTemplateGenerator, NamedRangesGenerator
    from generators.perf import LargeGridGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "multi_cell_diff": MultiCellDiffGenerator,
    "grid_tail_diff": GridTailDiffGenerator,
    "row_alignment_g8": RowAlignmentG8Generator,
    "row_alignment_g10": RowAlignmentG10Generator,
    "row_block_move_g11": RowBlockMoveG11Generator,
    "row_fuzzy_move_g13": RowFuzzyMoveG13Generator,
    "column_move_g12": ColumnMoveG12Generator,
    "rect_block_move_g12": RectBlockMoveG12Generator,
    "column_alignment_g9": ColumnAlignmentG9Generator,
    "sheet_case_rename": SheetCaseRenameGenerator,
    "pg6_sheet_scenario": Pg6SheetScenarioGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "mashup:one_query": MashupOneQueryGenerator,
    "mashup:multi_query_with_embedded": MashupMultiEmbeddedGenerator,
    "mashup:permissions_metadata": MashupPermissionsMetadataGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
    "named_ranges": NamedRangesGenerator,
    "charts": ChartsGenerator,
    "copy_template": CopyTemplateGenerator,
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
