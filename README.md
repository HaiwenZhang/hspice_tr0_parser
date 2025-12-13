# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![C API](https://img.shields.io/badge/C_API-Available-blue.svg)](docs/api/c_cpp.md)
[![WASM](https://img.shields.io/badge/WASM-Browser-purple.svg)](docs/api/wasm.md)

High-performance HSPICE binary file parser with **Python**, **Rust**, **C/C++**, and **WebAssembly** APIs.

## Features

- 🚀 **High Performance**: Memory-mapped I/O with Rust
- 📦 **Multi-Language**: Python, Rust, C, WASM from single codebase
- 🔄 **Format Conversion**: Convert to SPICE3/ngspice .raw format
- 📊 **Streaming**: Process GB-sized files with minimal memory
- 🖥️ **Cross-Platform**: Linux, macOS, Windows, Browser

## Supported Formats

| Format | Version   | Precision | Extensions       |
| ------ | --------- | --------- | ---------------- |
| 9601   | 9007/9601 | float32   | .tr0, .ac0, .sw0 |
| 2001   | 2001      | float64   | .tr0, .ac0, .sw0 |

## Quick Start

### Python

```bash
pip install maturin numpy
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser && maturin develop --release
```

```python
from hspice_tr0_parser import read, convert_to_raw, stream

# Read waveform file
result = read('simulation.tr0')
print(result.title)           # Simulation title
print(result.analysis)        # 'transient', 'ac', 'dc'
print(result.scale_name)      # 'TIME', 'HERTZ'

# Access signal data (NumPy arrays)
time = result.get('TIME')
vout = result.get('v(out)')

# List all variables
for var in result.variables:
    print(f"{var.name}: {var.var_type}")

# Convert to SPICE3 raw format
convert_to_raw('input.tr0', 'output.raw')

# Stream large files
for chunk in stream('huge.tr0', chunk_size=100000):
    print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
```

### Rust

```toml
[dependencies]
hspice-core = { git = "https://github.com/HaiwenZhang/hspice_tr0_parser" }
```

```rust
use hspice_core::{read, WaveformResult};

fn main() -> hspice_core::Result<()> {
    let result = read("simulation.tr0")?;
    println!("Title: {}", result.title);
    println!("Analysis: {:?}", result.analysis);
    println!("Variables: {}", result.num_vars());

    // Access data by name
    if let Some(time) = result.get("TIME") {
        println!("Points: {}", time.len());
    }
    Ok(())
}
```

### C/C++

```bash
cargo build -p hspice-ffi --release
# Output: target/release/libhspicetr0parser.a
```

```c
#include "hspice_tr0_parser.h"

CWaveformResult* result = waveform_read("simulation.tr0", 0);
if (result) {
    printf("Title: %s\n", waveform_get_title(result));
    printf("Analysis: %d\n", waveform_get_analysis_type(result));
    printf("Variables: %d\n", waveform_get_var_count(result));
    waveform_free(result);
}
```

### WebAssembly

```typescript
import init, { parseHspice } from "hspice-wasm";

await init();

const fileData = new Uint8Array(await file.arrayBuffer());
const result = parseHspice(fileData);

console.log(result.title);
console.log(result.analysis); // 'transient', 'ac', 'dc'
const time = result.tables[0].signals["TIME"];
```

## API Documentation

| Language   | Documentation                            |
| ---------- | ---------------------------------------- |
| **Python** | [docs/api/python.md](docs/api/python.md) |
| **Rust**   | [docs/api/rust.md](docs/api/rust.md)     |
| **C/C++**  | [docs/api/c_cpp.md](docs/api/c_cpp.md)   |
| **WASM**   | [docs/api/wasm.md](docs/api/wasm.md)     |

## Project Structure

```
hspice_tr0_parser/
├── Cargo.toml               # Workspace definition
├── pyproject.toml           # Python package config
├── hspice_tr0_parser.py     # Python wrapper
├── crates/
│   ├── hspice-core/         # Pure Rust library
│   ├── hspice-python/       # Python bindings (PyO3)
│   ├── hspice-ffi/          # C FFI bindings
│   └── hspice-wasm/         # WebAssembly bindings
├── include/                  # C header files
├── docs/                     # Documentation
│   ├── ARCHITECTURE.md
│   └── api/
├── tests/                    # Python tests (91 tests)
└── example/                  # Example HSPICE files
```

## Building

```bash
# Build all Rust crates
cargo build --release

# Build Python extension
maturin develop --release

# Build C static library
cargo build -p hspice-ffi --release

# Build WASM (requires wasm-pack)
cd crates/hspice-wasm && wasm-pack build --target web

# Run tests
cargo test -p hspice-core
pytest tests/ -v
```

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for multi-crate workspace design.

## License

MIT License - see [LICENSE](LICENSE) file.
