import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    
    Supports:
    - extra_rows: Add new rows with specified id/name/amount/category
    - updates: Modify existing rows by id (e.g., [{ id: 7, amount: 120 }])
    - shuffle: Randomize row order
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])
        updates = self.args.get('updates', [])

        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            for row in extra_rows:
                data_rows.append(row)

            updates_by_id = {u['id']: u for u in updates}
            for row in data_rows:
                if row['id'] in updates_by_id:
                    upd = updates_by_id[row['id']]
                    for key in ['name', 'amount', 'category']:
                        if key in upd:
                            row[key] = upd[key]

            if shuffle:
                rng.shuffle(data_rows)

            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)

