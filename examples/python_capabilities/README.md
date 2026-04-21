# Python Examples for py2rs - Capabilities and Modules

These Python files demonstrate how to define capabilities and modules that can be automatically converted to Rust using the `py2rs` tool.

## Overview

The `py2rs` command generates Rust FFI glue code from Python capability definitions. The workflow is:

1. **Define capability interface in Python** - Specify config, client state, and methods
2. **Convert to Rust** - Run `py2rs` to generate the boilerplate
3. **Implement in Rust** - Complete the method implementations
4. **Compile and test** - Build as dylib/WASM module

## Capabilities

### 1. CSV Transform Capability
**File:** `csv_transform.py`

Demonstrates:
- `@dataclass` config structures for capability initialization
- Client state definition with per-invocation data
- Multiple capability methods (parse, filter, transform)

**Convert to Rust:**
```bash
pyroduct py2rs examples/python_capabilities/csv_transform.py \
  -o lib/capabilities/csv-transform/src/lib.rs
```

**Use in Rust module:**
```rust
use csv_transform::{CsvTransformClient, CsvTransformClientMethods};

#[pyroduct::module(output = result)]
fn transform_data(csv_string: String) -> Result<String> {
    let client = CsvTransformClient {
        skip_rows: 1,
        column_filter: vec!["id", "name"].into(),
    }.register()?;
    
    let parsed = client.parse_csv(csv_string)?;
    Ok(format!("{:?}", parsed))
}
```

### 2. Data Validation Capability
**File:** `data_validation.py`

Demonstrates:
- Complex nested dataclass structures (SchemaField)
- Configuration for error handling modes
- Stateful validation with schema registration
- Multiple validation methods

**Convert to Rust:**
```bash
pyroduct py2rs examples/python_capabilities/data_validation.py \
  -o lib/capabilities/data-validation/src/lib.rs
```

## Key Patterns

### Configuration
```python
@dataclass
class MyConfig:
    """Passed in pipeline.yaml"""
    setting1: str
    setting2: bool = True
```

Maps to `#[pyroduct::config]` in Rust.

### Client State
```python
@dataclass
class MyClient:
    """Per-call state, serialized across FFI"""
    filter_value: str
    verbose: bool = False
```

Maps to `#[pyroduct::magma]` in Rust.

### Methods
```python
def my_method(self, client: MyClient, input: str) -> dict:
    """Becomes a callable method on the client"""
    pass
```

Each method becomes callable from WASM modules.

## Running the Examples

### Convert all capabilities:
```bash
for py_file in examples/python_capabilities/*.py; do
    pyroduct py2rs "$py_file"
done
```

### Use in a pipeline:
```bash
pyroduct run examples/customer_pipeline.yaml examples/customer_data.jsonl -o output/
```

## Modules

### Customer ETL Module
**File:** `customer_etl.py`

A complete ETL pipeline example that demonstrates:
- Parsing CSV data using CsvTransformClient
- Validating data using ValidationClient
- Deduplicating and normalizing records
- Error handling and reporting

## Pipeline Configuration

**File:** `customer_pipeline.yaml`

Example pipeline that uses the customer ETL module with configuration for validation and CSV parsing.

## Next Steps

1. Convert a Python capability: `pyroduct py2rs examples/python_capabilities/csv_transform.py`
2. Implement the generated Rust code with actual logic
3. Review `customer_etl.py` for module usage patterns
4. Create a Rust module that uses your capability
5. Define a pipeline.yaml that orchestrates them
6. Run the pipeline with sample data: `pyroduct run examples/python_capabilities/customer_pipeline.yaml data.jsonl -o output/`
