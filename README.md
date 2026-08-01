# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.12–3.14](https://img.shields.io/badge/Python-3.12%20%7C%203.13%20%7C%203.14-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![C API](https://img.shields.io/badge/C_API-Available-blue.svg)](docs/api/c_cpp.md)
[![Go](https://img.shields.io/badge/Go-CGO-00ADD8.svg)](docs/api/golang.md)
[![Java](https://img.shields.io/badge/Java-JNA-red.svg)](docs/api/java.md)
[![WASM](https://img.shields.io/badge/WASM-Browser-purple.svg)](docs/api/wasm.md)

High-performance HSPICE binary file and SPICE3 raw file parser with **Python**, **Rust**, **C/C++**, **Go**, **Java**, **WebAssembly** APIs, and a standalone **command-line tool**.

## Features

- 🚀 **High Performance**: Memory-mapped I/O with Rust
- 📦 **Multi-Language**: Python, Rust, C/C++, Go, Java, and WASM from one codebase
- 🛠️ **Standalone CLI**: One native executable, no Python runtime required
- 🔄 **Format Conversion**: Convert in both directions between HSPICE and SPICE3/ngspice `.raw`
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

## Release Downloads

Pushing a `v*` tag builds one GitHub Release containing:

- Python `cp312-abi3` wheels compatible with Python 3.12, 3.13, and 3.14
  on Linux, macOS, and Windows, plus a source distribution.
- Standalone CLI archives for Linux x86_64/aarch64, macOS Intel/Apple Silicon,
  and Windows x86_64.
- C ABI static/dynamic libraries and `hspice_tr0_parser.h` for the same native
  targets. Go callers use these archives through CGO.
- An npm-compatible WASM package plus the Java/JNA wrapper JAR and Maven POM.
- `SHA256SUMS` for every attached artifact.

GitHub also provides the repository source archives automatically. See
[docs/RELEASING.md](docs/RELEASING.md) for the asset matrix and release process.

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
| `convert <IN> <OUT>` | Convert HSPICE ↔ SPICE3 raw; `--post-version 9601\|2001` for HSPICE output |
| `stream <FILE>`     | Stream chunks as JSON Lines (`--chunk-size`, `--signal`)            |
| `export <FILE>`     | Export to CSV (`--output`, `--format auto\|hspice\|raw`, `--signal`, `--delimiter`) |

Global option: `--log-level trace|debug|info|warn|error` (default `warn`).

Examples:

```bash
hspice-cli info simulation.tr0
hspice-cli read simulation.tr0 --json | jq .num_points
hspice-cli convert simulation.tr0 simulation.raw
hspice-cli convert simulation.raw simulation.tr0
hspice-cli convert simulation.raw simulation.tr0 --post-version 2001
hspice-cli export simulation.tr0 --signal TIME --signal "v(out" -o out.csv
hspice-cli stream huge.tr0 --chunk-size 50000 | process_each_chunk.py
```

Use `.tr0` for transient, `.ac0` for AC, and `.sw0` for DC output. Version
`9601` is the default because it matches the widest range of WaveView versions;
`2001` preserves double precision. The writer reproduces the fixed-width
header and record framing found in `example/`; round-trip tests reproduce the
9601 TR0, 2001 TR0, 9601 AC0, and 9601 SW0 references byte for byte.

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
├── tests/                    # Python integration tests
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
