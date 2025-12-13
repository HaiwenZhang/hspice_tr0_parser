# Python API Documentation

This document covers the Python API for `hspicetr0parser`, providing NumPy-integrated access to HSPICE binary files.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or: .venv\Scripts\activate  # Windows

# Install build tools and build
pip install maturin numpy
maturin develop --release
```

### Build Wheel

```bash
maturin build --release
pip install target/wheels/hspicetr0parser-*.whl
```

## API Reference

### `hspice_tr0_read(filename, debug=0, data_type='numpy')`

Read an HSPICE binary file.

**Parameters:**

- `filename` (str): Path to the HSPICE file
- `debug` (int): Debug level (0=quiet, 1=info, 2=verbose)
- `data_type` (str): `'numpy'` for NumPy arrays, `'list'` for Python lists

**Returns:** List containing simulation results

```python
from hspice_tr0_parser import hspice_tr0_read

result = hspice_tr0_read('simulation.tr0')
```

### `hspice_tr0_to_raw(input_path, output_path, debug=0)`

Convert HSPICE file to SPICE3 binary raw format.

**Parameters:**

- `input_path` (str): Path to input HSPICE file
- `output_path` (str): Path for output .raw file
- `debug` (int): Debug level

**Returns:** `True` on success, `False` on failure

```python
from hspice_tr0_parser import hspice_tr0_to_raw

success = hspice_tr0_to_raw('simulation.tr0', 'output.raw')
```

### `hspice_tr0_stream(filename, chunk_size=10000, signals=None, debug=0)`

Stream large files in chunks for memory efficiency.

**Parameters:**

- `filename` (str): Path to HSPICE file
- `chunk_size` (int): Minimum points per chunk
- `signals` (list[str]): Optional signal filter
- `debug` (int): Debug level

**Returns:** Generator yielding chunk dictionaries

```python
from hspice_tr0_parser import hspice_tr0_stream

for chunk in hspice_tr0_stream('large_file.tr0', chunk_size=50000):
    print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
```

## Result Structure

The result from `hspice_tr0_read` has the following structure:

```python
[
    (
        (sweep_name, sweep_values, [data_dict, ...]),  # Sweep info + data
        scale_name,        # e.g., "TIME"
        None,              # Reserved
        title,             # Simulation title
        date,              # Date string
        None               # Reserved
    )
]
```

### Accessing Data

```python
result = hspice_tr0_read('simulation.tr0')

# Extract components
sweep_info = result[0][0]
sweep_name = sweep_info[0]      # Sweep parameter name (or None)
sweep_values = sweep_info[1]    # Sweep values (or None)
data_tables = sweep_info[2]     # List of data dictionaries

scale_name = result[0][1]       # "TIME", "FREQUENCY", etc.
title = result[0][3]            # Simulation title
date = result[0][4]             # Date string

# Get first data table
data = data_tables[0]

# Access signals (NumPy arrays)
time = data['TIME']
voltage = data['v(out)']
```

## Complete Examples

### Basic Reading

```python
from hspice_tr0_parser import hspice_tr0_read
import numpy as np

# Read file
result = hspice_tr0_read('example/PinToPinSim.tr0')

# Get data
data = result[0][0][2][0]
scale_name = result[0][1]
title = result[0][3]

print(f"Title: {title}")
print(f"Scale: {scale_name}")
print(f"Signals: {list(data.keys())}")

# Access time and signals
time = data['TIME']
print(f"Time range: {time[0]:.3e} to {time[-1]:.3e}")
print(f"Points: {len(time)}")
```

### Plotting with Matplotlib

```python
from hspice_tr0_parser import hspice_tr0_read
import matplotlib.pyplot as plt

result = hspice_tr0_read('simulation.tr0')
data = result[0][0][2][0]

time = data['TIME'] * 1e9  # Convert to nanoseconds

plt.figure(figsize=(12, 6))

# Plot first 5 voltage signals
for name in list(data.keys())[:6]:
    if name != 'TIME' and name.startswith('v('):
        plt.plot(time, data[name], label=name)

plt.xlabel('Time (ns)')
plt.ylabel('Voltage (V)')
plt.title('HSPICE Transient Simulation')
plt.legend()
plt.grid(True)
plt.show()
```

### Processing Large Files

```python
from hspice_tr0_parser import hspice_tr0_stream
import numpy as np

# Process in chunks to save memory
all_time = []
all_vout = []

for chunk in hspice_tr0_stream('large_sim.tr0', chunk_size=100000):
    data = chunk['data']
    all_time.append(data['TIME'])
    all_vout.append(data['v(out)'])

# Combine chunks
time = np.concatenate(all_time)
vout = np.concatenate(all_vout)

print(f"Total points: {len(time)}")
```

### Filtering Signals

```python
from hspice_tr0_parser import hspice_tr0_stream

# Only load specific signals
signals = ['TIME', 'v(out)', 'v(in)', 'i(vdd)']

for chunk in hspice_tr0_stream('simulation.tr0', signals=signals):
    data = chunk['data']
    # Only requested signals are present
    print(f"Loaded signals: {list(data.keys())}")
```

### Converting to SPICE3 Raw Format

```python
from hspice_tr0_parser import hspice_tr0_to_raw

# Convert for use with ngspice or other tools
if hspice_tr0_to_raw('hspice_output.tr0', 'ngspice_compatible.raw'):
    print("Conversion successful!")
else:
    print("Conversion failed")
```

### Working with Sweep Data

```python
from hspice_tr0_parser import hspice_tr0_read

result = hspice_tr0_read('sweep_simulation.tr0')

sweep_name = result[0][0][0]
sweep_values = result[0][0][1]
data_tables = result[0][0][2]

if sweep_name:
    print(f"Sweep parameter: {sweep_name}")
    print(f"Sweep values: {sweep_values}")

    # Each sweep point has its own data table
    for i, (val, table) in enumerate(zip(sweep_values, data_tables)):
        time = table['TIME']
        print(f"  {sweep_name}={val}: {len(time)} points")
```

## Supported Formats

| Extension | Analysis Type | Data Type |
| --------- | ------------- | --------- |
| `.tr0`    | Transient     | Real      |
| `.ac0`    | AC Analysis   | Complex   |
| `.sw0`    | DC Sweep      | Real      |

## Requirements

- Python >= 3.10
- NumPy >= 2.0
