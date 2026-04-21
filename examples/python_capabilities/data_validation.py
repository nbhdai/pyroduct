"""
Data Validation Capability - Example for py2rs conversion

Demonstrates a schema validation capability that can be converted to Rust.

Usage:
  pyroduct py2rs examples/python_capabilities/data_validation.py \\
    -o lib/capabilities/data-validation/src/lib.rs
"""
from dataclasses import dataclass
from typing import List, Dict


@dataclass
class ValidationConfig:
    """Configuration for the Data Validation capability."""
    strict_mode: bool = True
    max_errors: int = 100
    collect_errors: bool = True


@dataclass
class ValidationClient:
    """Per-client validation state."""
    schema_name: str
    fail_on_error: bool = True


@dataclass
class SchemaField:
    """Schema field definition."""
    name: str
    field_type: str  # "string", "int", "float", "bool"
    required: bool = True
    pattern: str = None  # Regex pattern for string validation
    min_value: float = None  # For numeric types
    max_value: float = None  # For numeric types


@dataclass
class ValidationResult:
    """Result of a validation operation."""
    is_valid: bool
    error_count: int
    error_messages: List[str]


class ValidationServer:
    """
    Data Validation capability - Type checking and schema validation.
    
    Registers and enforces schemas, validating incoming data against
    predefined field types and constraints.
    """

    def __init__(self, config: ValidationConfig = None):
        """Initialize the validation server."""
        self.config = config or ValidationConfig()
        self.schemas: Dict[str, List[SchemaField]] = {}

    def validate_row(self, client: ValidationClient, row: dict) -> ValidationResult:
        """
        Validate a single row against the registered schema.
        
        Args:
            client: Client state specifying which schema to use
            row: Dictionary representing a data row
            
        Returns:
            ValidationResult with validity status and any error messages
        """
        pass

    def register_schema(self, client: ValidationClient, schema_json: str) -> bool:
        """
        Register a schema for validation.
        
        Schema JSON format:
        [
            {"name": "id", "type": "int", "required": true},
            {"name": "email", "type": "string", "pattern": "^[^@]+@[^@]+\\.[^@]+$", "required": true},
            {"name": "age", "type": "int", "min_value": 0, "max_value": 150, "required": false}
        ]
        
        Args:
            client: Client state
            schema_json: JSON string defining the schema
            
        Returns:
            True if schema was registered successfully
        """
        pass

    def get_validation_errors(self, client: ValidationClient) -> List[str]:
        """
        Get accumulated validation errors for this client.
        
        Args:
            client: Client state
            
        Returns:
            List of error message strings
        """
        pass

    def clear_errors(self, client: ValidationClient) -> None:
        """Clear accumulated errors for this client."""
        pass

    def validate_field(
        self,
        client: ValidationClient,
        field_name: str,
        field_value: str
    ) -> ValidationResult:
        """
        Validate a single field value.
        
        Args:
            client: Client state
            field_name: Name of the field to validate
            field_value: Value to validate as string
            
        Returns:
            ValidationResult for this field
        """
        pass
