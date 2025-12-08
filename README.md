# hspicetr0parser

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)

Read and convert HSPICE binary output files (.tr0) from Python.

## Overview

This package provides high-performance functions to:

- **Read** HSPICE binary output files (.tr0) and return data as NumPy arrays
- **Convert** HSPICE .tr0 files to SPICE3 binary raw format (.raw)

## Features

- 🚀 **High Performance**: Memory-mapped I/O, parallel processing with Rayon, and bulk data conversion
- 📦 **Minimal Dependencies**: Pure Rust implementation with PyO3 bindings
- 🔄 **Format Conversion**: Convert .tr0 to SPICE3/ngspice compatible .raw format
- 📊 **NumPy Integration**: Direct NumPy array output for seamless data analysis
- 🖥️ **Cross-Platform**: Supports Linux and macOS

## Requirements

- Python >= 3.10
- NumPy
- Rust toolchain (for building from source)

## Installation

### From Source (Development)

```bash
# Clone the repository
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or: .venv\Scripts\activate  # Windows

# Install build tools
pip install maturin numpy

# Build and install in development mode
maturin develop --release
```

### Build Wheel Package

```bash
# Build optimized wheel
maturin build --release

# Install the wheel
pip install target/wheels/hspicetr0parser-*.whl
```

## Usage

### Reading HSPICE .tr0 Files

```python
from hspice_tr0_parser import hspice_tr0_read

# Read a .tr0 file
result = hspice_tr0_read('simulation.tr0')

# With debug output
result = hspice_tr0_read('simulation.tr0', debug=1)

# Access metadata
title = result[0][3]
date = result[0][4]
scale_name = result[0][1]  # Usually "TIME" for transient analysis

# Access signal data
data = result[0][0][2][0]  # First sweep's data dictionary

# Iterate over all signals
for name, values in data.items():
    print(f"{name}: {len(values)} points, range [{values.min():.3e}, {values.max():.3e}]")

# Get specific signal
time = data['TIME']
voltage = data['v(out)']  # Signal names are lowercase
```

### Converting to SPICE3 Raw Format

```python
from hspice_tr0_parser import hspice_tr0_to_raw

# Convert .tr0 to .raw format
success = hspice_tr0_to_raw('simulation.tr0', 'simulation.raw')

if success:
    print("Conversion completed successfully!")
else:
    print("Conversion failed")

# With debug output
hspice_tr0_to_raw('simulation.tr0', 'simulation.raw', debug=1)
```

### Complete Example

```python
from hspice_tr0_parser import hspice_tr0_read, hspice_tr0_to_raw
import matplotlib.pyplot as plt

# Read HSPICE file
result = hspice_tr0_read('example/PinToPinSim.tr0')
data = result[0][0][2][0]

# Get time and voltage signals
time = data['TIME']
signals = [k for k in data.keys() if k != 'TIME']

# Plot first few signals
plt.figure(figsize=(12, 6))
for sig in signals[:5]:
    plt.plot(time * 1e9, data[sig], label=sig)

plt.xlabel('Time (ns)')
plt.ylabel('Voltage (V)')
plt.legend()
plt.grid(True)
plt.title(f'HSPICE Simulation - {len(signals)} signals, {len(time)} points')
plt.show()

# Convert to SPICE3 raw format for use with other tools
hspice_tr0_to_raw('example/PinToPinSim.tr0', 'example/output.raw')
```

## Return Format

The `hspice_tr0_read()` function returns a nested structure:

```python
[
    (
        (sweep_name, sweep_values, [data_dict, ...]),  # Simulation results
        scale_name,        # Independent variable name (e.g., "TIME")
        None,              # Placeholder
        title,             # Simulation title
        date,              # Date string
        None               # Placeholder
    )
]
```

| Field          | Description                                                     |
| -------------- | --------------------------------------------------------------- |
| `sweep_name`   | Name of swept parameter (or `None` if no sweep)                 |
| `sweep_values` | NumPy array of sweep values (or `None`)                         |
| `data_dict`    | Dictionary with signal names as keys and NumPy arrays as values |
| `scale_name`   | Name of the independent variable (usually "TIME")               |

## SPICE3 Raw Output Format

The generated `.raw` file follows the SPICE3/ngspice binary format:

```
Title: <simulation title>
Date: <date string>
Plotname: Transient Analysis
Flags: real
No. Variables: <count>
No. Points: <count>
Variables:
    0   TIME    time
    1   signal1 voltage
    2   signal2 voltage
    ...
Binary:
<binary data>
```

## Project Structure

```
hspice_tr0_parser/
├── Cargo.toml              # Rust package configuration
├── pyproject.toml          # Python package configuration
├── hspice_tr0_parser.py    # Python wrapper module
├── src/
│   ├── lib.rs              # Module entry + PyO3 bindings
│   ├── types.rs            # Types, constants, error definitions
│   ├── reader.rs           # Memory-mapped file reader
│   ├── parser.rs           # HSPICE binary parser
│   └── writer.rs           # SPICE3 raw file writer
├── tests/
│   ├── conftest.py         # Pytest fixtures
│   └── test_tr0_parser.py  # Python test suite
├── example/
│   └── PinToPinSim.tr0     # Example test file
└── docs/
```

## Testing

Run the test suite using pytest:

```bash
# Install development dependencies
pip install pytest

# Run all tests
pytest tests/ -v

# Run specific test class
pytest tests/test_tr0_parser.py::TestHspiceTr0Read -v

# Run with coverage (optional)
pip install pytest-cov
pytest tests/ -v --cov=hspice_tr0_parser
```

## Performance

Optimized for large files using:

| Optimization          | Description                                          |
| --------------------- | ---------------------------------------------------- |
| Memory-mapped I/O     | Uses `memmap2` crate for efficient file access       |
| Parallel Processing   | Leverages `rayon` for multi-threaded data conversion |
| Bulk Conversion       | Single-pass byte-to-float conversion                 |
| Pre-allocated Buffers | Minimizes memory allocations during parsing          |

## Dependencies

### Rust Dependencies

| Crate       | Version | Purpose             |
| ----------- | ------- | ------------------- |
| `pyo3`      | 0.27.2  | Python bindings     |
| `numpy`     | 0.27.1  | NumPy array support |
| `byteorder` | 1.5.0   | Byte order handling |
| `memmap2`   | 0.9.9   | Memory-mapped files |
| `rayon`     | 1.11.0  | Parallel processing |

### Python Dependencies

- `numpy` - Required runtime dependency

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Thanks

- [hspicefile](https://pypi.org/project/hspicefile)
- [PyOPUS](https://fides.fe.uni-lj.si/pyopus/)
- [hspiceParser](https://github.com/HMC-ACE/hspiceParser)
- Google Antigravity
