"""
Test suite for HSPICE file format variants.

Uses pytest parametrization to test multiple formats with shared test logic:
- 9601 format (float32): .tr0, .ac0, .sw0
- 2001 format (float64): .tr0
"""

import pytest
import numpy as np
from pathlib import Path

from tests.conftest import (
    read_waveform,
    get_data_dict,
    get_scale_name,
    EXAMPLE_DIR,
    FORMAT_TEST_FILES,
)


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
        result = read_waveform(filepath)
        
        assert result is not None, f"Failed to read {filepath}"
        assert hasattr(result, 'variables'), "Result should have variables"
        assert len(result.variables) >= 1, "Should have at least one variable"
    
    def test_scale_name(self, format_testcase):
        """Test that scale name matches expected value"""
        filepath, expected_scale, _ = format_testcase
        result = read_waveform(filepath)
        
        if expected_scale is not None:
            assert result.scale_name.upper() == expected_scale.upper(), \
                f"Expected scale '{expected_scale}', got '{result.scale_name}'"
        else:
            # DC sweep has variable scale name, just verify it's non-empty
            assert isinstance(result.scale_name, str) and len(result.scale_name) > 0
    
    def test_analysis_type(self, format_testcase):
        """Test that analysis type matches expected value"""
        filepath, _, expected_analysis = format_testcase
        result = read_waveform(filepath)
        
        assert result.analysis == expected_analysis, \
            f"Expected analysis '{expected_analysis}', got '{result.analysis}'"
    
    def test_data_arrays_valid(self, format_testcase):
        """Test that all data arrays are valid numpy arrays"""
        filepath, _, _ = format_testcase
        result = read_waveform(filepath)
        
        for var in result.variables:
            values = result.get(var.name)
            assert isinstance(values, np.ndarray), \
                f"Signal '{var.name}' should be numpy array"
            assert len(values) > 0, f"Signal '{var.name}' should not be empty"
    
    def test_data_consistency(self, format_testcase):
        """Test that all signals have the same length"""
        filepath, _, _ = format_testcase
        result = read_waveform(filepath)
        
        lengths = set()
        for var in result.variables:
            data = result.get(var.name)
            if data is not None:
                lengths.add(len(data))
        
        assert len(lengths) == 1, \
            f"All signals should have same length, got {lengths}"
    
    def test_no_nan_or_inf(self, format_testcase):
        """Test that data contains no NaN or Inf values"""
        filepath, _, _ = format_testcase
        result = read_waveform(filepath)
        
        for var in result.variables:
            values = result.get(var.name)
            # Handle complex values (AC analysis)
            if np.iscomplexobj(values):
                assert not np.any(np.isnan(np.real(values))), \
                    f"Signal '{var.name}' real part contains NaN"
                assert not np.any(np.isnan(np.imag(values))), \
                    f"Signal '{var.name}' imag part contains NaN"
            else:
                assert not np.any(np.isnan(values)), \
                    f"Signal '{var.name}' contains NaN"
                assert not np.any(np.isinf(values)), \
                    f"Signal '{var.name}' contains Inf"


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
        result_9601 = read_waveform(self.tr0_9601)
        result_2001 = read_waveform(self.tr0_2001)
        
        assert result_9601 is not None, "9601 format should be readable"
        assert result_2001 is not None, "2001 format should be readable"
    
    def test_same_signal_names(self):
        """Test that both formats have the same signal names"""
        result_9601 = read_waveform(self.tr0_9601)
        result_2001 = read_waveform(self.tr0_2001)
        
        signals_9601 = set(v.name for v in result_9601.variables)
        signals_2001 = set(v.name for v in result_2001.variables)
        
        assert signals_9601 == signals_2001, \
            f"Signal names differ: 9601={signals_9601}, 2001={signals_2001}"
    
    def test_same_data_length(self):
        """Test that both formats have same number of data points"""
        result_9601 = read_waveform(self.tr0_9601)
        result_2001 = read_waveform(self.tr0_2001)
        
        assert len(result_9601) == len(result_2001), \
            f"Data length differs: 9601={len(result_9601)}, 2001={len(result_2001)}"


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
        result = read_waveform(self.test_file, debug=debug_level)
        assert result is not None, f"Failed with debug={debug_level}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
