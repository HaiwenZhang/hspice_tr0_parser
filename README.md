# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![C API](https://img.shields.io/badge/C_API-Available-blue.svg)](docs/api/c_cpp.md)
[![Go](https://img.shields.io/badge/Go-CGO-00ADD8.svg)](docs/api/golang.md)
[![Java](https://img.shields.io/badge/Java-JNA-red.svg)](docs/api/java.md)
[![WASM](https://img.shields.io/badge/WASM-Browser-purple.svg)](docs/api/wasm.md)

High-performance HSPICE binary file and SPICE3 raw file parser with **Python**, **Rust**, **C/C++**, **Go**, **Java**, **WebAssembly** APIs, and a standalone **command-line tool**.

## Features

- 🚀 **High Performance**: Memory-mapped I/O with Rust
- 📦 **Multi-Language**: Python, Rust, C, Go, WASM from single codebase
- 🛠️ **Standalone CLI**: One static binary, no Python or runtime required
- 🔄 **Format Conversion**: Convert to SPICE3/ngspice .raw format
- 📊 **Streaming**: Process GB-sized files with minimal memory
- 📑 **CSV Export**: One-shot dump for downstream analysis
- 🖥️ **Cross-Platform**: Linux, macOS, Windows, Browser

## Supported Formats

| Format      | Type           | Extensions       |
| ----------- | -------------- | ---------------- |
| HSPICE 9601 | Binary float32 | .tr0, .ac0, .sw0 |
| HSPICE 2001 | Binary float64 | .tr0, .ac0, .sw0 |
| SPICE3 Raw  | Binary/ASCII   | .raw             |

## API Documentation

| Language   | Documentation                            |
| ---------- | ---------------------------------------- |
| **Python** | [docs/api/python.md](docs/api/python.md) |
| **Rust**   | [docs/api/rust.md](docs/api/rust.md)     |
| **C/C++**  | [docs/api/c_cpp.md](docs/api/c_cpp.md)   |
| **Go**     | [docs/api/golang.md](docs/api/golang.md) |
| **Java**   | [docs/api/java.md](docs/api/java.md)     |
| **WASM**   | [docs/api/wasm.md](docs/api/wasm.md)     |

## Command-Line Tool

`hspice-cli` mirrors the Python API as subcommands, so the project is usable
without any runtime. Prebuilt binaries for Linux / macOS / Windows are attached
to GitHub Releases; or build from source:

```bash
cargo build -p hspice-cli --release
./target/release/hspice-cli --help
```

| Subcommand          | Description                                                         |
| ------------------- | ------------------------------------------------------------------- |
| `info <FILE>`       | Print only the file header (fast, no data read)                     |
| `read <FILE>`       | Read HSPICE file; `--json` for structured output, `--signal NAME` to dump one signal |
| `read-raw <FILE>`   | Read SPICE3 raw file (auto binary / ASCII)                          |
| `convert <IN> <OUT>` | Convert HSPICE → SPICE3 binary raw                                 |
| `stream <FILE>`     | Stream chunks as JSON Lines (`--chunk-size`, `--signal`)            |
| `export <FILE>`     | Export to CSV (`--output`, `--format auto\|hspice\|raw`, `--signal`, `--delimiter`) |

Global option: `--log-level trace|debug|info|warn|error` (default `warn`).

Examples:

```bash
hspice-cli info simulation.tr0
hspice-cli read simulation.tr0 --json | jq .num_points
hspice-cli convert simulation.tr0 simulation.raw
hspice-cli export simulation.tr0 --signal TIME --signal "v(out)" -o out.csv
hspice-cli stream huge.tr0 --chunk-size 50000 | process_each_chunk.py
```

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
│   ├── hspice-wasm/         # WebAssembly bindings
│   └── hspice-cli/          # Standalone command-line tool
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

# Build the standalone CLI
cargo build -p hspice-cli --release

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
