# Pyroduct Examples

## Running Existing Pipelines

To run the transform example:
```bash
pyroduct run examples/transform/pipeline.yaml examples/transform/data.jsonl -o examples/transform/
```

Or use the TUI:
```bash
pyroduct tui examples/transform/pipeline.yaml examples/transform/data.jsonl
```

The TUI maintains a second pipeline in `pipeline.tui.json` that you can edit.

## Learning the Framework with Python Examples

### Quick Start: Python → Rust Capability Development

Start with the Python examples to understand the framework before writing Rust:

#### 1. Explore Python Capability Interfaces
```bash
cat examples/python_capabilities/csv_transform.py
cat examples/python_capabilities/data_validation.py
cat examples/python_capabilities/customer_etl.py
```

These show the three-part capability structure:
- **Config**: Initialization settings (e.g., CSV delimiter)
- **Client**: Per-call state (e.g., which columns to keep)
- **Server**: The actual methods (e.g., parse_csv, validate_row)

#### 2. Convert Python to Rust with py2rs
```bash
# Generate Rust boilerplate from Python
pyroduct py2rs examples/python_capabilities/csv_transform.py \
  -o lib/capabilities/csv-transform/src/lib.rs
```

This creates:
- Rust struct definitions
- FFI boilerplate
- Client method stubs

#### 3. Implement the Rust Capability
Edit `lib/capabilities/csv-transform/src/lib.rs` and implement the methods:
```rust
#[pyroduct::capability]
impl CsvTransformServer {
    /// Parse CSV string into dictionary
    async fn parse_csv(&self, client: &CsvTransformClient, data: String) -> Result<...> {
        // Your implementation here
        Ok(parsed_data)
    }
}
```

#### 4. Compile to Dylib
```bash
cd lib/capabilities/csv-transform
cargo build --release
ls artifacts/lib.so  # or lib.dylib on macOS
```

#### 5. Use in a Rust Module
Create a module that calls your capability:
```rust
use csv_transform::{CsvTransformClient, CsvTransformClientMethods};

#[pyroduct::module(output = result)]
fn process_csv(input: String) -> Result<String> {
    let client = CsvTransformClient {
        skip_rows: 0,
        column_filter: None,
    }.register()?;
    
    let parsed = client.parse_csv(input)?;
    Ok(format!("{:?}", parsed))
}
```

#### 6. Define a Pipeline
```yaml
pipeline:
  transform:
    module: lib/modules/csv_module/
    configurations:
      csv_transform:
        delimiter: ","
        has_header: true
```

#### 7. Run the Pipeline
```bash
pyroduct run examples/python_capabilities/customer_pipeline.yaml data.jsonl -o output/
```

## Example Structures

```
examples/
├── python_capabilities/                      # Python interfaces and examples
│   ├── csv_transform.py                      # Parse and transform CSV
│   ├── data_validation.py                    # Validate data against schemas
│   ├── customer_etl.py                       # Complete ETL workflow example
│   ├── customer_pipeline.yaml                # Example pipeline configuration
│   └── README.md                             # py2rs workflow and patterns guide
├── chat/                                     # Existing chat example
├── transform/                                # Existing transform example
└── README.md                                 # This file
```

## Workflow Summary

**For users new to Pyroduct:**
1. Read `python_capabilities/README.md` 
2. Look at `csv_transform.py` - shows capability structure
3. Run `py2rs` to convert to Rust
4. Read `pytpython_capabilities/csv_transform.py` - shows capability structure
3. Look at `python_capabilities/customer_etl.py` - shows module usage patterns
4. Run `py2rs` to convert a capability to Rust
5. Implement your Rust capability and modules
6. Create a pipeline.yaml and run with your data

**Key files by topic:**
- **Capability patterns**: `python_capabilities/csv_transform.py`, `data_validation.py`
- **Module patterns**: `python_capabilities/customer_etl.py`
- **Real pipeline**: `python_capabilities/customer_pipeline.yaml