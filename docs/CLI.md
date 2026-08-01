# Command-Line Interface

`hspice-cli` provides native inspection, conversion, streaming, checksum, and
CSV-export workflows without a Python runtime.

## Download

Each `v*` GitHub Release contains ready-to-run archives for Linux x86_64 and
aarch64, macOS Intel and Apple Silicon, and Windows x86_64. Select the archive
whose Rust target triple matches the host, extract it, and verify it against
the release's `SHA256SUMS` file.

## Build and Help

```bash
cargo build -p hspice-cli --release
./target/release/hspice-cli --help
./target/release/hspice-cli <COMMAND> --help
```

Use the global `--log-level trace|debug|info|warn|error` option to control
diagnostics. Logs and completion messages go to stderr so stdout remains usable
for JSON, JSON Lines, signal values, or CSV pipelines.

## Commands

| Command | Purpose |
| ------- | ------- |
| `info <FILE>` | Parse only an HSPICE header and print metadata |
| `read <FILE>` | Read a complete HSPICE file |
| `read-raw <FILE>` | Read a complete binary or ASCII SPICE3 raw file |
| `convert <INPUT> <OUTPUT>` | Convert HSPICE to SPICE3 raw or raw to HSPICE, based on extensions |
| `stream <FILE>` | Emit HSPICE chunks as JSON Lines |
| `scan <FILE>` | Decode HSPICE with bounded memory and print a checksum summary |
| `export <FILE>` | Export HSPICE or raw data as CSV |

All commands exit with status 0 on success and 1 when parsing, validation,
serialization, or file I/O fails.

## Inspect and Read

```bash
hspice-cli info simulation.tr0
hspice-cli read simulation.tr0
hspice-cli read simulation.tr0 --json
hspice-cli read simulation.tr0 --signal "v(out"
hspice-cli read-raw simulation.raw --json
```

`info` is HSPICE-only and does not decode waveform samples. `read` and
`read-raw` print a summary by default. `--json` prints structured metadata and
table summaries, not every sample. `--signal NAME` prints samples from the
first table, one real value per line or `real,imag` for a complex signal, and
takes precedence over `--json`.

## Convert Between HSPICE and SPICE3

The input and output extensions select the direction:

```bash
# HSPICE -> SPICE3 binary raw
hspice-cli convert simulation.tr0 simulation.raw

# SPICE3 binary or ASCII raw -> HSPICE 9601 (default)
hspice-cli convert simulation.raw simulation.tr0

# SPICE3 raw -> HSPICE 2001
hspice-cli convert simulation.raw simulation.tr0 --post-version 2001
```

Accepted HSPICE extensions are `.trN`, `.acN`, and `.swN`, where `N` is one or
more digits. Extension matching is case-insensitive. The raw analysis must
match the HSPICE output extension:

| Analysis | Required output extension |
| -------- | ------------------------- |
| Transient | `.trN` |
| AC | `.acN` |
| DC | `.swN` |

`--post-version` accepts `9601` or `2001` and is used only when the output is
HSPICE. Version 9601 stores `f32` values and is the compatibility-oriented
default; version 2001 stores `f64` values. Operating-point, noise, and unknown
raw analyses cannot currently be written to HSPICE.

HSPICE-to-raw conversion writes the first data table. Raw-to-HSPICE conversion
validates names, vector lengths, finite/representable values, and analysis
compatibility before writing.

## Stream JSON Lines

```bash
hspice-cli stream large.tr0 --chunk-size 50000
hspice-cli stream large.tr0 \
  --signal TIME --signal "v(out" --chunk-size 50000
```

Each stdout line is one JSON object:

```json
{"chunk_index":0,"time_range":[0.0,1e-6],"data":{"TIME":[0.0,1e-9],"v(out":[0.0,0.1]}}
```

Complex vectors are arrays of `[real, imaginary]` pairs. The scale is always
included even when dependent signals are filtered. `--chunk-size` is a minimum
target because the reader consumes complete HSPICE records. The current stream
and scan implementations stop at the first data-table end marker; use `read`
or `export` when every table in a parameter sweep is required.

## Scan with Bounded Memory

```bash
hspice-cli scan large.tr0
hspice-cli scan large.tr0 --chunk-size 250000
```

`scan` reports signal, chunk, and point counts; the first and last scale values;
and a compact floating-point checksum. It is useful for validating that a
large file can be decoded without printing its samples.

## Export CSV

```bash
# Auto-detect HSPICE versus .raw from the extension; write to stdout
hspice-cli export simulation.tr0

# Select signals and write a file
hspice-cli export simulation.tr0 \
  --signal TIME --signal "v(out" --output output.csv

# Force a parser and choose a delimiter
hspice-cli export waveform.dat --format raw --delimiter ';' -o output.csv
```

`--format` accepts `auto`, `hspice`, or `raw`. Auto mode selects the raw parser
only for a `.raw` extension and uses HSPICE otherwise. Complex columns are
expanded to `<name>.re` and `<name>.im`. Parameter-sweep output includes a
leading sweep column and writes all tables.
