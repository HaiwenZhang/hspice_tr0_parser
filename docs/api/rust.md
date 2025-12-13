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

Read a waveform file.

```rust
use hspice_core::read;

let result = read("simulation.tr0")?;
println!("Title: {}", result.title);
println!("Analysis: {:?}", result.analysis);
```

#### `read_debug(filename: &str, debug: i32) -> Result<WaveformResult>`

Read with debug output (0=quiet, 1=info, 2=verbose).

#### `read_and_convert(input: &str, output: &str) -> Result<()>`

Convert HSPICE file to SPICE3 raw format.

```rust
hspice_core::read_and_convert("input.tr0", "output.raw")?;
```

### Streaming API

#### `read_stream(path: &str) -> Result<HspiceStreamReader>`

Stream large files in chunks.

```rust
use hspice_core::read_stream;

for chunk in read_stream("large_file.tr0")? {
    let chunk = chunk?;
    println!("Chunk {}: {} points", chunk.chunk_index, chunk.time.len());
}
```

#### `read_stream_chunked(path: &str, chunk_size: usize) -> Result<HspiceStreamReader>`

Control minimum points per chunk.

#### `read_stream_signals(path: &str, signals: &[&str], chunk_size: usize) -> Result<HspiceStreamReader>`

Filter to specific signals.

```rust
let signals = ["TIME", "v(out)"];
let reader = hspice_core::read_stream_signals("file.tr0", &signals, 10000)?;
```

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
    pub time: Vec<f64>,
    pub time_range: (f64, f64),
    pub data: HashMap<String, VectorData>,
}
```

## Complete Example

```rust
use hspice_core::{read, VectorData, AnalysisType};

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

| Format | Version   | Precision | Extensions       |
| ------ | --------- | --------- | ---------------- |
| 9601   | 9007/9601 | float32   | .tr0, .ac0, .sw0 |
| 2001   | 2001      | float64   | .tr0, .ac0, .sw0 |
