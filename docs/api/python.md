# Python API Documentation

This document covers the Python API for `hspicetr0parser`.

`hspicetr0parser` is the native PyO3 extension. The repository also ships
`hspice_tr0_parser.py`, a compatibility wrapper that keeps the deprecated
`debug` keyword and returns an empty iterator when streaming cannot open or
parse a file.

## Installation

Download the `cp312-abi3` wheel matching the host from a GitHub Release. The
same wheel works with CPython 3.12, 3.13, and 3.14:

```bash
python -m pip install ./hspicetr0parser-1.5.0-cp312-abi3-<platform>.whl
```

To build from source instead:

```bash
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser
pip install maturin numpy
maturin develop --release
```

## Logging

The library uses structured logging via `tracing`. To enable log output, call `init_logging()` before using other functions:

```python
import hspicetr0parser

# Initialize logging with desired level
# Levels: "trace", "debug", "info", "warn", "error"
hspicetr0parser.init_logging("info")

# Now all operations will output logs
result = hspicetr0parser.read("simulation.tr0")
```

If `init_logging()` is not called, library tracing output is disabled.

### Log Levels

| Level   | Description                                                |
| ------- | ---------------------------------------------------------- |
| `trace` | Most verbose, includes per-chunk and per-sweep details     |
| `debug` | Detailed info: file sizes, data block statistics           |
| `info`  | Key operations: file open, parse complete, conversion done |
| `warn`  | Warnings only                                              |
| `error` | Error events                                               |

## API Reference

### `init_logging(level="info")`

Initialize the logging subsystem. Call once at application startup.

```python
import hspicetr0parser

hspicetr0parser.init_logging("debug")  # Enable debug logging
```

### `read(filename)`

Read an HSPICE binary waveform file and return a `WaveformResult` object, or
`None` if reading fails.

```python
from hspicetr0parser import read

result = read('simulation.tr0')
print(result.title)        # Simulation title
print(result.date)         # Date string
print(result.analysis)     # 'transient', 'ac', 'dc', etc.
print(result.scale_name)   # 'TIME', 'HERTZ', etc.
```

### `convert_to_raw(input_path, output_path)`

Convert HSPICE file to SPICE3 binary raw format.

```python
from hspicetr0parser import convert_to_raw

success = convert_to_raw('simulation.tr0', 'output.raw')
```

### `convert_raw_to_hspice(input_path, output_path, post_version="9601")`

Convert a SPICE3/ngspice binary or ASCII raw file to HSPICE. Use `.trN` for
transient, `.acN` for AC, and `.swN` for DC output. Version `9601` is the
compatibility-oriented default and stores `float32`; version `2001` stores
`float64`.

```python
from hspicetr0parser import convert_raw_to_hspice

success = convert_raw_to_hspice('simulation.raw', 'simulation.tr0')
```

The function returns `True` on success and `False` for parse, validation, or
write failures. An unsupported `post_version` raises `RuntimeError`.

The writer supports transient, AC, and DC analyses. The output extension must
match the raw file's analysis, and all vectors must be non-empty, equal length,
finite, and representable by the selected precision.

### `stream(filename, chunk_size=10000, signals=None)`

Stream large files in chunks for memory efficiency.

```python
from hspicetr0parser import stream

for chunk in stream('large_file.tr0', chunk_size=50000):
    print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
    data = chunk['data']  # dict of signal_name -> numpy array
```

The scale vector is always returned even when `signals` filters dependent
signals. `chunk_size` is a minimum target because complete HSPICE records are
consumed. The native extension raises `RuntimeError` on open/header errors;
`hspice_tr0_parser.stream()` converts that failure into an empty iterator.
Streaming currently stops at the first data-table end marker, so use `read()`
to retrieve every table in a parameter sweep.

### `read_raw(filename)`

Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).

```python
from hspicetr0parser import read_raw

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

- `name` (str): Variable name as stored (e.g., `'TIME'`, `'v(out'` for HSPICE)
- `var_type` (str): Variable type (`'time'`, `'voltage'`, `'current'`, `'frequency'`)

### `DataTable`

One data table per sweep point.

**Attributes:**

- `sweep_value` (float | None): Sweep value for this table

**Methods:**

- `get(name)`: Get signal data by name
- `keys()`: Get list of signal names

## Examples

### Basic Reading with Logging

```python
import hspicetr0parser

# Enable info-level logging to see progress
hspicetr0parser.init_logging("info")

result = hspicetr0parser.read('simulation.tr0')

print(f"Title: {result.title}")
print(f"Analysis: {result.analysis}")
print(f"Variables: {result.num_vars()}")
print(f"Points: {len(result)}")

# List all variables
for var in result.variables:
    print(f"  {var.name}: {var.var_type}")

# Get signal data
time = result.get('TIME')
vout = result.get('v(out')
```

### Plotting

```python
import hspicetr0parser
import matplotlib.pyplot as plt

result = hspicetr0parser.read('simulation.tr0')
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
import hspicetr0parser
import numpy as np

# Enable trace logging for detailed chunk info
hspicetr0parser.init_logging("trace")

all_time = []
all_vout = []

for chunk in hspicetr0parser.stream('large_sim.tr0', chunk_size=100000):
    all_time.append(chunk['data']['TIME'])
    all_vout.append(chunk['data']['v(out'])

time = np.concatenate(all_time)
vout = np.concatenate(all_vout)
print(f"Total points: {len(time)}")
```

### Working with Sweeps

```python
import hspicetr0parser

result = hspicetr0parser.read('sweep.tr0')

if result.has_sweep():
    print(f"Sweep parameter: {result.sweep_param}")
    for i, table in enumerate(result.tables):
        print(f"  Sweep {i}: {table.sweep_value}")
```

### Converting to SPICE3

```python
import hspicetr0parser

# Enable logging to see conversion progress
hspicetr0parser.init_logging("info")

if hspicetr0parser.convert_to_raw('hspice.tr0', 'ngspice.raw'):
    print("Conversion successful!")
```

### Converting SPICE3 Raw to HSPICE

```python
import hspicetr0parser

if hspicetr0parser.convert_raw_to_hspice(
    'ngspice.raw',
    'hspice.tr0',
    post_version='2001',
):
    print("Conversion successful!")
```

## Supported Formats

| Format | Read | Write |
| ------ | ---- | ----- |
| HSPICE 9007/9601 binary (`.trN`, `.acN`, `.swN`) | Yes | 9601 output |
| HSPICE 2001 binary (`.trN`, `.acN`, `.swN`) | Yes | Yes |
| SPICE3 raw binary | Yes | Yes, from HSPICE via `convert_to_raw` |
| SPICE3 raw ASCII | Yes | No |

## Requirements

- Python 3.12, 3.13, or 3.14
- NumPy >= 2.0

## Logging Migration

The native extension does not accept the old `debug` parameter. Use
`init_logging()` instead:

```python
# Old (v1.3.x)
result = read('file.tr0', debug=1)

# New (v1.4.0+)
init_logging("info")
result = read('file.tr0')
```

Code that still imports `hspice_tr0_parser` can temporarily use the deprecated
`debug` keyword because the compatibility wrapper translates it to
`init_logging()`.
