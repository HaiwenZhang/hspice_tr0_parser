"""
Test suite for basic waveform reading functionality.

Tests module import, basic reading, result structure, and error handling.
"""

import pytest
import numpy as np
from pathlib import Path

from tests.conftest import (
    read_waveform,
    get_data_dict,
    get_scale_name,
    EXAMPLE_DIR,
    EXAMPLE_TR0,
)


class TestModuleImport:
    """Tests for module import and function availability"""
    
    def test_import_read_function(self):
        """Test that read can be imported"""
        from hspice_tr0_parser import read
        assert callable(read)
    
    def test_import_convert_function(self):
        """Test that convert_to_raw can be imported"""
        from hspice_tr0_parser import convert_to_raw
        assert callable(convert_to_raw)
    
    def test_import_classes(self):
        """Test that classes can be imported"""
        from hspice_tr0_parser import WaveformResult, Variable, DataTable
        assert WaveformResult is not None
        assert Variable is not None
        assert DataTable is not None


class TestBasicReading:
    """Tests for basic TR0 file reading"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
    
    def test_read_returns_result(self):
        """Test that reading returns a WaveformResult"""
        result = read_waveform(EXAMPLE_TR0)
        
        assert result is not None, "read returned None"
        assert hasattr(result, 'title'), "Result should have title"
        assert hasattr(result, 'analysis'), "Result should have analysis"
    
    def test_result_structure(self):
        """Test the structure of the returned result"""
        result = read_waveform(EXAMPLE_TR0)
        
        assert isinstance(result.title, str), "title should be str"
        assert isinstance(result.date, str), "date should be str"
        assert isinstance(result.scale_name, str), "scale_name should be str"
        assert isinstance(result.analysis, str), "analysis should be str"
        assert len(result.variables) > 0, "should have variables"
        assert len(result.tables) > 0, "should have tables"
    
    def test_variables_structure(self):
        """Test that variables have correct structure"""
        result = read_waveform(EXAMPLE_TR0)
        
        for var in result.variables:
            assert hasattr(var, 'name'), "Variable should have name"
            assert hasattr(var, 'var_type'), "Variable should have var_type"
            assert isinstance(var.name, str), "name should be str"
    
    def test_get_signal(self):
        """Test getting signal data by name"""
        result = read_waveform(EXAMPLE_TR0)
        
        # Get scale signal
        scale_data = result.get(result.scale_name)
        assert scale_data is not None, "Should get scale data"
        assert isinstance(scale_data, np.ndarray), "Should be numpy array"
        assert len(scale_data) > 0, "Should have data"
    
    def test_time_signal_exists(self):
        """Test that TIME signal exists for transient analysis"""
        result = read_waveform(EXAMPLE_TR0)
        
        assert result.scale_name.upper() == "TIME", f"Expected scale name 'TIME', got '{result.scale_name}'"
        time_data = result.get('TIME')
        if time_data is None:
            time_data = result.get('time')
        assert time_data is not None, "TIME signal not found"
    
    def test_data_consistency(self):
        """Test that all signals have the same length"""
        result = read_waveform(EXAMPLE_TR0)
        
        lengths = set()
        for var in result.variables:
            data = result.get(var.name)
            if data is not None:
                lengths.add(len(data))
        
        assert len(lengths) == 1, f"All signals should have same length, got {lengths}"
    
    def test_debug_modes(self):
        """Test reading with different debug levels"""
        for debug_level in [0, 1, 2]:
            result = read_waveform(EXAMPLE_TR0, debug=debug_level)
            assert result is not None, f"Failed with debug={debug_level}"


class TestErrorHandling:
    """Tests for error handling scenarios"""
    
    def test_nonexistent_file(self):
        """Test reading a non-existent file returns None"""
        result = read_waveform("/nonexistent/path/file.tr0")
        assert result is None, "Expected None for non-existent file"
    
    def test_invalid_path(self):
        """Test reading from invalid path"""
        result = read_waveform("")
        assert result is None, "Expected None for empty path"


class TestEdgeCases:
    """Tests for edge cases and special scenarios"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
    
    def test_multiple_reads(self):
        """Test that file can be read multiple times with identical results"""
        result1 = read_waveform(EXAMPLE_TR0)
        result2 = read_waveform(EXAMPLE_TR0)
        
        assert result1 is not None and result2 is not None
        
        names1 = set(v.name for v in result1.variables)
        names2 = set(v.name for v in result2.variables)
        
        assert names1 == names2
    
    def test_data_values_valid(self):
        """Test that data values are not NaN or Inf"""
        result = read_waveform(EXAMPLE_TR0)
        
        for var in result.variables:
            values = result.get(var.name)
            if values is not None:
                assert not np.any(np.isnan(values)), f"Signal '{var.name}' contains NaN"
                assert not np.any(np.isinf(values)), f"Signal '{var.name}' contains Inf"
    
    def test_analysis_type(self):
        """Test that analysis type is correct"""
        result = read_waveform(EXAMPLE_TR0)
        assert result.analysis == "transient", f"Expected 'transient', got '{result.analysis}'"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
