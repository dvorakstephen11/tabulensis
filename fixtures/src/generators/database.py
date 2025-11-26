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

