# How to Test

This document describes how to set up and run the test suite for the HSPICE TR0 Parser.

## Prerequisites

- Python >= 3.10
- Rust toolchain (for building the native extension)
- The example test file `example/PinToPinSim.tr0`

## Quick Start

```bash
# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or: .venv\Scripts\activate  # Windows

# Install dependencies
pip install maturin numpy pytest

# Build the native extension
maturin develop --release

# Run all tests
pytest tests/ -v
```

## Running Tests

### Run All Tests

```bash
pytest tests/ -v
```

### Run Specific Test Class

```bash
# Test reading functionality
pytest tests/test_tr0_parser.py::TestHspiceTr0Read -v

# Test conversion functionality
pytest tests/test_tr0_parser.py::TestHspiceTr0ToRaw -v

# Test data integrity
pytest tests/test_tr0_parser.py::TestDataIntegrity -v

# Test edge cases
pytest tests/test_tr0_parser.py::TestEdgeCases -v
```

### Run Specific Test

```bash
pytest tests/test_tr0_parser.py::TestHspiceTr0Read::test_read_tr0_file -v
```

### Run with Coverage Report

```bash
pip install pytest-cov
pytest tests/ -v --cov=hspice_tr0_parser --cov-report=html
```

## Test Structure

```
tests/
├── __init__.py           # Test package initialization
├── conftest.py           # Shared pytest fixtures
└── test_tr0_parser.py    # Main test module
```

## Test Cases

### TestHspiceTr0Read

Tests for the `hspice_tr0_read()` function:

| Test                      | Description                      |
| ------------------------- | -------------------------------- |
| `test_import_module`      | Verify module can be imported    |
| `test_read_tr0_file`      | Basic TR0 file reading           |
| `test_result_structure`   | Validate return data structure   |
| `test_data_dictionary`    | Check data contains numpy arrays |
| `test_time_signal_exists` | Verify TIME signal exists        |
| `test_data_consistency`   | All signals have same length     |
| `test_debug_mode`         | Reading with debug enabled       |
| `test_nonexistent_file`   | Handle missing file gracefully   |

### TestHspiceTr0ToRaw

Tests for the `hspice_tr0_to_raw()` function:

| Test                            | Description                       |
| ------------------------------- | --------------------------------- |
| `test_import_function`          | Verify function can be imported   |
| `test_convert_to_raw`           | Basic TR0 to RAW conversion       |
| `test_raw_file_header`          | Validate SPICE3 raw header format |
| `test_convert_with_debug`       | Conversion with debug mode        |
| `test_convert_nonexistent_file` | Handle missing input file         |
| `test_convert_to_readonly_path` | Handle invalid output path        |

### TestDataIntegrity

Tests for data consistency:

| Test                        | Description                                 |
| --------------------------- | ------------------------------------------- |
| `test_roundtrip_data_count` | Verify point count matches after conversion |

### TestEdgeCases

Tests for edge cases and special scenarios:

| Test                    | Description                     |
| ----------------------- | ------------------------------- |
| `test_multiple_reads`   | File can be read multiple times |
| `test_signal_name_case` | Signal name case handling       |
| `test_data_range_valid` | No NaN or Inf values in data    |

## Adding New Tests

1. Add new test functions to the appropriate class in `test_tr0_parser.py`
2. Use fixtures from `conftest.py` for common test data:
   - `example_tr0_path` - Path to the example TR0 file
   - `tr0_data` - Parsed TR0 data
   - `tr0_signals` - Signal dictionary
   - `tr0_metadata` - Title, date, and scale name

Example:

```python
def test_my_new_feature(self, tr0_signals):
    """Test description"""
    assert "time" in tr0_signals
    assert len(tr0_signals["time"]) > 0
```

## Troubleshooting

### ModuleNotFoundError: No module named '\_tr0_parser'

The native extension has not been built. Run:

```bash
maturin develop --release
```

### Test file not found

Ensure `example/PinToPinSim.tr0` exists in the project root.

### Import errors

Make sure you're in the virtual environment:

```bash
source .venv/bin/activate
```
