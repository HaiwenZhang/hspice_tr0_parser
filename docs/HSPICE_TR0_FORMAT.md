# HSPICE TR0 Binary File Format

This document describes the binary file format used by HSPICE for storing transient analysis results (`.tr0` files).

## Overview

HSPICE writes simulation results in a proprietary binary format. The `.tr0` extension is used for transient analysis output. The file consists of:

1. **Header Blocks** - Metadata and variable descriptions
2. **Data Blocks** - Simulation waveform data

## File Structure

```
┌─────────────────────────────────────────────────────┐
│                  Header Blocks                       │
│  (Variable-length, terminated by $&%# marker)       │
├─────────────────────────────────────────────────────┤
│                  Data Block 1                        │
├─────────────────────────────────────────────────────┤
│                  Data Block 2                        │
├─────────────────────────────────────────────────────┤
│                      ...                             │
├─────────────────────────────────────────────────────┤
│         Data Block N (ends with >9e29 marker)       │
└─────────────────────────────────────────────────────┘
```

## Block Structure

Each block in the file follows this structure:

```
┌──────────────────┬─────────────────────┬──────────────────┐
│   Block Header   │      Block Data     │   Block Trailer  │
│   (8-12 bytes)   │  (variable length)  │    (4 bytes)     │
└──────────────────┴─────────────────────┴──────────────────┘
```

### Block Header

The block header contains:

| Offset | Size (bytes) | Description                           |
| ------ | ------------ | ------------------------------------- |
| 0      | 4            | Number of items in this block         |
| 4      | 4            | Trailer value (used for verification) |

### Endianness Detection

The file's endianness is detected from the first block header:

- If num_items > 8192 when read as little-endian → use big-endian
- Otherwise → use little-endian

## Header Section

The header blocks contain metadata encoded as text within binary blocks. Key positions:

| Position (bytes) | Length | Content                  |
| ---------------- | ------ | ------------------------ |
| 0-3              | 4      | Number of variables      |
| 4-7              | 4      | Number of probes         |
| 8-11             | 4      | Number of sweeps         |
| 16-19            | 4      | Post format identifier 1 |
| 20-23            | 4      | Post format identifier 2 |
| 24-87            | 64     | Simulation title         |
| 88-111           | 24     | Date string              |
| 176-185          | 10     | Sweep size (format 1)    |
| 187-196          | 10     | Sweep size (format 2)    |
| 256+             | varies | Vector descriptions      |

### Post Format Identifiers

The post format version is identified by magic strings:

| String | Description                 |
| ------ | --------------------------- |
| `9007` | Post format version 1       |
| `9601` | Post format version 1 (alt) |
| `2001` | Post format version 2       |

### Vector Description Section

Starting at byte 256, the vector description section contains:

```
<var_type> <name_0> <name_1> ... <name_n> <scale_name> <desc_0> <desc_1> ...
```

Where:

- `var_type`: 1 = time domain (real), 2 = frequency domain (complex)
- `name_x`: Internal variable names
- `scale_name`: Independent variable name (typically "TIME")
- `desc_x`: Human-readable signal descriptions (e.g., "v(out)")

## Data Section

### Data Types

| Variable Type | ID  | Data Format                     |
| ------------- | --- | ------------------------------- |
| Real          | 1   | 32-bit IEEE 754 floating point  |
| Complex       | 2   | Two 32-bit floats (real + imag) |

### Data Block Format

Each data block contains:

1. **Block Header**: 8 bytes (num_items, trailer)
2. **Float Data**: `num_items × 4` bytes of 32-bit floats
3. **Block Trailer**: 4 bytes (matches header trailer value)

### Data Organization

For each sweep point:

```
┌────────────────────────────────────────────────────────────┐
│ [sweep_value] time_0 sig0_0 sig1_0 ... time_1 sig0_1 ...  │
└────────────────────────────────────────────────────────────┘
```

- If sweep is present: first value is the sweep parameter value
- Data is organized row-major: all signals for time point 0, then time point 1, etc.
- For complex data: each signal value consists of (real, imag) pairs

### End-of-Data Marker

The last data block contains a special marker value `> 9e29` (approximately 1e30) to indicate end of data.

## Sweep Support

HSPICE supports parameter sweeps where a simulation is repeated for different parameter values.

### Single Sweep

When a sweep is present:

- `num_sweeps = 1` in header
- Each data table is preceded by the sweep parameter value
- The sweep parameter name is found after vector descriptions

### No Sweep

When no sweep is present:

- `num_sweeps = 0` in header
- Data begins immediately without sweep value prefix

## Example Header Parse

```
Bytes [0:4]   = "   4"     → 4 variables
Bytes [4:8]   = "  15"     → 15 probes
Bytes [8:12]  = "   0"     → 0 sweeps
Bytes [16:20] = "9007"     → Post version 1
Bytes [24:88] = "my_sim"   → Title
Bytes [88:112] = "Dec 2025" → Date
Bytes [256:]  = "1 time..."  → Variable descriptions
```

## Limitations

This format description covers:

- ✅ Transient analysis (.tr0)
- ✅ Binary format only (ASCII not supported)
- ✅ Single-dimensional sweeps
- ⚠️ Complex data (AC analysis) is supported but less common

## References

- HSPICE User Documentation (Synopsys)
