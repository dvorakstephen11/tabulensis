import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2/P3/P4/P5 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)
        pattern_length = self.args.get('pattern_length', 100)
        fill_percent = self.args.get('fill_percent', 100)
        edit_row = self.args.get('edit_row')
        edit_col = self.args.get('edit_col')
        edit_value = self.args.get('edit_value')
        if edit_row is not None:
            edit_row = int(edit_row)
        if edit_col is not None:
            edit_col = int(edit_col)

        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    row_data = [rng.random() for _ in range(cols)]
                
                elif mode == 'repetitive':
                    pattern_idx = (r - 1) % pattern_length
                    row_data = [f"P{pattern_idx}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'sparse':
                    row_data = []
                    for c in range(1, cols + 1):
                        if rng.randint(1, 100) <= fill_percent:
                            row_data.append(f"R{r}C{c}")
                        else:
                            row_data.append(None)
                if edit_row is not None and edit_col is not None and r == edit_row:
                    idx = edit_col - 1
                    if 0 <= idx < cols:
                        row_data[idx] = edit_value

                ws.append(row_data)

            wb.save(output_dir / name)

