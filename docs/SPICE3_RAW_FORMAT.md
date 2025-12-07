# SPICE3 Binary Raw File Format

This document describes the SPICE3 binary raw file format (`.raw`), commonly used by ngspice, LTspice, and other SPICE-compatible simulators.

## Overview

The SPICE3 raw format is a flexible format for storing simulation results. It consists of:

1. **Text Header** - Human-readable metadata
2. **Binary Data** - Simulation waveforms in binary format

This format is widely supported and serves as an interchange format between different SPICE tools.

## File Structure

```
┌─────────────────────────────────────────────────────┐
│                   Text Header                        │
│              (ASCII, line-separated)                 │
├─────────────────────────────────────────────────────┤
│                 "Binary:" marker                     │
├─────────────────────────────────────────────────────┤
│                   Binary Data                        │
│              (IEEE 754 floats)                       │
└─────────────────────────────────────────────────────┘
```

## Text Header

The header consists of key-value pairs, one per line:

```
Title: <simulation title>
Date: <date string>
Plotname: <analysis type>
Flags: <data type flags>
No. Variables: <count>
No. Points: <count>
Variables:
    <index>    <name>    <type>
    ...
Binary:
```

### Header Fields

| Field           | Required | Description                                 |
| --------------- | -------- | ------------------------------------------- |
| `Title`         | Yes      | Simulation title or description             |
| `Date`          | Yes      | Date/time stamp of simulation               |
| `Plotname`      | Yes      | Type of analysis (see below)                |
| `Flags`         | Yes      | Data format: `real` or `complex`            |
| `No. Variables` | Yes      | Total number of variables (including scale) |
| `No. Points`    | Yes      | Number of data points per variable          |
| `Variables`     | Yes      | Variable definitions (see below)            |
| `Command`       | No       | Original SPICE command                      |
| `Option`        | No       | Simulation options                          |

### Plotname Values

| Plotname                 | Description                    |
| ------------------------ | ------------------------------ |
| `Transient Analysis`     | Time-domain simulation (.TRAN) |
| `AC Analysis`            | Frequency sweep (.AC)          |
| `DC Analysis`            | DC operating point (.DC)       |
| `DC transfer curve`      | DC sweep simulation            |
| `Operating Point`        | DC operating point (.OP)       |
| `Noise Spectral Density` | Noise analysis (.NOISE)        |

### Flags Field

| Flag      | Description                         |
| --------- | ----------------------------------- |
| `real`    | All data is real-valued (64-bit)    |
| `complex` | Data is complex-valued (2×64-bit)   |
| `padded`  | Data may have padding (rarely used) |

### Variable Definitions

After the `Variables:` line, each variable is defined on its own line:

```
    <index>    <name>    <type>
```

| Field     | Description                            |
| --------- | -------------------------------------- |
| `<index>` | 0-based variable index                 |
| `<name>`  | Variable name (e.g., `time`, `v(out)`) |
| `<type>`  | Variable type (see below)              |

### Variable Types

| Type        | Description             |
| ----------- | ----------------------- |
| `time`      | Time variable (seconds) |
| `frequency` | Frequency variable (Hz) |
| `voltage`   | Node voltage (V)        |
| `current`   | Branch current (A)      |
| `device`    | Device parameter        |

**Note**: The first variable (index 0) is always the scale variable (time, frequency, etc.).

## Binary Data Section

### Binary Marker

The binary data section begins immediately after the line containing only `Binary:`.

### Data Format

#### Real Data (`Flags: real`)

Data is stored as 64-bit IEEE 754 double-precision floats in **little-endian** byte order.

For each data point, values are stored in variable order:

```
Point 0: [var_0] [var_1] [var_2] ... [var_n-1]
Point 1: [var_0] [var_1] [var_2] ... [var_n-1]
...
Point m-1: [var_0] [var_1] [var_2] ... [var_n-1]
```

Where:

- `n` = Number of variables
- `m` = Number of points
- Each value is 8 bytes (64-bit double)

**Total binary size**: `n × m × 8` bytes

#### Complex Data (`Flags: complex`)

For complex data, each value consists of two 64-bit doubles:

```
[real_part][imaginary_part]
```

**Total binary size**: `n × m × 16` bytes

### Byte Order

SPICE3 raw files use **little-endian** byte order for binary data.

## Complete Example

### Header Example

```
Title: RC Low-pass Filter Simulation
Date: Sat Dec  7 23:30:00 2025
Plotname: Transient Analysis
Flags: real
No. Variables: 3
No. Points: 1001
Variables:
	0	time	time
	1	v(out)	voltage
	2	i(r1)	current
Binary:
<binary data follows>
```

### Parsing Pseudocode

```python
def parse_raw_file(filename):
    with open(filename, 'rb') as f:
        # Read header until "Binary:" line
        header = {}
        variables = []

        while True:
            line = read_line(f)
            if line.startswith("Binary:"):
                break

            if line.startswith("Title:"):
                header['title'] = line[6:].strip()
            elif line.startswith("Variables:"):
                # Read variable definitions
                pass
            # ... parse other fields

        # Read binary data
        num_vars = header['num_variables']
        num_points = header['num_points']
        is_complex = header['flags'] == 'complex'

        bytes_per_value = 16 if is_complex else 8
        data = np.zeros((num_points, num_vars))

        for point in range(num_points):
            for var in range(num_vars):
                value = struct.unpack('<d', f.read(8))[0]
                data[point, var] = value

        return header, variables, data
```

## Compatibility Notes

### ngspice

ngspice uses this format as its native output format. Files can be viewed with:

```bash
ngspice -b circuit.cir -r output.raw
```

### LTspice

LTspice also uses a compatible raw format but may include additional header fields like `Offset` and `Scale`.

### This Library's Output

The `hspice_tr0_to_raw` function in this library generates SPICE3-compatible files with:

- `Flags: real` (complex data is converted to magnitude)
- Little-endian byte order
- 64-bit double precision for all values

## Data Type Summary

| Data Type        | Size (bytes) | Format                |
| ---------------- | ------------ | --------------------- |
| Scale (real)     | 8            | IEEE 754 double (f64) |
| Signal (real)    | 8            | IEEE 754 double (f64) |
| Scale (complex)  | 16           | 2× IEEE 754 double    |
| Signal (complex) | 16           | 2× IEEE 754 double    |

## References

- [ngspice Manual - Raw File Format](http://ngspice.sourceforge.net/docs/ngspice-manual.pdf)
- [SPICE3 Source Code](https://embedded.eecs.berkeley.edu/pubs/downloads/spice/)
- [LTspice Raw File Format Notes](https://www.analog.com/en/design-center/design-tools-and-calculators/ltspice-simulator.html)
