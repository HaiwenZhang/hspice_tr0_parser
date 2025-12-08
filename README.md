# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![C API](https://img.shields.io/badge/C_API-Available-blue.svg)](#c-api)

High-performance HSPICE binary file parser with **Python**, **Rust**, and **C** APIs.

## Overview

A pure Rust implementation for reading HSPICE binary output files (.tr0, .ac0, .sw0) with bindings for multiple languages:

| Language   | Status | Use Case                                    |
| ---------- | ------ | ------------------------------------------- |
| **Python** | ✅     | Data analysis, scripting, NumPy integration |
| **Rust**   | ✅     | Native performance, Rust applications       |
| **C/C++**  | ✅     | Legacy integration, EDA tools, MATLAB MEX   |

## Supported Formats

| Format | Version   | Precision | Description                   |
| ------ | --------- | --------- | ----------------------------- |
| 9601   | 9007/9601 | float32   | Standard HSPICE binary format |
| 2001   | 2001      | float64   | Double precision format       |

| Extension | Analysis Type              |
| --------- | -------------------------- |
| `.tr0`    | Transient analysis         |
| `.ac0`    | AC analysis (complex data) |
| `.sw0`    | DC sweep analysis          |

## Features

- 🚀 **High Performance**: Memory-mapped I/O and bulk data conversion
- 📦 **Multi-Language**: Python, Rust, and C APIs from single codebase
- 🔄 **Format Conversion**: Convert to SPICE3/ngspice compatible .raw format
- 📊 **NumPy Integration**: Direct NumPy array output (Python)
- 🖥️ **Cross-Platform**: Linux, macOS, Windows

---

## Python API

### Installation

```bash
# From source
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser
python3 -m venv .venv && source .venv/bin/activate
pip install maturin numpy
maturin develop --release

# Or build wheel
maturin build --release
pip install target/wheels/hspicetr0parser-*.whl
```

### Usage

```python
from hspice_tr0_parser import hspice_tr0_read, hspice_tr0_to_raw

# Read file (returns NumPy arrays by default)
result = hspice_tr0_read('simulation.tr0')

# Or return Python lists (no NumPy dependency)
result = hspice_tr0_read('simulation.tr0', data_type='list')

# Access data
data = result[0][0][2][0]  # First sweep's data dictionary
time = data['TIME']
voltage = data['v(out)']

# Convert to SPICE3 raw format
hspice_tr0_to_raw('simulation.tr0', 'simulation.raw')
```

### Return Format

```python
[
    (
        (sweep_name, sweep_values, [data_dict, ...]),  # Simulation results
        scale_name,        # e.g., "TIME"
        None,              # Placeholder
        title,             # Simulation title
        date,              # Date string
        None               # Placeholder
    )
]
```

### Complete Example

```python
from hspice_tr0_parser import hspice_tr0_read
import matplotlib.pyplot as plt

result = hspice_tr0_read('example/PinToPinSim.tr0')
data = result[0][0][2][0]

time = data['TIME']
signals = [k for k in data.keys() if k != 'TIME']

plt.figure(figsize=(12, 6))
for sig in signals[:5]:
    plt.plot(time * 1e9, data[sig], label=sig)

plt.xlabel('Time (ns)')
plt.ylabel('Voltage (V)')
plt.legend()
plt.grid(True)
plt.show()
```

---

## Rust API

### Add to Cargo.toml

```toml
[dependencies]
hspicetr0parser = { version = "1.0", default-features = false }
```

### Usage

```rust
use hspicetr0parser::{read, read_and_convert, VectorData};

fn main() -> hspicetr0parser::Result<()> {
    // Read HSPICE file
    let result = read("simulation.tr0")?;

    println!("Title: {}", result.title);
    println!("Scale: {}", result.scale_name);

    // Access data
    for table in &result.data_tables {
        for (name, data) in table {
            match data {
                VectorData::Real(vec) => {
                    println!("{}: {} points", name, vec.len());
                }
                VectorData::Complex(vec) => {
                    println!("{}: {} complex points", name, vec.len());
                }
            }
        }
    }

    // Convert to SPICE3 raw format
    read_and_convert("input.tr0", "output.raw")?;

    Ok(())
}
```

### API Reference

```rust
// Read HSPICE file
pub fn read(filename: &str) -> Result<HspiceResult>;
pub fn read_debug(filename: &str, debug: i32) -> Result<HspiceResult>;

// Convert to SPICE3 raw
pub fn read_and_convert(input: &str, output: &str) -> Result<()>;
pub fn read_and_convert_debug(input: &str, output: &str, debug: i32) -> Result<()>;

// Core types
pub struct HspiceResult {
    pub title: String,
    pub date: String,
    pub scale_name: String,
    pub sweep_name: Option<String>,
    pub sweep_values: Option<Vec<f64>>,
    pub data_tables: Vec<HashMap<String, VectorData>>,
}

pub enum VectorData {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}
```

---

## C API

### Build Static Library

```bash
cargo build --release --no-default-features
# Output: target/release/libhspicetr0parser.a
```

### Usage

```c
#include "hspice_tr0_parser.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    // Read HSPICE file
    CHspiceResult* result = hspice_read("simulation.tr0", 0);
    if (!result) {
        printf("Failed to read file\n");
        return 1;
    }

    // Get metadata
    printf("Title: %s\n", hspice_result_get_title(result));
    printf("Scale: %s\n", hspice_result_get_scale_name(result));
    printf("Tables: %d\n", hspice_result_get_table_count(result));

    // Get signal count
    int count = hspice_result_get_signal_count(result, 0);
    printf("Signals: %d\n", count);

    // Get TIME data
    int len = hspice_result_get_signal_length(result, 0, "TIME");
    double* time = malloc(len * sizeof(double));
    hspice_result_get_signal_real(result, 0, "TIME", time, len);

    printf("First: %e, Last: %e\n", time[0], time[len-1]);

    // Cleanup
    free(time);
    hspice_result_free(result);
    return 0;
}
```

### Compile

```bash
gcc -o example example.c -I./include -L./target/release -lhspicetr0parser
```

### API Reference

```c
// Core functions
CHspiceResult* hspice_read(const char* filename, int debug);
void hspice_result_free(CHspiceResult* result);

// Metadata
const char* hspice_result_get_title(const CHspiceResult* result);
const char* hspice_result_get_date(const CHspiceResult* result);
const char* hspice_result_get_scale_name(const CHspiceResult* result);
int hspice_result_get_table_count(const CHspiceResult* result);

// Sweep data
int hspice_result_has_sweep(const CHspiceResult* result);
const char* hspice_result_get_sweep_name(const CHspiceResult* result);
int hspice_result_get_sweep_count(const CHspiceResult* result);
int hspice_result_get_sweep_values(const CHspiceResult* result, double* out, int max);

// Signal data
int hspice_result_get_signal_count(const CHspiceResult* result, int table);
int hspice_result_get_signal_names(const CHspiceResult* result, int table, const char** out, int max);
int hspice_result_get_signal_length(const CHspiceResult* result, int table, const char* name);
int hspice_result_signal_is_complex(const CHspiceResult* result, int table, const char* name);
int hspice_result_get_signal_real(const CHspiceResult* result, int table, const char* name, double* out, int max);
int hspice_result_get_signal_complex(const CHspiceResult* result, int table, const char* name, double* real, double* imag, int max);
```

---

## Project Structure

```
hspice_tr0_parser/
├── Cargo.toml              # Rust package configuration
├── pyproject.toml          # Python package configuration
├── hspice_tr0_parser.py    # Python wrapper module
├── include/
│   └── hspice_tr0_parser.h # C API header file
├── src/
│   ├── lib.rs              # Module entry + API exports
│   ├── types.rs            # Types, constants, error definitions
│   ├── reader.rs           # Memory-mapped file reader
│   ├── parser.rs           # HSPICE binary parser
│   ├── writer.rs           # SPICE3 raw file writer
│   └── ffi.rs              # C Foreign Function Interface
├── tests/                  # Python test suite (66 tests)
└── example/                # Example HSPICE files
```

## Testing

```bash
# Python tests
pip install pytest
pytest tests/ -v  # 66 tests

# Rust tests
cargo test --no-default-features
```

## Performance

| Optimization          | Description                                 |
| --------------------- | ------------------------------------------- |
| Memory-mapped I/O     | Uses `memmap2` for efficient file access    |
| Generic Bulk Read     | Unified trait for f32/f64 conversion        |
| Pre-allocated Buffers | Minimizes memory allocations during parsing |
| Modular Design        | Clean separation of parsing stages          |

## License

MIT License - see [LICENSE](LICENSE) file.

## Acknowledgments

- [hspicefile](https://pypi.org/project/hspicefile)
- [PyOPUS](https://fides.fe.uni-lj.si/pyopus/)
- [hspiceParser](https://github.com/HMC-ACE/hspiceParser)
- Google Antigravity
