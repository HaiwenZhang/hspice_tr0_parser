"""
Test suite for HSPICE file format variants.

Uses pytest parametrization to test multiple formats with shared test logic:
- 9601 format (float32): .tr0, .ac0, .sw0
- 2001 format (float64): .tr0
"""

import pytest
import numpy as np
from pathlib import Path


# Project paths
PROJECT_ROOT = Path(__file__).parent.parent
EXAMPLE_DIR = PROJECT_ROOT / "example"

# All supported file formats with their expected properties
FORMAT_TEST_FILES = [
    # (filename, expected_scale_name, analysis_type)
    ("test_9601.tr0", "TIME", "transient"),
    ("test_2001.tr0", "TIME", "transient"),
    ("test_9601.ac0", "HERTZ", "ac"),
    ("test_9601.sw0", None, "dc"),  # DC sweep has variable scale name
]


def read_hspice_file(filepath, debug=0):
    """Unified HSPICE file reading interface"""
    from hspice_tr0_parser import hspice_tr0_read
    return hspice_tr0_read(str(filepath), debug=debug)


def get_data_dict(result):
    """Extract data dictionary from parse result"""
    return result[0][0][2][0]


def get_scale_name(result):
    """Extract scale name from parse result"""
    return result[0][1]


# Generate test IDs for parametrization
def format_id(param):
    return param[0]


@pytest.fixture(params=FORMAT_TEST_FILES, ids=[f[0] for f in FORMAT_TEST_FILES])
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


class TestFormatReading:
    """Parametrized tests for all supported file formats"""
    
    def test_read_file(self, format_testcase):
        """Test that file can be read successfully"""
        filepath, _, _ = format_testcase
        result = read_hspice_file(filepath)
        
        assert result is not None, f"Failed to read {filepath}"
        assert isinstance(result, list), f"Expected list, got {type(result)}"
        assert len(result) >= 1, "Result should contain at least one analysis"
    
    def test_scale_name(self, format_testcase):
        """Test that scale name matches expected value"""
        filepath, expected_scale, _ = format_testcase
        result = read_hspice_file(filepath)
        scale_name = get_scale_name(result)
        
        if expected_scale is not None:
            assert scale_name.upper() == expected_scale.upper(), \
                f"Expected scale '{expected_scale}', got '{scale_name}'"
        else:
            # DC sweep has variable scale name, just verify it's non-empty
            assert isinstance(scale_name, str) and len(scale_name) > 0
    
    def test_data_arrays_valid(self, format_testcase):
        """Test that all data arrays are valid numpy arrays"""
        filepath, _, _ = format_testcase
        result = read_hspice_file(filepath)
        data_dict = get_data_dict(result)
        
        for name, values in data_dict.items():
            assert isinstance(values, np.ndarray), \
                f"Signal '{name}' should be numpy array"
            assert len(values) > 0, f"Signal '{name}' should not be empty"
    
    def test_data_consistency(self, format_testcase):
        """Test that all signals have the same length"""
        filepath, _, _ = format_testcase
        result = read_hspice_file(filepath)
        data_dict = get_data_dict(result)
        
        lengths = [len(v) for v in data_dict.values()]
        assert len(set(lengths)) == 1, \
            f"All signals should have same length, got {set(lengths)}"
    
    def test_no_nan_or_inf(self, format_testcase):
        """Test that data contains no NaN or Inf values"""
        filepath, _, _ = format_testcase
        result = read_hspice_file(filepath)
        data_dict = get_data_dict(result)
        
        for name, values in data_dict.items():
            # Handle complex values (AC analysis)
            if np.iscomplexobj(values):
                assert not np.any(np.isnan(np.real(values))), \
                    f"Signal '{name}' real part contains NaN"
                assert not np.any(np.isnan(np.imag(values))), \
                    f"Signal '{name}' imag part contains NaN"
            else:
                assert not np.any(np.isnan(values)), \
                    f"Signal '{name}' contains NaN"
                assert not np.any(np.isinf(values)), \
                    f"Signal '{name}' contains Inf"


class TestFormatComparison:
    """Tests comparing 9601 and 2001 format outputs"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: define test file paths"""
        self.tr0_9601 = EXAMPLE_DIR / "test_9601.tr0"
        self.tr0_2001 = EXAMPLE_DIR / "test_2001.tr0"
        
        if not self.tr0_9601.exists() or not self.tr0_2001.exists():
            pytest.skip("Comparison test files not found")
    
    def test_both_formats_readable(self):
        """Test that both 9601 and 2001 formats can be read"""
        result_9601 = read_hspice_file(self.tr0_9601)
        result_2001 = read_hspice_file(self.tr0_2001)
        
        assert result_9601 is not None, "9601 format should be readable"
        assert result_2001 is not None, "2001 format should be readable"
    
    def test_same_signal_names(self):
        """Test that both formats have the same signal names"""
        result_9601 = read_hspice_file(self.tr0_9601)
        result_2001 = read_hspice_file(self.tr0_2001)
        
        signals_9601 = set(get_data_dict(result_9601).keys())
        signals_2001 = set(get_data_dict(result_2001).keys())
        
        assert signals_9601 == signals_2001, \
            f"Signal names differ: 9601={signals_9601}, 2001={signals_2001}"
    
    def test_same_data_length(self):
        """Test that both formats have same number of data points"""
        result_9601 = read_hspice_file(self.tr0_9601)
        result_2001 = read_hspice_file(self.tr0_2001)
        
        data_9601 = get_data_dict(result_9601)
        data_2001 = get_data_dict(result_2001)
        
        first_key = list(data_9601.keys())[0]
        len_9601 = len(data_9601[first_key])
        len_2001 = len(data_2001[first_key])
        
        assert len_9601 == len_2001, \
            f"Data length differs: 9601={len_9601}, 2001={len_2001}"


class TestDebugOutput:
    """Tests for debug output functionality"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: find an available test file"""
        candidates = [
            EXAMPLE_DIR / "test_9601.tr0",
            EXAMPLE_DIR / "PinToPinSim.tr0",
        ]
        self.test_file = None
        for candidate in candidates:
            if candidate.exists():
                self.test_file = candidate
                break
        if self.test_file is None:
            pytest.skip("No test file found")
    
    @pytest.mark.parametrize("debug_level", [0, 1, 2])
    def test_debug_levels(self, debug_level):
        """Test that all debug levels work without error"""
        result = read_hspice_file(self.test_file, debug=debug_level)
        assert result is not None, f"Failed with debug={debug_level}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
