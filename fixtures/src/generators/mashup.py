import base64
import copy
import io
import random
import re
import struct
import zipfile
from pathlib import Path
from typing import Callable, List, Optional, Union
from xml.etree import ElementTree as ET
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        if tree.tag.endswith("DataMashup"):
            return tree
        return tree.find('.//dm:DataMashup', namespaces=NS)

    def _process_excel_container(
        self,
        base_path,
        output_path,
        callback,
        text_mutator: Optional[Callable[[str], str]] = None,
    ):
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
                    has_marker = b"DataMashup" in buffer or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in buffer
                    if item.filename.startswith("customXml/item") and has_marker:
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
                                new_text = base64.b64encode(new_bytes).decode('utf-8')
                                if text_mutator is not None:
                                    new_text = text_mutator(new_text)
                                dm_node.text = new_text
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
            text_mutator = self._garble_base64_text if mode == 'byte_flip' else None
            self._process_excel_container(
                base.resolve(),
                target_path,
                corruptor,
                text_mutator=text_mutator,
            )

    def _garble_base64_text(self, encoded: str) -> str:
        if not encoded:
            return "!!"
        chars = list(encoded)
        chars[0] = "!"
        return "".join(chars)


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


class MashupPackagePartsGenerator(MashupBaseGenerator):
    """
    Generates PackageParts-focused fixtures starting from a base workbook.
    """

    variant: str = "one_query"

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        if self.variant == "one_query":
            return raw_bytes

        version, package_parts, permissions, metadata, bindings = self._split_sections(raw_bytes)
        package_xml, main_section_text, content_types = self._extract_package_parts(package_parts)

        embedded_guid = self.args.get(
            "embedded_guid", "{11111111-2222-3333-4444-555555555555}"
        )
        embedded_section_text = self.args.get(
            "embedded_section",
            self._default_embedded_section(),
        )
        updated_main_section = self._extend_main_section(main_section_text, embedded_guid)
        embedded_bytes = self._build_embedded_package(embedded_section_text, content_types)
        updated_package_parts = self._build_package_parts(
            package_xml,
            updated_main_section,
            content_types,
            embedded_guid,
            embedded_bytes,
        )

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions,
            metadata,
            bindings,
        )

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts_end = offset + package_parts_len
        if package_parts_end > len(raw_bytes):
            raise ValueError("invalid PackageParts length")
        package_parts = raw_bytes[offset:package_parts_end]
        offset = package_parts_end

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions_end = offset + permissions_len
        if permissions_end > len(raw_bytes):
            raise ValueError("invalid permissions length")
        permissions = raw_bytes[offset:permissions_end]
        offset = permissions_end

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata_end = offset + metadata_len
        if metadata_end > len(raw_bytes):
            raise ValueError("invalid metadata length")
        metadata = raw_bytes[offset:metadata_end]
        offset = metadata_end

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings_end = offset + bindings_len
        if bindings_end > len(raw_bytes):
            raise ValueError("invalid bindings length")
        bindings = raw_bytes[offset:bindings_end]
        offset = bindings_end

        if offset != len(raw_bytes):
            raise ValueError("DataMashup trailing bytes mismatch")

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _extract_package_parts(self, package_parts: bytes):
        with zipfile.ZipFile(io.BytesIO(package_parts), "r") as z:
            package_xml = z.read("Config/Package.xml")
            content_types = z.read("[Content_Types].xml")
            main_section = z.read("Formulas/Section1.m")
        return package_xml, main_section.decode("utf-8", errors="ignore"), content_types

    def _extend_main_section(self, base_section: str, embedded_guid: str) -> str:
        stripped = base_section.rstrip()
        lines = [
            stripped,
            "",
            "shared EmbeddedQuery = let",
            f'    Source = Embedded.Value("Content/{embedded_guid}.package")',
            "in",
            "    Source;",
        ]
        return "\n".join(lines)

    def _build_embedded_package(self, section_text: str, content_types_template: bytes) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Formulas/Section1.m", section_text)
        return buffer.getvalue()

    def _build_package_parts(
        self,
        package_xml: bytes,
        main_section: str,
        content_types_template: bytes,
        embedded_guid: str,
        embedded_package: bytes,
    ) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Config/Package.xml", package_xml)
            z.writestr("Formulas/Section1.m", main_section)
            z.writestr(f"Content/{embedded_guid}.package", embedded_package)
        return buffer.getvalue()

    def _augment_content_types(self, content_types_bytes: bytes) -> str:
        text = content_types_bytes.decode("utf-8", errors="ignore")
        if "Extension=\"package\"" not in text and "Extension='package'" not in text:
            text = text.replace(
                "</Types>",
                '<Default Extension="package" ContentType="application/octet-stream" /></Types>',
                1,
            )
        return text

    def _default_embedded_section(self) -> str:
        return "\n".join(
            [
                "section Section1;",
                "",
                "shared Inner = let",
                "    Source = 1",
                "in",
                "    Source;",
            ]
        )


class MashupOneQueryGenerator(MashupPackagePartsGenerator):
    variant = "one_query"


class MashupMultiEmbeddedGenerator(MashupPackagePartsGenerator):
    variant = "multi_query_with_embedded"


class MashupDuplicateGenerator(MashupBaseGenerator):
    """
    Duplicates the customXml part that contains DataMashup to produce two
    DataMashup occurrences in a single workbook.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        mode = self.args.get('mode', 'part')
        if not base_file_arg:
            raise ValueError("MashupDuplicateGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            if mode == 'part':
                self._duplicate_datamashup_part(base.resolve(), target_path)
            elif mode == 'element':
                self._duplicate_datamashup_element(base.resolve(), target_path)
            else:
                raise ValueError(f"Unsupported duplicate mode: {mode}")

    def _duplicate_datamashup_part(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            try:
                item1_xml = zin.read("customXml/item1.xml")
                item_props1 = zin.read("customXml/itemProps1.xml")
                item1_rels = zin.read("customXml/_rels/item1.xml.rels")
                content_types = zin.read("[Content_Types].xml")
                workbook_rels = zin.read("xl/_rels/workbook.xml.rels")
            except KeyError as e:
                raise FileNotFoundError(f"Required DataMashup part missing: {e}") from e

            updated_content_types = self._add_itemprops_override(content_types)
            updated_workbook_rels = self._add_workbook_relationship(workbook_rels)
            item2_rels = item1_rels.replace(b"itemProps1.xml", b"itemProps2.xml")
            item_props2 = item_props1.replace(
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BE}",
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BF}",
            )

            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename == "[Content_Types].xml":
                        data = updated_content_types
                    elif info.filename == "xl/_rels/workbook.xml.rels":
                        data = updated_workbook_rels
                    zout.writestr(info, data)

                zout.writestr("customXml/item2.xml", item1_xml)
                zout.writestr("customXml/itemProps2.xml", item_props2)
                zout.writestr("customXml/_rels/item2.xml.rels", item2_rels)

    def _add_itemprops_override(self, content_types_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/content-types"
        root = ET.fromstring(content_types_bytes)
        override_tag = f"{{{ns}}}Override"
        if not any(
            elem.get("PartName") == "/customXml/itemProps2.xml"
            for elem in root.findall(override_tag)
        ):
            new_override = ET.SubElement(root, override_tag)
            new_override.set("PartName", "/customXml/itemProps2.xml")
            new_override.set(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
            )
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _add_workbook_relationship(self, rels_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/relationships"
        root = ET.fromstring(rels_bytes)
        rel_tag = f"{{{ns}}}Relationship"
        existing_ids = {elem.get("Id") for elem in root.findall(rel_tag)}
        next_id = 1
        while f"rId{next_id}" in existing_ids:
            next_id += 1
        new_rel = ET.SubElement(root, rel_tag)
        new_rel.set("Id", f"rId{next_id}")
        new_rel.set(
            "Type",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        )
        new_rel.set("Target", "../customXml/item2.xml")
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _duplicate_datamashup_element(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            root = etree.fromstring(data)
                            dm_node = self._get_mashup_element(root)
                            if dm_node is not None:
                                duplicate = copy.deepcopy(dm_node)
                                parent = dm_node.getparent()
                                if parent is not None:
                                    parent.append(duplicate)
                                    target_root = root
                                else:
                                    container = etree.Element("root", nsmap=root.nsmap)
                                    container.append(dm_node)
                                    container.append(duplicate)
                                    target_root = container
                                data = etree.tostring(
                                    target_root, encoding="utf-8", xml_declaration=True
                                )
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)


class MashupEncodeGenerator(MashupBaseGenerator):
    """
    Re-encodes the DataMashup customXml stream to a target encoding and optionally
    inserts whitespace into the base64 payload.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        encoding = self.args.get('encoding', 'utf-8')
        whitespace = bool(self.args.get('whitespace', False))
        if not base_file_arg:
            raise ValueError("MashupEncodeGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._rewrite_datamashup_xml(base.resolve(), target_path, encoding, whitespace)

    def _rewrite_datamashup_xml(
        self,
        base_path: Path,
        output_path: Path,
        encoding: str,
        whitespace: bool,
    ):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            data = self._process_datamashup_stream(data, encoding, whitespace)
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)

    def _process_datamashup_stream(
        self,
        xml_bytes: bytes,
        encoding: str,
        whitespace: bool,
    ) -> bytes:
        root = etree.fromstring(xml_bytes)
        dm_node = self._get_mashup_element(root)
        if dm_node is None:
            return xml_bytes

        if dm_node.text and whitespace:
            dm_node.text = self._with_whitespace(dm_node.text)

        xml_bytes = etree.tostring(root, encoding="utf-8", xml_declaration=True)
        return self._encode_bytes(xml_bytes, encoding)

    def _with_whitespace(self, text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return text
        midpoint = max(1, len(cleaned) // 2)
        return f"\n  {cleaned[:midpoint]}\n  {cleaned[midpoint:]}\n"

    def _encode_bytes(self, xml_bytes: bytes, encoding: str) -> bytes:
        enc = encoding.lower()
        if enc == "utf-8":
            return xml_bytes
        if enc == "utf-16-le":
            return self._to_utf16(xml_bytes, little_endian=True)
        if enc == "utf-16-be":
            return self._to_utf16(xml_bytes, little_endian=False)
        raise ValueError(f"Unsupported encoding: {encoding}")

    def _to_utf16(self, xml_bytes: bytes, little_endian: bool) -> bytes:
        text = xml_bytes.decode("utf-8")
        text = self._rewrite_declaration(text)
        encoded = text.encode("utf-16-le" if little_endian else "utf-16-be")
        bom = b"\xff\xfe" if little_endian else b"\xfe\xff"
        return bom + encoded

    def _rewrite_declaration(self, text: str) -> str:
        pattern = r'encoding=["\'][^"\']+["\']'
        if re.search(pattern, text):
            return re.sub(pattern, 'encoding="UTF-16"', text, count=1)
        prefix = "<?xml version='1.0'?>"
        if text.startswith(prefix):
            return text.replace(prefix, "<?xml version='1.0' encoding='UTF-16'?>", 1)
        return text


class MashupPermissionsMetadataGenerator(MashupBaseGenerator):
    """
    Builds fixtures that exercise Permissions and Metadata parsing by rewriting
    the PackageParts Section1.m, Permissions XML, and Metadata XML inside
    the DataMashup stream.
    """

    def __init__(self, args):
        super().__init__(args)
        self.mode = args.get("mode")

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if not self.mode:
            raise ValueError("MashupPermissionsMetadataGenerator requires 'mode' argument")

        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        version, package_parts, _, _, bindings = self._split_sections(raw_bytes)
        scenario = self._scenario_definition()

        updated_package_parts = self._replace_section(
            package_parts,
            scenario["section_text"],
        )
        permissions_bytes = self._permissions_bytes(**scenario["permissions"])
        metadata_bytes = self._metadata_bytes(scenario["metadata_entries"])

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions_bytes,
            metadata_bytes,
            bindings,
        )

    def _scenario_definition(self):
        shared_section_simple = "\n".join(
            [
                "section Section1;",
                "",
                "shared LoadToSheet = 1;",
                "shared LoadToModel = 2;",
            ]
        )

        if self.mode in ("permissions_defaults", "permissions_firewall_off", "metadata_simple"):
            return {
                "section_text": shared_section_simple,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": self.mode != "permissions_firewall_off",
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/LoadToSheet",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/LoadToModel",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", True),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_query_groups":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared RootQuery = 1;",
                    "shared GroupedFoo = 2;",
                    "shared NestedBar = 3;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/RootQuery",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/GroupedFoo",
                        "entries": [
                            ("FillEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                    {
                        "path": "Section1/NestedBar",
                        "entries": [
                            ("FillToDataModelEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_hidden_queries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared ConnectionOnly = 1;",
                    "shared VisibleLoad = 2;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/ConnectionOnly",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/VisibleLoad",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        raise ValueError(f"Unsupported mode: {self.mode}")

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts = raw_bytes[offset : offset + package_parts_len]
        offset += package_parts_len

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions = raw_bytes[offset : offset + permissions_len]
        offset += permissions_len

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata = raw_bytes[offset : offset + metadata_len]
        offset += metadata_len

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings = raw_bytes[offset : offset + bindings_len]

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _replace_section(self, package_parts: bytes, section_text: str) -> bytes:
        return self._replace_in_zip(package_parts, "Formulas/Section1.m", section_text)

    def _replace_in_zip(self, zip_bytes: bytes, filename: str, new_content: str) -> bytes:
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()

        with zipfile.ZipFile(in_buffer, "r") as zin:
            with zipfile.ZipFile(out_buffer, "w", compression=zipfile.ZIP_DEFLATED) as zout:
                for item in zin.infolist():
                    if item.filename == filename:
                        zout.writestr(filename, new_content.encode("utf-8"))
                    else:
                        zout.writestr(item, zin.read(item.filename))
        return out_buffer.getvalue()

    def _permissions_bytes(self, can_eval: bool, firewall_enabled: bool, group_type: str) -> bytes:
        xml = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<PermissionList xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" "
            "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
            f"<CanEvaluateFuturePackages>{str(can_eval).lower()}</CanEvaluateFuturePackages>"
            f"<FirewallEnabled>{str(firewall_enabled).lower()}</FirewallEnabled>"
            f"<WorkbookGroupType>{group_type}</WorkbookGroupType>"
            "</PermissionList>"
        )
        return ("\ufeff" + xml).encode("utf-8")

    def _metadata_bytes(self, items: List[dict]) -> bytes:
        xml = self._metadata_xml(items)
        xml_bytes = ("\ufeff" + xml).encode("utf-8")
        header = struct.pack("<I", 0) + struct.pack("<I", len(xml_bytes))
        return header + xml_bytes

    def _metadata_xml(self, items: List[dict]) -> str:
        parts = [
            '<?xml version="1.0" encoding="utf-8"?>',
            '<LocalPackageMetadataFile xmlns:xsd="http://www.w3.org/2001/XMLSchema" '
            'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">',
            "<Items>",
            "<Item><ItemLocation><ItemType>AllFormulas</ItemType><ItemPath /></ItemLocation><StableEntries /></Item>",
        ]

        for item in items:
            parts.append("<Item>")
            parts.append("<ItemLocation>")
            parts.append("<ItemType>Formula</ItemType>")
            parts.append(f"<ItemPath>{item['path']}</ItemPath>")
            parts.append("</ItemLocation>")
            parts.append("<StableEntries>")
            for entry_name, entry_value in item.get("entries", []):
                value = self._format_entry_value(entry_value)
                parts.append(f'<Entry Type="{entry_name}" Value="{value}" />')
            parts.append("</StableEntries>")
            parts.append("</Item>")

        parts.append("</Items></LocalPackageMetadataFile>")
        return "".join(parts)

    def _format_entry_value(self, value):
        if isinstance(value, bool):
            return f"l{'1' if value else '0'}"
        return f"s{value}"

