# tr0parser

Read and convert HSPICE binary output files (.tr0) from Python.

## Overview

This package provides functions to:

- **Read** HSPICE binary output files (.tr0) and return data as NumPy arrays
- **Convert** HSPICE .tr0 files to SPICE3 binary raw format (.raw)

Based on the original PyOPUS implementation by Janez Puhan, rewritten in Rust with PyO3 bindings for better performance by Haiwen Zhang.

## Features

- 🚀 **High Performance**: Memory-mapped I/O and bulk data processing
- 📦 **Zero Dependencies**: Pure Rust implementation with PyO3 bindings
- 🔄 **Format Conversion**: Convert .tr0 to SPICE3 .raw format
- 📊 **NumPy Integration**: Direct NumPy array output

## Requirements

- Python >= 3.8
- NumPy
- Rust toolchain (for building from source)

## Installation

### From Source (Development)

```bash
# Clone the repository
git clone <repository-url>
cd tr0parser

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
pip install target/wheels/tr0parser-*.whl
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
hspice_tr0_to_raw('example/PinToPinSim.tr0', 'output.raw')
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

- **sweep_name**: Name of swept parameter (or `None` if no sweep)
- **sweep_values**: NumPy array of sweep values (or `None`)
- **data_dict**: Dictionary with signal names as keys and NumPy arrays as values
- **scale_name**: Name of the independent variable (usually "TIME")

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
tr0parser/
├── Cargo.toml              # Rust package configuration
├── pyproject.toml          # Python package configuration
├── README.md               # This file
├── hspice_tr0_parser.py    # Python wrapper module
├── src/
│   ├── lib.rs              # Module entry + PyO3 bindings
│   ├── types.rs            # Types, constants, error definitions
│   ├── reader.rs           # Memory-mapped file reader
│   ├── parser.rs           # HSPICE binary parser
│   └── writer.rs           # SPICE3 raw file writer
└── example/
    └── PinToPinSim.tr0     # Example test file
```

## Performance

Optimized for large files using:

- Memory-mapped file I/O (`memmap2`)
- Bulk byte-to-float conversion
- Single-pass data reading
- Pre-allocated buffers

## License

MIT
