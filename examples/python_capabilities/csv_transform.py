"""
CSV Transform Capability - Example for py2rs conversion

This Python file demonstrates a capability interface that can be converted to Rust
using the py2rs tool.

Usage:
  pyroduct py2rs examples/python_capabilities/csv_transform.py \\
    -o lib/capabilities/csv-transform/src/lib.rs

The generated Rust code will need to be completed with actual implementation.
"""
from dataclasses import dataclass
from typing import List


@dataclass
class CsvTransformConfig:
    """Configuration for the CSV Transform capability."""
    delimiter: str = ","
    has_header: bool = True
    encoding: str = "utf-8"
    skip_errors: bool = False


@dataclass
class CsvTransformClient:
    """Per-client state for CSV transformation operations."""
    skip_rows: int = 0
    column_filter: List[str] = None  # None means all columns


class CsvTransformServer:
    """
    CSV Transform capability - Parse, filter, and transform CSV data.
    
    Provides methods for parsing CSV strings, filtering columns, and transforming
    individual rows with custom logic.
    """

    def __init__(self, config: CsvTransformConfig = None):
        """Initialize the CSV Transform server with optional configuration."""
        self.config = config or CsvTransformConfig()

    def parse_csv(self, client: CsvTransformClient, data: str) -> dict:
        """
        Parse CSV string into a dictionary with headers and rows.
        
        Args:
            client: Client state with filtering preferences
            data: Raw CSV data as string
            
        Returns:
            Dictionary with 'headers' and 'rows' keys
        """
        pass

    def filter_columns(self, client: CsvTransformClient, data: dict) -> dict:
        """
        Filter CSV columns based on client configuration.
        
        Args:
            client: Client state specifying which columns to keep
            data: Parsed CSV data (from parse_csv)
            
        Returns:
            Filtered data dictionary
        """
        pass

    def transform_row(
        self,
        client: CsvTransformClient,
        row: dict,
        transformations: dict
    ) -> dict:
        """
        Apply transformations to a row.
        
        Transformations can include type conversion, field renaming, and custom functions.
        
        Args:
            client: Client state
            row: Single row as dictionary
            transformations: Dict mapping field names to transformation functions
            
        Returns:
            Transformed row dictionary
        """
        pass

    def count_rows(self, client: CsvTransformClient, data: dict) -> int:
        """Count the number of rows in parsed CSV data."""
        pass
