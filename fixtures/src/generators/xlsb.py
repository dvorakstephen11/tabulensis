import zipfile
from pathlib import Path
from typing import List, Union

from .base import BaseGenerator


class XlsbStubGenerator(BaseGenerator):
    """
    Create a minimal OPC container with xl/workbook.bin to exercise XLSB detection.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._write_stub(target_path)

    def _write_stub(self, path: Path):
        content_types = (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="bin" '
            'ContentType="application/vnd.ms-excel.sheet.binary.macroEnabled.main" />'
            "</Types>"
        )
        rels = (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            "</Relationships>"
        )

        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zout:
            zout.writestr("[Content_Types].xml", content_types)
            zout.writestr("_rels/.rels", rels)
            zout.writestr("xl/workbook.bin", b"XLSB-STUB")
