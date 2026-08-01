# SPICE3 Raw File Format

This document describes the SPICE3/ngspice raw subset read and written by this
repository. Both binary and ASCII (`Values:`) input are accepted; generated raw
files are binary.

## File Structure

A raw file starts with a line-oriented text header and ends with either binary
or ASCII waveform values:

```text
Title: <simulation title>
Date: <date string>
Plotname: <analysis type>
Flags: real | complex
No. Variables: <count>
No. Points: <count>
Variables:
    <index>    <name>    <type>
    ...
Binary:
<little-endian binary values>
```

For ASCII data, `Values:` replaces `Binary:` and the samples follow as text.

## Header Fields

| Field | Used by the parser | Description |
| ----- | ------------------ | ----------- |
| `Title` | Yes | Simulation title |
| `Date` | Yes | Date/time text |
| `Plotname` | Yes | Used to infer the analysis |
| `Flags` | Yes | A `complex` token selects complex decoding; otherwise values are real |
| `No. Variables` | Yes | Number of vectors including the scale |
| `No. Points` | Yes | Number of points per vector |
| `Variables` | Yes | Indexed name/type definitions |
| `Command`, `Option`, and other fields | Ignored | May be present in third-party files |

Recognized plot names include transient, AC, DC, operating-point, and noise
variants. Variable types recognized by the shared model are `time`,
`frequency`, `voltage`, and `current`; other values become `unknown`.

The first variable is the scale vector, normally `time`, `frequency`, or a DC
sweep parameter.

## Binary Data

The parser expects little-endian IEEE 754 `f64` values interleaved by point.

For `Flags: real`:

```text
point 0: var0, var1, ... varN
point 1: var0, var1, ... varN
...
```

Each scalar occupies 8 bytes, so the data section size is:

```text
number_of_points × number_of_variables × 8
```

For `Flags: complex`, every variable—including the scale—is stored as a pair:

```text
real(var0), imag(var0), real(var1), imag(var1), ...
```

Each complex value occupies 16 bytes. When the library writes a complex raw
file from a real scale vector, the scale's imaginary component is `0.0`.

## ASCII Data

The parser recognizes a `Values:` section and accepts indexed SPICE3-style
rows. Real values are parsed as `f64`. Complex values can use `real,imag`,
`(real,imag)`, or a single real value (which implies a zero imaginary part).

The library does not currently write ASCII raw files.

## Library APIs

### Read raw data

```rust
let result = hspice_core::read_raw("simulation.raw")?;
let result_from_memory = hspice_core::read_raw_bytes(&bytes)?;
```

Python and CLI equivalents are:

```python
result = hspicetr0parser.read_raw("simulation.raw")
```

```bash
hspice-cli read-raw simulation.raw --json
```

### Write or convert to raw

`write_spice3_raw()` writes a `WaveformResult` as binary raw. The convenience
function `read_and_convert()` parses HSPICE and writes raw:

```rust
use hspice_core::{read_and_convert, write_spice3_raw};

read_and_convert("simulation.tr0", "simulation.raw")?;

let waveform = hspice_core::read("simulation.ac0")?;
write_spice3_raw(&waveform, "simulation.raw")?;
```

The writer preserves complex real and imaginary components. If any vector in
the selected table is complex, the output uses `Flags: complex` and represents
real vectors with a zero imaginary component.

Only the first `WaveformResult` data table is written to a raw file. Therefore,
converting an HSPICE parameter sweep with multiple tables does not preserve the
additional tables in the SPICE3 output.

### Convert raw to HSPICE

Raw-to-HSPICE conversion is available through the Rust, Python, and CLI APIs:

```rust
use hspice_core::{convert_raw_to_hspice, PostVersion};

convert_raw_to_hspice(
    "simulation.raw",
    "simulation.tr0",
    PostVersion::V9601,
)?;
```

```python
import hspicetr0parser

ok = hspicetr0parser.convert_raw_to_hspice(
    "simulation.raw", "simulation.tr0", "9601"
)
```

```bash
hspice-cli convert simulation.raw simulation.tr0
hspice-cli convert simulation.raw simulation.tr0 --post-version 2001
```

The raw plot analysis and HSPICE output extension must agree:

| Raw analysis | HSPICE extension |
| ------------ | ---------------- |
| Transient | `.trN` (commonly `.tr0`) |
| AC | `.acN` (commonly `.ac0`) |
| DC | `.swN` (commonly `.sw0`) |

Operating-point, noise, and unknown analyses cannot currently be written as
HSPICE. HSPICE output also requires non-empty, equal-length vectors, finite
values, and variable names that can be normalized to non-empty ASCII names
without whitespace. See [HSPICE_TR0_FORMAT.md](HSPICE_TR0_FORMAT.md) for the
record layout and writer constraints.

## Generated Raw Compatibility

Files produced by this repository have these properties:

- `Binary:` data section
- little-endian `f64` storage
- `Flags: real` or `Flags: complex`, based on the table vectors
- one plot and one data table
- standard variable index, name, and type lines

These choices match the subset consumed by this repository's parser and the
SPICE3/ngspice interchange workflow covered by the round-trip tests.
