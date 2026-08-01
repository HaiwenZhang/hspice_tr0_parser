# Rust API Documentation

This document covers the Rust API for `hspice-core`.

## Installation

```toml
[dependencies]
hspice-core = { git = "https://github.com/HaiwenZhang/hspice_tr0_parser" }
```

## API Reference

### Core Functions

#### `read(filename: &str) -> Result<WaveformResult>`

Read an HSPICE binary waveform file.

```rust
use hspice_core::read;

let result = read("simulation.tr0")?;
println!("Title: {}", result.title);
println!("Analysis: {:?}", result.analysis);
```

#### `read_bytes(data: &[u8], filename_hint: &str) -> Result<WaveformResult>`

Read HSPICE data from memory. `filename_hint` is used only when the analysis
cannot be inferred from the header scale.

#### `read_and_convert(input: &str, output: &str) -> Result<()>`

Convert HSPICE file to SPICE3 raw format.

```rust
hspice_core::read_and_convert("input.tr0", "output.raw")?;
```

This conversion writes the first data table. For direct control, parse the
input and call `write_spice3_raw(&result, output)`.

#### `read_raw(filename: &str) -> Result<WaveformResult>`

Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).

```rust
use hspice_core::read_raw;

let result = read_raw("simulation.raw")?;
println!("Title: {}", result.title);
println!("Analysis: {:?}", result.analysis);
```

#### `read_raw_bytes(data: &[u8]) -> Result<WaveformResult>`

Read binary or ASCII SPICE3 raw data from memory.

#### `convert_raw_to_hspice(input: &str, output: &str, version: PostVersion) -> Result<()>`

Convert a SPICE3/ngspice binary or ASCII raw file to HSPICE. The output
extension must match the analysis: `.trN` for transient, `.acN` for AC, and
`.swN` for DC.

```rust
use hspice_core::{convert_raw_to_hspice, PostVersion};

convert_raw_to_hspice("input.raw", "output.tr0", PostVersion::V9601)?;
```

#### `write_hspice(result: &WaveformResult, output: &str, version: PostVersion) -> Result<()>`

Write the format-neutral `WaveformResult` as HSPICE. This is the extension
point for waveform formats other than SPICE3 raw: parse them into a
`WaveformResult`, then call `write_hspice`.

```rust
use hspice_core::{read_raw, write_hspice, PostVersion};

let result = read_raw("transient.raw")?;
write_hspice(&result, "output.tr0", PostVersion::V2001)?;
```

The writer supports transient, AC, and DC analyses. It validates the output
extension, vector/table consistency, finite representable values, and HSPICE
name constraints before creating the output file.

#### `write_spice3_raw(result: &WaveformResult, output: &str) -> Result<()>`

Write the first table of a `WaveformResult` as little-endian SPICE3 binary raw.
Complex components are preserved; real vectors receive a zero imaginary part
when the table contains any complex vector.

### Deprecated Compatibility Functions

`read_debug`, `read_raw_debug`, and `read_and_convert_debug` remain exported
for source compatibility. Their debug arguments are ignored. Use the non-debug
function and install a `tracing` subscriber in the calling application.

### Streaming API

#### `read_stream(path: &str) -> Result<HspiceStreamReader>`

Stream large tr0 files in chunks.

```rust
use hspice_core::read_stream;

for chunk in read_stream("large_file.tr0")? {
    let chunk = chunk?;
    println!("Chunk {}: {:?}", chunk.chunk_index, chunk.time_range);
}
```

#### `read_stream_chunked(path: &str, chunk_size: usize) -> Result<HspiceStreamReader>`

Control minimum points per chunk.

#### `read_stream_signals(path: &str, signals: &[&str], chunk_size: usize) -> Result<HspiceStreamReader>`

Filter to specific signals.

```rust
let signals = ["v(out"];
let reader = hspice_core::read_stream_signals("file.tr0", &signals, 10000)?;
```

The scale vector is always included. `chunk_size` is a minimum target because
the reader consumes complete HSPICE records. `HspiceStreamReader::metadata()`
returns header metadata, and `reset()` restarts at the first data block.

## Data Types

### `WaveformResult`

Main result structure.

```rust
pub struct WaveformResult {
    pub title: String,
    pub date: String,
    pub analysis: AnalysisType,
    pub variables: Vec<Variable>,
    pub sweep_param: Option<String>,
    pub tables: Vec<DataTable>,
}
```

**Methods:**

- `scale_name() -> &str`: Get scale variable name
- `get(name: &str) -> Option<&VectorData>`: Get signal by name
- `var_index(name: &str) -> Option<usize>`: Get variable index
- `var_names() -> Vec<&str>`: Get all variable names
- `len() -> usize`: Number of data points
- `num_vars() -> usize`: Number of variables
- `num_sweeps() -> usize`: Number of sweeps
- `has_sweep() -> bool`: Check for sweep data

### `AnalysisType`

```rust
pub enum AnalysisType {
    Transient,
    AC,
    DC,
    Operating,
    Noise,
    Unknown,
}
```

### `PostVersion`

```rust
pub enum PostVersion {
    V9601, // HSPICE 9601, f32 values
    V2001, // HSPICE 2001, f64 values
}
```

### `Variable`

```rust
pub struct Variable {
    pub name: String,
    pub var_type: VarType,
}
```

### `VarType`

```rust
pub enum VarType {
    Time,
    Frequency,
    Voltage,
    Current,
    Unknown,
}
```

### `DataTable`

```rust
pub struct DataTable {
    pub sweep_value: Option<f64>,
    pub vectors: Vec<VectorData>,
}
```

### `VectorData`

```rust
pub enum VectorData {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}
```

### `DataChunk` (Streaming)

```rust
pub struct DataChunk {
    pub chunk_index: usize,
    pub time_range: (f64, f64),
    pub data: HashMap<String, VectorData>,
}
```

`time_range` is the range of the scale vector; for AC and DC files it is a
frequency or sweep range. Streaming currently stops at the first data-table
end marker, so use `read()` for all tables in a parameter sweep.

## Complete Example

```rust
use hspice_core::{read, VectorData};

fn main() -> hspice_core::Result<()> {
    let result = read("simulation.tr0")?;

    println!("Title: {}", result.title);
    println!("Date: {}", result.date);
    println!("Analysis: {:?}", result.analysis);
    println!("Scale: {}", result.scale_name());
    println!("Variables: {}", result.num_vars());
    println!("Points: {}", result.len());

    // List variables
    for var in &result.variables {
        println!("  {}: {:?}", var.name, var.var_type);
    }

    // Access data
    if let Some(VectorData::Real(time)) = result.get("TIME") {
        println!("Time range: {:.3e} to {:.3e}",
            time.first().unwrap(), time.last().unwrap());
    }

    // Check for sweep
    if result.has_sweep() {
        println!("Sweep: {:?}", result.sweep_param);
        for table in &result.tables {
            println!("  Value: {:?}", table.sweep_value);
        }
    }

    Ok(())
}
```

## Supported Formats

| Format | Read | Write | Notes |
| ------ | ---- | ----- | ----- |
| HSPICE 9007/9601 binary | Yes | 9601 output | `f32`, `.trN` / `.acN` / `.swN` |
| HSPICE 2001 binary | Yes | Yes | `f64`, `.trN` / `.acN` / `.swN` |
| SPICE3 raw binary | Yes | Yes | Little-endian `f64` |
| SPICE3 raw ASCII | Yes | No | `Values:` input |
