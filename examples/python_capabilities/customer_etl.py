"""
Customer ETL Module - Example Pyroduct module using capabilities

This example shows how a Pyroduct module would use capabilities in a real ETL pipeline.

In the actual Rust implementation, this would use capabilities like:
  - CsvTransformClient for parsing CSV data
  - ValidationClient for data quality checks

The module would be configured in a pipeline.yaml as:
  
  pipeline:
    etl_customers:
      module: examples/modules/customer_etl/
      configurations:
        csv_transform:
          delimiter: ","
          has_header: true
        validation:
          strict_mode: true
          
Then run with:
  pyroduct run examples/python_capabilities/customer_pipeline.yaml examples/customer_data.jsonl
"""
from dataclasses import dataclass
from typing import List, Dict, TypedDict


class ValidationError(TypedDict):
    """Validation error information."""
    field: str
    value: str
    error: str


class CustomerRecord(TypedDict):
    """Cleaned customer record."""
    customer_id: str
    name: str
    email: str
    purchase_count: int


class ETLResult(TypedDict):
    """Result of the ETL process."""
    success: bool
    record_count: int
    error_count: int
    records: List[CustomerRecord]
    errors: List[ValidationError]


def process_customer_data(input_data: str) -> ETLResult:
    """
    ETL Pipeline for customer data:
    
    1. Parse CSV using CsvTransformClient
    2. Validate each row using ValidationClient
    3. Deduplicate and normalize fields
    4. Output cleaned records
    
    Args:
        input_data: Raw CSV string with customer data
        
    Returns:
        ETLResult with cleaned records or errors
    """
    
    # Step 1: Parse CSV
    # In actual Rust: parse_client.parse_csv(input_data)
    parsed_data = parse_csv_data(input_data)
    
    if not parsed_data:
        return {
            "success": False,
            "record_count": 0,
            "error_count": 1,
            "records": [],
            "errors": [
                {
                    "field": "input",
                    "value": input_data[:50],
                    "error": "Failed to parse CSV data"
                }
            ]
        }
    
    # Step 2: Validate each row
    # In actual Rust: validation_client.validate_row(row)
    validated_records = []
    validation_errors = []
    
    for row in parsed_data:
        validation_result = validate_customer_row(row)
        if validation_result["is_valid"]:
            validated_records.append(row)
        else:
            for error_msg in validation_result["errors"]:
                validation_errors.append({
                    "field": "customer_record",
                    "value": str(row),
                    "error": error_msg
                })
    
    # Step 3: Deduplicate and normalize
    cleaned_records = deduplicate_and_normalize(validated_records)
    
    return {
        "success": len(validation_errors) == 0,
        "record_count": len(cleaned_records),
        "error_count": len(validation_errors),
        "records": cleaned_records,
        "errors": validation_errors
    }


def parse_csv_data(csv_string: str) -> List[Dict]:
    """Parse CSV string into list of dictionaries."""
    lines = csv_string.strip().split("\n")
    if not lines:
        return []
    
    headers = lines[0].split(",")
    records = []
    
    for line in lines[1:]:
        values = line.split(",")
        if len(values) == len(headers):
            records.append({
                headers[i].strip(): values[i].strip()
                for i in range(len(headers))
            })
    
    return records


def validate_customer_row(row: Dict) -> Dict:
    """Validate a customer record."""
    errors = []
    
    # Validate customer_id
    if not row.get("customer_id", "").strip():
        errors.append("customer_id is required")
    
    # Validate name
    if not row.get("name", "").strip():
        errors.append("name is required")
    
    # Validate email format
    email = row.get("email", "").strip()
    if not email:
        errors.append("email is required")
    elif "@" not in email or "." not in email.split("@")[1]:
        errors.append(f"Invalid email format: {email}")
    
    # Validate purchase_count is numeric
    try:
        int(row.get("purchase_count", 0))
    except ValueError:
        errors.append(f"purchase_count must be numeric, got: {row.get('purchase_count')}")
    
    return {
        "is_valid": len(errors) == 0,
        "errors": errors
    }


def deduplicate_and_normalize(records: List[Dict]) -> List[CustomerRecord]:
    """Deduplicate records and normalize field values."""
    seen = set()
    cleaned = []
    
    for record in records:
        customer_id = record.get("customer_id", "").strip().upper()
        
        if customer_id not in seen and customer_id:
            cleaned.append({
                "customer_id": customer_id,
                "name": record.get("name", "").strip(),
                "email": record.get("email", "").strip().lower(),
                "purchase_count": int(record.get("purchase_count", 0))
            })
            seen.add(customer_id)
    
    return cleaned
