"""
Pytest configuration and shared fixtures for waveform parser tests
"""

import pytest
import pickle
import tempfile
import os
import numpy as np
from pathlib import Path

# Project paths
PROJECT_ROOT = Path(__file__).parent.parent
EXAMPLE_DIR = PROJECT_ROOT / "example"
EXAMPLE_TR0 = EXAMPLE_DIR / "PinToPinSim.tr0"


# =============================================================================
# Basic fixtures
# =============================================================================

@pytest.fixture
def example_tr0_path():
    """Return the path to the example TR0 file"""
    assert EXAMPLE_TR0.exists(), f"Example TR0 file not found: {EXAMPLE_TR0}"
    return str(EXAMPLE_TR0)


@pytest.fixture
def waveform_result(example_tr0_path):
    """Return parsed waveform result"""
    from hspice_tr0_parser import read
    result = read(example_tr0_path)
    assert result is not None, "Failed to read TR0 file"
    return result


# =============================================================================
# Format test data definitions
# =============================================================================

# All supported file formats with their expected properties
FORMAT_TEST_FILES = [
    # (filename, expected_scale_name, analysis_type)
    ("test_9601.tr0", "TIME", "transient"),
    ("test_2001.tr0", "TIME", "transient"),
    ("test_9601.ac0", "HERTZ", "ac"),
    ("test_9601.sw0", None, "dc"),  # DC sweep has variable scale name
]

# Reference data files for validation
REFERENCE_DATA_FILES = [
    # (tr0_file, pickle_file, tolerance_rtol, tolerance_atol)
    ("test_9601.tr0", "data_dict_9601.pickle", 1e-5, 1e-10),
    ("test_2001.tr0", "data_dict_tr_2001.pickle", 1e-10, 1e-15),
    ("test_9601.ac0", "data_dict_ac_9601.pickle", 1e-5, 1e-10),
    ("test_9601.sw0", "data_dict_sw_9601.pickle", 1e-5, 1e-10),
]


# =============================================================================
# Helper functions
# =============================================================================

def read_waveform(filepath, debug=0):
    """Unified waveform file reading interface"""
    from hspice_tr0_parser import read
    return read(str(filepath), debug=debug)


def convert_to_raw(input_path, output_path, debug=0):
    """Unified conversion interface"""
    from hspice_tr0_parser import convert_to_raw as convert
    return convert(str(input_path), str(output_path), debug=debug)


def get_data_dict(result):
    """Extract data dictionary from WaveformResult"""
    if result is None:
        return {}
    # Build dict from first table
    d = {}
    table = result.tables[0]
    for var in result.variables:
        data = result.get(var.name)
        if data is not None:
            d[var.name] = data
    return d


def get_scale_name(result):
    """Extract scale name from WaveformResult"""
    if result is None:
        return ""
    return result.scale_name


def load_reference_pickle(pickle_path):
    """Load reference data from pickle file"""
    with open(pickle_path, 'rb') as f:
        return pickle.load(f)


def get_time_key(data_dict):
    """Get the TIME/HERTZ key from data dictionary"""
    for key in ['TIME', 'time', 'HERTZ', 'hertz']:
        if key in data_dict:
            return key
    # Return first key as fallback
    return list(data_dict.keys())[0] if data_dict else None


# =============================================================================
# Parametrized fixtures
# =============================================================================

@pytest.fixture(params=FORMAT_TEST_FILES, ids=lambda x: x[0])
def format_testcase(request):
    """
    Parametrized fixture for format testing.
    Yields: (filepath, expected_scale, analysis_type)
    """
    filename, expected_scale, analysis_type = request.param
    filepath = EXAMPLE_DIR / filename
    if not filepath.exists():
        pytest.skip(f"Test file not found: {filepath}")
    return str(filepath), expected_scale, analysis_type


@pytest.fixture(params=REFERENCE_DATA_FILES, ids=lambda x: x[0])
def reference_testcase(request):
    """
    Parametrized fixture for reference data comparison.
    Yields: (tr0_path, reference_data, rtol, atol)
    """
    tr0_file, pickle_file, rtol, atol = request.param
    tr0_path = EXAMPLE_DIR / tr0_file
    pickle_path = EXAMPLE_DIR / pickle_file
    
    if not tr0_path.exists() or not pickle_path.exists():
        pytest.skip(f"Test files not found: {tr0_file} or {pickle_file}")
    
    reference_data = load_reference_pickle(pickle_path)
    return str(tr0_path), reference_data, rtol, atol


@pytest.fixture
def temp_raw_file():
    """Create a temporary file for raw output and clean up after test"""
    with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
        output_path = f.name
    yield output_path
    if os.path.exists(output_path):
        os.unlink(output_path)
