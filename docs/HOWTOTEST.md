# How to Test

This repository has Rust unit/integration tests and Python integration tests.
The committed files in `example/` are required for the format and reference
comparisons.

CI runs the Python suite on Python 3.12, 3.13, and 3.14 across Linux, macOS,
and Windows.

## Prerequisites

- Rust stable toolchain
- Python 3.12, 3.13, or 3.14
- A virtual environment with `maturin`, NumPy, and pytest

## Set Up the Python Environment

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install maturin numpy pytest
maturin develop --release
```

On Windows, activate the environment with
`.venv\Scripts\activate` before running `maturin` or pytest.

## Run the Suites

Run the core Rust tests while iterating on parser or writer code:

```bash
cargo test -p hspice-core
```

Run every Rust crate before submitting a workspace-wide change:

```bash
cargo test --workspace
```

Run the Python API and compatibility-wrapper tests after building the native
extension:

```bash
python -m pytest tests -v
```

The Python suite is split by responsibility:

```text
tests/
├── conftest.py        # paths, fixtures, and compatibility helpers
├── test_read.py       # basic API and error behavior
├── test_formats.py    # TR0/AC0/SW0 and 9601/2001 coverage
├── test_reference.py  # comparison with committed reference data
├── test_convert.py    # HSPICE-to-SPICE3 conversion
└── test_stream.py     # chunks, filters, formats, and error behavior
```

## Focused Commands

```bash
# Rust raw-to-HSPICE reference round trips
cargo test -p hspice-core test_raw_to_hspice_round_trip -- --exact

# All HSPICE writer unit tests
cargo test -p hspice-core hspice_writer::tests

# Python conversion tests
python -m pytest tests/test_convert.py -v

# Python streaming tests
python -m pytest tests/test_stream.py -v

# One Python test
python -m pytest \
  tests/test_stream.py::TestStreamingErrorHandling::test_nonexistent_file -v
```

The core round-trip integration test converts each of these reference files to
SPICE3 raw and back, then compares the generated HSPICE bytes with the source:

- `example/test_9601.tr0`
- `example/test_2001.tr0`
- `example/test_9601.ac0`
- `example/test_9601.sw0`

The writer unit tests additionally cover header layout, 8192-byte record
splitting, AC complex values, parameter-sweep tables, and rejected transient
complex values.

## CLI Smoke Tests

```bash
cargo run -p hspice-cli -- info example/test_9601.tr0
cargo run -p hspice-cli -- read example/test_9601.ac0 --json
cargo run -p hspice-cli -- scan example/PinToPinSim.tr0
```

For a conversion smoke test, use a temporary directory so generated files do
not alter the repository:

```bash
tmp_dir="$(mktemp -d)"
cargo run -p hspice-cli -- convert \
  example/test_9601.tr0 "$tmp_dir/test.raw"
cargo run -p hspice-cli -- convert \
  "$tmp_dir/test.raw" "$tmp_dir/test.tr0" --post-version 9601
cmp example/test_9601.tr0 "$tmp_dir/test.tr0"
```

## Formatting and Lints

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Troubleshooting

### `ModuleNotFoundError: No module named 'hspicetr0parser'`

The PyO3 extension has not been installed into the active environment. Activate
the virtual environment and run:

```bash
maturin develop --release
```

### Tests are skipped because fixtures are missing

Confirm that the binary fixtures and `.pickle` reference files are present in
`example/`. The tests intentionally skip fixture-dependent cases when those
files are absent.

### Python code does not reflect recent Rust changes

Re-run `maturin develop --release`; an already installed extension is not
rebuilt automatically when Rust source changes.
