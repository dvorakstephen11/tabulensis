"""Base classes for fixture generators."""

from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List


class BaseGenerator(ABC):
    """Abstract base class for all fixture generators."""

    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """Generate the fixture file(s).

        Args:
            output_dir: The directory to save the file(s) in.
            output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass
