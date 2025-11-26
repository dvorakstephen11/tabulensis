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

