import base64
import zipfile
from pathlib import Path

from lxml import etree

from .base import BaseGenerator


_NS = {"dm": "http://schemas.microsoft.com/DataMashup"}


def _find_datamashup_element(root):
    if root is None:
        return None
    if root.tag.endswith("DataMashup"):
        return root
    return root.find(".//dm:DataMashup", namespaces=_NS)


def _extract_datamashup_bytes_from_xlsx(path: Path) -> bytes:
    with zipfile.ZipFile(path, "r") as zin:
        for info in zin.infolist():
            name = info.filename
            if not (name.startswith("customXml/item") and name.endswith(".xml")):
                continue

            buf = zin.read(name)
            if (
                b"DataMashup" not in buf
                and b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" not in buf
            ):
                continue

            root = etree.fromstring(buf)
            node = _find_datamashup_element(root)
            if node is None or node.text is None:
                continue

            text = node.text.strip()
            if not text:
                continue

            return base64.b64decode(text)

    raise ValueError("DataMashup not found in xlsx")


class PbixGenerator(BaseGenerator):
    def generate(self, out_dir: Path, outputs):
        if isinstance(outputs, str):
            outputs = [outputs]

        if len(outputs) != 1:
            raise ValueError("pbix generator expects exactly one output filename")

        out_path = out_dir / outputs[0]

        mode = self.args.get("mode", "from_xlsx")
        base_file = self.args.get("base_file")

        if mode not in ("from_xlsx", "no_datamashup", "no_schema"):
            raise ValueError(f"Unsupported pbix generator mode: {mode}")

        include_datamashup = mode == "from_xlsx"
        include_markers = True
        include_schema = mode != "no_schema"

        schema_bytes = None
        model_schema = self.args.get("model_schema")
        model_schema_file = self.args.get("model_schema_file")
        if model_schema_file:
            model_schema_path = Path(model_schema_file)
            if not model_schema_path.exists():
                model_schema_path = Path("fixtures") / model_schema_file
            schema_bytes = model_schema_path.read_bytes()
        elif model_schema is not None:
            schema_bytes = str(model_schema).encode("utf-8")

        dm_bytes = b""
        if include_datamashup:
            if not base_file:
                raise ValueError("base_file is required for mode=from_xlsx")
            base_path = Path(base_file)
            if not base_path.exists():
                base_path = Path("fixtures") / base_file
            dm_bytes = _extract_datamashup_bytes_from_xlsx(base_path)

        with zipfile.ZipFile(out_path, "w", compression=zipfile.ZIP_DEFLATED) as zout:
            if include_datamashup:
                zout.writestr("DataMashup", dm_bytes)
            if include_markers:
                zout.writestr("Report/Layout", b"{}")
                zout.writestr("Report/Version", b"1")
                if include_schema:
                    zout.writestr("DataModelSchema", schema_bytes or b"{}")
