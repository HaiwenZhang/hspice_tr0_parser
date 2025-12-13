# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![C API](https://img.shields.io/badge/C_API-Available-blue.svg)](docs/api/c_cpp.md)
[![Go](https://img.shields.io/badge/Go-CGO-00ADD8.svg)](docs/api/golang.md)

High-performance HSPICE binary file parser with **Python**, **Rust**, **C**, and **Go** APIs.

## Overview

A pure Rust implementation for reading HSPICE binary output files (.tr0, .ac0, .sw0) with bindings for multiple languages:

| Language   | Status | Documentation                            |
| ---------- | ------ | ---------------------------------------- |
| **Python** | ✅     | [docs/api/python.md](docs/api/python.md) |
| **Rust**   | ✅     | [docs/api/rust.md](docs/api/rust.md)     |
| **C/C++**  | ✅     | [docs/api/c_cpp.md](docs/api/c_cpp.md)   |
| **Go**     | ✅     | [docs/api/golang.md](docs/api/golang.md) |

## Supported Formats

| Format | Version   | Precision | Extensions       |
| ------ | --------- | --------- | ---------------- |
| 9601   | 9007/9601 | float32   | .tr0, .ac0, .sw0 |
| 2001   | 2001      | float64   | .tr0, .ac0, .sw0 |

## Features

- 🚀 **High Performance**: Memory-mapped I/O and bulk data conversion
- 📦 **Multi-Language**: Python, Rust, C, and Go APIs from single codebase
- 🔄 **Format Conversion**: Convert to SPICE3/ngspice compatible .raw format
- 📊 **Streaming**: Process large files in chunks for memory efficiency
- 🖥️ **Cross-Platform**: Linux, macOS, Windows

## Quick Start

### Python

```bash
pip install maturin numpy
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser && maturin develop --release
```

```python
from hspice_tr0_parser import hspice_tr0_read

result = hspice_tr0_read('simulation.tr0')
data = result[0][0][2][0]
time = data['TIME']
voltage = data['v(out)']
```

### Rust

```toml
[dependencies]
hspice-core = { git = "https://github.com/HaiwenZhang/hspice_tr0_parser" }
```

```rust
let result = hspice_core::read("simulation.tr0")?;
println!("Title: {}", result.title);
```

### C/C++

```bash
cargo build -p hspice-ffi --release
# Output: target/release/libhspicetr0parser.a
```

```c
CHspiceResult* result = hspice_read("simulation.tr0", 0);
printf("Title: %s\n", hspice_result_get_title(result));
hspice_result_free(result);
```

## Documentation

| Document                             | Description                  |
| ------------------------------------ | ---------------------------- |
| [Architecture](docs/architecture.md) | Multi-crate workspace design |
| [Python API](docs/api/python.md)     | Full Python API reference    |
| [Rust API](docs/api/rust.md)         | Full Rust API reference      |
| [C/C++ API](docs/api/c_cpp.md)       | Full C FFI reference         |
| [Go API](docs/api/golang.md)         | Go CGO integration guide     |

## Project Structure

```
hspice_tr0_parser/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── hspice-core/       # Pure Rust library
│   ├── hspice-python/     # Python bindings (PyO3)
│   └── hspice-ffi/        # C FFI bindings
├── docs/                   # Documentation
│   ├── architecture.md
│   └── api/               # API docs per language
├── hspice_tr0_parser.py    # Python wrapper
├── include/                # C header files
├── tests/                  # Python tests (85 tests)
└── example/                # Example HSPICE files
```

## Building

```bash
# Build all crates
cargo build --release

# Build Python extension
maturin develop --release

# Build C static library
cargo build -p hspice-ffi --release

# Run tests
pytest tests/ -v
```

## License

MIT License - see [LICENSE](LICENSE) file.

## Acknowledgments

- [hspicefile](https://pypi.org/project/hspicefile)
- [PyOPUS](https://fides.fe.uni-lj.si/pyopus/)
- [hspiceParser](https://github.com/HMC-ACE/hspiceParser)
