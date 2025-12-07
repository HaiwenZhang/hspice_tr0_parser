"""
Pytest configuration and shared fixtures for HSPICE TR0 parser tests
"""

import pytest
from pathlib import Path

# Project paths
PROJECT_ROOT = Path(__file__).parent.parent
EXAMPLE_DIR = PROJECT_ROOT / "example"
EXAMPLE_TR0 = EXAMPLE_DIR / "PinToPinSim.tr0"


@pytest.fixture
def example_tr0_path():
    """Return the path to the example TR0 file"""
    assert EXAMPLE_TR0.exists(), f"Example TR0 file not found: {EXAMPLE_TR0}"
    return str(EXAMPLE_TR0)


@pytest.fixture
def tr0_data(example_tr0_path):
    """Return parsed TR0 data"""
    from hspice_tr0_parser import hspice_tr0_read
    result = hspice_tr0_read(example_tr0_path)
    assert result is not None, "Failed to read TR0 file"
    return result


@pytest.fixture
def tr0_signals(tr0_data):
    """Return the signal dictionary from TR0 data"""
    return tr0_data[0][0][2][0]


@pytest.fixture
def tr0_metadata(tr0_data):
    """Return metadata (title, date, scale_name) from TR0 data"""
    analysis = tr0_data[0]
    return {
        "title": analysis[3],
        "date": analysis[4],
        "scale_name": analysis[1]
    }
