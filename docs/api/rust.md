# Rust API Documentation

This document covers the Rust API for `hspice-core`, the pure Rust library for parsing HSPICE binary files.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hspice-core = { git = "https://github.com/HaiwenZhang/hspice_tr0_parser" }
```

Or for local development:

```toml
[dependencies]
hspice-core = { path = "path/to/hspice_tr0_parser/crates/hspice-core" }
```

## Building

```bash
# Clone the repository
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser

# Build the core library
cargo build -p hspice-core --release
```

## API Reference

### Core Functions

#### `read(filename: &str) -> Result<HspiceResult>`

Read an HSPICE binary file (.tr0, .ac0, .sw0).

```rust
use hspice_core::read;

fn main() -> hspice_core::Result<()> {
    let result = read("simulation.tr0")?;
    println!("Title: {}", result.title);
    println!("Date: {}", result.date);
    println!("Scale: {}", result.scale_name);
    Ok(())
}
```

#### `read_debug(filename: &str, debug: i32) -> Result<HspiceResult>`

Read with debug output (0=quiet, 1=info, 2=verbose).

```rust
let result = hspice_core::read_debug("simulation.tr0", 1)?;
```

#### `read_and_convert(input: &str, output: &str) -> Result<()>`

Convert HSPICE file to SPICE3 binary raw format.

```rust
use hspice_core::read_and_convert;

fn main() -> hspice_core::Result<()> {
    read_and_convert("input.tr0", "output.raw")?;
    println!("Conversion complete!");
    Ok(())
}
```

### Streaming API

For large files, use the streaming API to process data in chunks:

#### `read_stream(path: &str) -> Result<HspiceStreamReader>`

```rust
use hspice_core::read_stream;

fn main() -> hspice_core::Result<()> {
    let reader = read_stream("large_file.tr0")?;

    for chunk in reader {
        let chunk = chunk?;
        println!("Chunk {}: {} points, time {:.3e} to {:.3e}",
            chunk.chunk_index,
            chunk.time.len(),
            chunk.time_range.0,
            chunk.time_range.1
        );
    }
    Ok(())
}
```

#### `read_stream_chunked(path: &str, chunk_size: usize) -> Result<HspiceStreamReader>`

Control the minimum number of time points per chunk:

```rust
let reader = hspice_core::read_stream_chunked("file.tr0", 50000)?;
```

#### `read_stream_signals(path: &str, signals: &[&str], chunk_size: usize) -> Result<HspiceStreamReader>`

Filter to specific signals:

```rust
let signals = ["TIME", "v(out)", "i(vdd)"];
let reader = hspice_core::read_stream_signals("file.tr0", &signals, 10000)?;
```

### Data Types

#### `HspiceResult`

```rust
pub struct HspiceResult {
    pub title: String,
    pub date: String,
    pub scale_name: String,              // e.g., "TIME", "FREQUENCY"
    pub sweep_name: Option<String>,      // Sweep parameter name
    pub sweep_values: Option<Vec<f64>>,  // Sweep values
    pub data_tables: Vec<HashMap<String, VectorData>>,
}
```

#### `VectorData`

```rust
pub enum VectorData {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}
```

#### `DataChunk` (Streaming)

```rust
pub struct DataChunk {
    pub chunk_index: usize,
    pub time: Vec<f64>,
    pub time_range: (f64, f64),
    pub data: HashMap<String, VectorData>,
}
```

### Error Handling

```rust
pub enum HspiceError {
    IoError(std::io::Error),
    ParseError(String),
    FormatError(String),
}
```

## Complete Example

```rust
use hspice_core::{read, VectorData};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Read HSPICE file
    let result = read("example/PinToPinSim.tr0")?;

    println!("=== Simulation Info ===");
    println!("Title: {}", result.title);
    println!("Date: {}", result.date);
    println!("Scale: {}", result.scale_name);

    // Check for sweep
    if let Some(ref name) = result.sweep_name {
        println!("Sweep: {}", name);
        if let Some(ref values) = result.sweep_values {
            println!("Sweep values: {:?}", values);
        }
    }

    // Process data tables
    for (i, table) in result.data_tables.iter().enumerate() {
        println!("\n=== Table {} ===", i);
        println!("Signals: {}", table.len());

        for (name, data) in table {
            match data {
                VectorData::Real(vec) => {
                    println!("  {}: {} real points", name, vec.len());
                    if !vec.is_empty() {
                        println!("    First: {:.6e}, Last: {:.6e}",
                            vec.first().unwrap(),
                            vec.last().unwrap()
                        );
                    }
                }
                VectorData::Complex(vec) => {
                    println!("  {}: {} complex points", name, vec.len());
                }
            }
        }
    }

    Ok(())
}
```

## Supported Formats

| Format | Version   | Precision | Extension        |
| ------ | --------- | --------- | ---------------- |
| 9601   | 9007/9601 | float32   | .tr0, .ac0, .sw0 |
| 2001   | 2001      | float64   | .tr0, .ac0, .sw0 |
