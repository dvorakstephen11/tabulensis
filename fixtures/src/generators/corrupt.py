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
            elif mode == 'not_zip_text':
                out_path.write_text("This is not a zip container", encoding="utf-8")
            else:
                raise ValueError(f"Unsupported corrupt_container mode: {mode}")

