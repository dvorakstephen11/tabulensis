import random
import shutil
import zipfile
from pathlib import Path
from typing import List, Union

from .base import BaseGenerator


def _deterministic_bytes(length: int, seed) -> bytes:
    rng = random.Random(seed)
    try:
        return rng.randbytes(length)
    except AttributeError:
        # Python <3.9: no randbytes(). Generate in chunks to avoid huge ints.
        out = bytearray()
        while len(out) < length:
            chunk = min(65536, length - len(out))
            out.extend(rng.getrandbits(chunk * 8).to_bytes(chunk, "little"))
        return bytes(out)


class ZipPadGenerator(BaseGenerator):
    """
    Copy a base ZIP container and append a deterministic padding entry.

    This is useful when a test or UI scenario needs to cross a file-size threshold
    (e.g., desktop auto-large-mode) without paying workbook parse cost.

    Args:
      base_file: Path to an existing fixture file (relative to repo root via `fixtures/` or absolute).
      pad_bytes: Number of bytes to add as an uncompressed ZIP entry.
      entry_name: Optional ZIP entry name (default: "xl/_tabulensis_padding.bin").
      seed: Optional deterministic seed for the padding bytes.
    """

    DEFAULT_ENTRY_NAME = "xl/_tabulensis_padding.bin"

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file")
        if not base_file_arg:
            raise ValueError("zip_pad generator requires 'base_file' argument")

        pad_bytes = self.args.get("pad_bytes")
        if pad_bytes is None:
            raise ValueError("zip_pad generator requires 'pad_bytes' argument")
        try:
            pad_len = int(pad_bytes)
        except Exception as e:
            raise ValueError("zip_pad generator arg 'pad_bytes' must be an integer") from e
        if pad_len <= 0:
            raise ValueError("zip_pad generator arg 'pad_bytes' must be > 0")

        entry_name = self.args.get("entry_name") or self.DEFAULT_ENTRY_NAME
        if not isinstance(entry_name, str) or not entry_name.strip():
            raise ValueError("zip_pad generator arg 'entry_name' must be a non-empty string")
        entry_name = entry_name.strip()

        seed = self.args.get("seed", "tabulensis-zip-pad-v1")
        pad_data = _deterministic_bytes(pad_len, seed)

        base = self._resolve_fixture_path(str(base_file_arg))

        for name in output_names:
            target_path = (output_dir / name).resolve()
            shutil.copyfile(base, target_path)
            with zipfile.ZipFile(target_path, "a") as zout:
                info = zipfile.ZipInfo(entry_name)
                info.compress_type = zipfile.ZIP_STORED
                info.date_time = (1980, 1, 1, 0, 0, 0)
                zout.writestr(info, pad_data)

    def _resolve_fixture_path(self, value: str) -> Path:
        candidate = Path(value)
        if candidate.exists():
            return candidate
        fallback = Path("fixtures") / value
        if fallback.exists():
            return fallback
        raise FileNotFoundError(f"Fixture file not found: {value}")

