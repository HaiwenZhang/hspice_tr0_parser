# HSPICE Binary Output File Format

This document describes the binary file format used by HSPICE for storing simulation results (`.tr0`, `.ac0`, `.sw0` files).

## Overview

HSPICE is a circuit simulator that produces output files containing voltage and current values from simulated circuits. When using `.option post=1`, HSPICE generates binary output files. The format version can be specified with `.option post_version=9601` or `.option post_version=2001`.

### Supported Analysis Types

| Extension           | Analysis Type      | Data Type           |
| ------------------- | ------------------ | ------------------- |
| `.tr0`, `.tr1`, ... | Transient Analysis | Real (float32)      |
| `.ac0`, `.ac1`, ... | AC Analysis        | Complex (2×float32) |
| `.sw0`, `.sw1`, ... | DC Sweep Analysis  | Real (float32)      |

### Format Versions

| Version | Description     | Data Width    |
| ------- | --------------- | ------------- |
| `9007`  | Legacy format   | 4-byte float  |
| `9601`  | Standard format | 4-byte float  |
| `2001`  | Extended format | 8-byte double |

## File Structure

The binary file consists of ordered blocks: a **header block** followed by multiple **data blocks**.

![Binary file structure](figures/file.png)
<sub>Figure 1: Overall binary file structure</sub>

```
┌─────────────────────────────────────────────────────┐
│                  Header Block                        │
│  (UTF-8 text metadata, terminated by $&%# marker)   │
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

Each block consists of three sections: a 16-byte **block head**, a variable-length **data section**, and a 4-byte **block tail**.

![Block structure](figures/Block.png)
<sub>Figure 2: Generic block structure</sub>

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Block Header (16 bytes)                       │
├────────────────┬────────────────┬────────────────┬───────────────────┤
│  Endian Check  │  Endian Check  │  Endian Check  │  Data Size (N)    │
│  0x00000004    │    Padding     │  0x00000004    │  (4 bytes)        │
│  (4 bytes)     │   (4 bytes)    │  (4 bytes)     │                   │
├────────────────┴────────────────┴────────────────┴───────────────────┤
│                         Data Section (N bytes)                        │
│                      (Header text or float data)                      │
├──────────────────────────────────────────────────────────────────────┤
│                        Block Tail (4 bytes)                           │
│                    (Same value as Data Size N)                        │
└──────────────────────────────────────────────────────────────────────┘
```

### Block Header Details

| Offset | Size (bytes) | Description                                                  |
| ------ | ------------ | ------------------------------------------------------------ |
| 0      | 4            | Endianness marker (`0x00000004` for LE, `0x04000000` for BE) |
| 4      | 4            | Padding / reserved                                           |
| 8      | 4            | Endianness marker (same as offset 0)                         |
| 12     | 4            | Number of data bytes in this block                           |

### Endianness Detection

The file's endianness is detected from the first block header:

```c
// C-style detection logic:
if (blockHeader[0] == 0x00000004 && blockHeader[2] == 0x00000004) {
    // Little-endian: no byte swap needed
    swap = 0;
} else if (blockHeader[0] == 0x04000000 && blockHeader[2] == 0x04000000) {
    // Big-endian: byte swap required
    swap = 1;
}
```

## Header Block

The header block is unique because its data section contains **UTF-8 plain text** instead of binary data. This metadata describes the simulation and signal names.

### Header String Structure

The header string consists of several concatenated substrings (no newline characters):

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Number String (20/24 chars)  │  *  │  Simulation Info  │  Signal Names  │
└──────────────────────────────────────────────────────────────────────────┘
```

### Header Field Positions

| Position (bytes) | Length | Content                                  |
| ---------------- | ------ | ---------------------------------------- |
| 0-3              | 4      | Number of variables (including scale)    |
| 4-7              | 4      | Number of probes                         |
| 8-11             | 4      | Number of sweeps (0 or 1)                |
| 16-19            | 4      | Post format identifier 1 (`9007`/`9601`) |
| 20-23            | 4      | Post format identifier 2 (`2001`)        |
| 24-87            | 64     | Simulation title / source filename       |
| 88-111           | 24     | Date and time string                     |
| 176-185          | 10     | Sweep size (format 9601)                 |
| 187-196          | 10     | Sweep size (format 2001)                 |
| 256+             | varies | Vector descriptions                      |

### Example Header

```
  00050000000100009601    * exampleFile.sp
  06/08/2020      14:04:30 Copyright (c) 1986 - 2020 by Synopsys, Inc. All Rights Reserved.
  10
  1       1       1       1       8
  TIME            v(0             v(vo            v(vs            i(vs            r1
  $&%#
```

<sub>Figure 3: Example header block content (line breaks added for readability, not present in actual file)</sub>

### Vector Description Section

Starting at byte 256, the vector description section contains:

```
<var_type> <internal_names...> <scale_name> <signal_names...> $&%#
```

Where:

- `var_type`: Variable type indicator
  - `1` = Time domain (real values)
  - `2` = Frequency domain (complex values)
- `internal_names`: Internal variable identifiers (same count as variables)
- `scale_name`: Independent variable name (e.g., "TIME", "HERTZ")
- `signal_names`: Human-readable signal names (e.g., "v(out)", "i(vdd)")
- `$&%#`: End-of-header marker

## Data Blocks

All blocks after the header contain simulation data. The data section stores floating-point values.

### Data Format by Version

| Version | Data Type | Bytes per Value | Example End Marker       |
| ------- | --------- | --------------- | ------------------------ |
| 9601    | float32   | 4               | `1.0000000150474662e+30` |
| 2001    | float64   | 8               | `1.0e+30`                |

### Data Interleaving Pattern

Signal values are **interleaved** rather than stored consecutively per signal. For each time point, all signal values are stored together:

```
TIME₀, Signal₀_t₀, Signal₁_t₀, Signal₂_t₀, ...
TIME₁, Signal₀_t₁, Signal₁_t₁, Signal₂_t₁, ...
TIME₂, Signal₀_t₂, Signal₁_t₂, Signal₂_t₂, ...
...
```

Example with 5 signals (TIME, v(0), v(vo), v(vs), i(vs)):

```
TIME_value, v_0_value, v_vo_value, v_vs_value, i_vs_value,
TIME_value, v_0_value, v_vo_value, v_vs_value, i_vs_value,
TIME_value, ...
```

### Complex Data (AC Analysis)

For AC analysis, each signal (except time/frequency) consists of two float values:

```
FREQ₀, Real(Signal₀)_f₀, Imag(Signal₀)_f₀, Real(Signal₁)_f₀, Imag(Signal₁)_f₀, ...
```

### End-of-Data Marker

The last data block contains a special marker value to indicate end of data:

| Version | Marker Value                                      |
| ------- | ------------------------------------------------- |
| 9601    | `> 9e29` (approximately `1.0000000150474662e+30`) |
| 2001    | `1.0e+30`                                         |

## Sweep Support

HSPICE supports parameter sweeps where a simulation is repeated for different parameter values. Setting `.alter` statements produces additional output files (one per alter).

### Single Sweep (num_sweeps = 1)

When a sweep is present, each data table is prefixed with the sweep parameter value:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Sweep_value₀ │ TIME₀ Sig₀_t₀ ... │ TIME₁ Sig₀_t₁ ... │ END_MARKER │
├─────────────────────────────────────────────────────────────────────┤
│ Sweep_value₁ │ TIME₀ Sig₀_t₀ ... │ TIME₁ Sig₀_t₁ ... │ END_MARKER │
├─────────────────────────────────────────────────────────────────────┤
│                              ...                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### No Sweep (num_sweeps = 0)

Data begins immediately without sweep value prefix:

```
┌─────────────────────────────────────────────────────────────────────┐
│ TIME₀ Sig₀_t₀ Sig₁_t₀ ... │ TIME₁ Sig₀_t₁ Sig₁_t₁ ... │ END_MARKER │
└─────────────────────────────────────────────────────────────────────┘
```

## ASCII Format

Setting `.option post=2` produces ASCII output instead of binary.

![ASCII file structure](figures/ASCII_File.png)
<sub>Figure 4: Overall ASCII file structure</sub>

### ASCII Format Details

| Version | Value Format    | String Length | Terminator        |
| ------- | --------------- | ------------- | ----------------- |
| 9601    | `1.23456E±78`   | 11 characters | `0.10000E+31\n`   |
| 2001    | `1.2345678E±90` | 13 characters | `0.1000000E+31\n` |

Key differences from binary format:

- No block head or block tail
- Values are ASCII strings in scientific notation
- No separators between values
- Sweep terminator followed by newline character

## Parsing Algorithm Summary

```
1. Open file in binary mode
2. Read first block header (16 bytes)
3. Detect endianness from bytes 0-3 and 8-11
4. Read header data section until "$&%#" marker
5. Parse header: extract signal count, names, format version
6. Loop: Read data blocks
   a. Read block header (16 bytes)
   b. Read N bytes of float data
   c. Read block tail (4 bytes), verify matches header
   d. If last value > 9e29, end of current sweep
7. Convert interleaved data to per-signal arrays
```

## Limitations

This format description covers:

- ✅ Binary format (`.option post=1`)
- ✅ ASCII format (`.option post=2`)
- ✅ Transient analysis (`.tr0`)
- ✅ AC analysis (`.ac0`)
- ✅ DC sweep analysis (`.sw0`)
- ✅ Single-dimensional sweeps
- ⚠️ Multi-dimensional sweeps (`.alter`) produce separate files

## References

- HSPICE User Documentation (Synopsys)
- [hspicefile](https://pypi.org/project/hspicefile) - Python HSPICE file reader
- [PyOPUS](https://fides.fe.uni-lj.si/pyopus/) - Python-based optimization framework
- [hspiceParser](https://github.com/HMC-ACE/hspiceParser) - Python HSPICE parser
- [Gaw Data File Formats](https://www.rvq.fr/linux/gawfmt.php) - Additional format documentation
