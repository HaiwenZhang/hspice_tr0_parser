# Python API Documentation

This document covers the Python API for `hspicetr0parser`.

## Installation

```bash
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser
pip install maturin numpy
maturin develop --release
```

## API Reference

### `read(filename, debug=0)`

Read a waveform file and return a `WaveformResult` object.

```python
from hspice_tr0_parser import read

result = read('simulation.tr0')
print(result.title)        # Simulation title
print(result.date)         # Date string
print(result.analysis)     # 'transient', 'ac', 'dc', etc.
print(result.scale_name)   # 'TIME', 'HERTZ', etc.
```

### `convert_to_raw(input_path, output_path, debug=0)`

Convert HSPICE file to SPICE3 binary raw format.

```python
from hspice_tr0_parser import convert_to_raw

success = convert_to_raw('simulation.tr0', 'output.raw')
```

### `stream(filename, chunk_size=10000, signals=None, debug=0)`

Stream large files in chunks for memory efficiency.

```python
from hspice_tr0_parser import stream

for chunk in stream('large_file.tr0', chunk_size=50000):
    print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
    data = chunk['data']  # dict of signal_name -> numpy array
```

### `read_raw(filename, debug=0)`

Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).

```python
from hspice_tr0_parser import read_raw

result = read_raw('simulation.raw')
print(result.title)
print(result.analysis)  # 'transient', 'ac', 'dc'
time = result.get('time')
vout = result.get('v(out)')
```

## Classes

### `WaveformResult`

Main result class returned by `read()`.

**Attributes:**

- `title` (str): Simulation title
- `date` (str): Simulation date
- `analysis` (str): Analysis type (`'transient'`, `'ac'`, `'dc'`, `'operating'`, `'noise'`)
- `scale_name` (str): Scale variable name (`'TIME'`, `'HERTZ'`)
- `sweep_param` (str | None): Sweep parameter name
- `variables` (list[Variable]): List of variable definitions
- `tables` (list[DataTable]): Data tables (one per sweep point)

**Methods:**

- `get(name)`: Get signal data by name (returns NumPy array)
- `var_names()`: Get list of all variable names
- `num_vars()`: Number of variables
- `num_sweeps()`: Number of sweep points
- `has_sweep()`: Whether result has sweep data
- `__len__()`: Number of data points

### `Variable`

Variable/signal metadata.

**Attributes:**

- `name` (str): Variable name (e.g., `'TIME'`, `'v(out)'`)
- `var_type` (str): Variable type (`'time'`, `'voltage'`, `'current'`, `'frequency'`)

### `DataTable`

One data table per sweep point.

**Attributes:**

- `sweep_value` (float | None): Sweep value for this table

**Methods:**

- `get(name)`: Get signal data by name
- `keys()`: Get list of signal names

## Examples

### Basic Reading

```python
from hspice_tr0_parser import read

result = read('simulation.tr0')

print(f"Title: {result.title}")
print(f"Analysis: {result.analysis}")
print(f"Variables: {result.num_vars()}")
print(f"Points: {len(result)}")

# List all variables
for var in result.variables:
    print(f"  {var.name}: {var.var_type}")

# Get signal data
time = result.get('TIME')
vout = result.get('v(out)')
```

### Plotting

```python
from hspice_tr0_parser import read
import matplotlib.pyplot as plt

result = read('simulation.tr0')
time = result.get('TIME') * 1e9  # Convert to ns

plt.figure(figsize=(10, 6))
for var in result.variables:
    if var.var_type == 'voltage' and var.name != 'TIME':
        plt.plot(time, result.get(var.name), label=var.name)

plt.xlabel('Time (ns)')
plt.ylabel('Voltage (V)')
plt.legend()
plt.grid(True)
plt.show()
```

### Streaming Large Files

```python
from hspice_tr0_parser import stream
import numpy as np

all_time = []
all_vout = []

for chunk in stream('large_sim.tr0', chunk_size=100000):
    all_time.append(chunk['data']['TIME'])
    all_vout.append(chunk['data']['v(out)'])

time = np.concatenate(all_time)
vout = np.concatenate(all_vout)
print(f"Total points: {len(time)}")
```

### Working with Sweeps

```python
from hspice_tr0_parser import read

result = read('sweep.tr0')

if result.has_sweep():
    print(f"Sweep parameter: {result.sweep_param}")
    for i, table in enumerate(result.tables):
        print(f"  Sweep {i}: {table.sweep_value}")
```

### Converting to SPICE3

```python
from hspice_tr0_parser import convert_to_raw

if convert_to_raw('hspice.tr0', 'ngspice.raw'):
    print("Conversion successful!")
```

## Supported Formats

| Extension | Analysis  | Data Type |
| --------- | --------- | --------- |
| `.tr0`    | Transient | Real      |
| `.ac0`    | AC        | Complex   |
| `.sw0`    | DC Sweep  | Real      |

## Requirements

- Python >= 3.10
- NumPy >= 2.0
