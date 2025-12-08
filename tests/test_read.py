"""
Test suite for basic HSPICE TR0 reading functionality.

Tests module import, basic reading, result structure, and error handling.
"""

import pytest
import numpy as np
from pathlib import Path


# Project paths
PROJECT_ROOT = Path(__file__).parent.parent
EXAMPLE_DIR = PROJECT_ROOT / "example"
EXAMPLE_TR0 = EXAMPLE_DIR / "PinToPinSim.tr0"


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


class TestModuleImport:
    """Tests for module import and function availability"""
    
    def test_import_read_function(self):
        """Test that hspice_tr0_read can be imported"""
        from hspice_tr0_parser import hspice_tr0_read
        assert callable(hspice_tr0_read)
    
    def test_import_convert_function(self):
        """Test that hspice_tr0_to_raw can be imported"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        assert callable(hspice_tr0_to_raw)


class TestBasicReading:
    """Tests for basic TR0 file reading"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
    
    def test_read_returns_list(self):
        """Test that reading returns a list"""
        result = read_hspice_file(EXAMPLE_TR0)
        
        assert result is not None, "hspice_tr0_read returned None"
        assert isinstance(result, list), f"Expected list, got {type(result)}"
        assert len(result) >= 1, "Result should contain at least one analysis"
    
    def test_result_structure(self):
        """Test the structure of the returned result"""
        result = read_hspice_file(EXAMPLE_TR0)
        analysis = result[0]
        
        # Should be a tuple with expected structure
        assert isinstance(analysis, tuple), f"Analysis should be tuple, got {type(analysis)}"
        assert len(analysis) >= 4, f"Analysis tuple should have at least 4 elements"
        
        # Extract and verify components
        sim_results = analysis[0]
        scale_name = analysis[1]
        title = analysis[3]
        date = analysis[4]
        
        assert isinstance(sim_results, tuple), "sim_results should be tuple"
        assert len(sim_results) == 3, "sim_results should have 3 elements"
        assert isinstance(scale_name, str), "scale_name should be str"
        assert isinstance(title, str), "title should be str"
        assert isinstance(date, str), "date should be str"
    
    def test_data_dictionary_structure(self):
        """Test that data dictionary contains numpy arrays"""
        result = read_hspice_file(EXAMPLE_TR0)
        data_dict = get_data_dict(result)
        
        assert isinstance(data_dict, dict), f"data_dict should be dict"
        assert len(data_dict) >= 1, "data_dict should have at least one signal"
        
        for name, values in data_dict.items():
            assert isinstance(name, str), f"Signal name should be str"
            assert isinstance(values, np.ndarray), f"Signal '{name}' should be numpy array"
            assert values.dtype in [np.float32, np.float64], \
                f"Signal '{name}' should be float type, got {values.dtype}"
    
    def test_time_signal_exists(self):
        """Test that TIME signal exists for transient analysis"""
        result = read_hspice_file(EXAMPLE_TR0)
        scale_name = get_scale_name(result)
        data_dict = get_data_dict(result)
        
        assert scale_name.upper() == "TIME", f"Expected scale name 'TIME', got '{scale_name}'"
        assert "TIME" in data_dict or "time" in data_dict, \
            f"TIME signal not found. Keys: {list(data_dict.keys())}"
    
    def test_data_consistency(self):
        """Test that all signals have the same length"""
        result = read_hspice_file(EXAMPLE_TR0)
        data_dict = get_data_dict(result)
        
        lengths = [len(v) for v in data_dict.values()]
        assert len(set(lengths)) == 1, f"All signals should have same length, got {set(lengths)}"
    
    def test_debug_modes(self):
        """Test reading with different debug levels"""
        for debug_level in [0, 1, 2]:
            result = read_hspice_file(EXAMPLE_TR0, debug=debug_level)
            assert result is not None, f"Failed with debug={debug_level}"


class TestErrorHandling:
    """Tests for error handling scenarios"""
    
    def test_nonexistent_file(self):
        """Test reading a non-existent file returns None"""
        result = read_hspice_file("/nonexistent/path/file.tr0")
        assert result is None, "Expected None for non-existent file"
    
    def test_invalid_path(self):
        """Test reading from invalid path"""
        result = read_hspice_file("")
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
        result1 = read_hspice_file(EXAMPLE_TR0)
        result2 = read_hspice_file(EXAMPLE_TR0)
        
        assert result1 is not None and result2 is not None
        
        data1 = get_data_dict(result1)
        data2 = get_data_dict(result2)
        
        assert data1.keys() == data2.keys()
        for key in data1.keys():
            np.testing.assert_array_equal(data1[key], data2[key])
    
    def test_data_values_valid(self):
        """Test that data values are not NaN or Inf"""
        result = read_hspice_file(EXAMPLE_TR0)
        data_dict = get_data_dict(result)
        
        for name, values in data_dict.items():
            assert not np.any(np.isnan(values)), f"Signal '{name}' contains NaN"
            assert not np.any(np.isinf(values)), f"Signal '{name}' contains Inf"
    
    def test_signal_name_case(self):
        """Test signal name case consistency"""
        result = read_hspice_file(EXAMPLE_TR0)
        data_dict = get_data_dict(result)
        
        for name in data_dict.keys():
            # Names should be consistently cased
            assert name == name.lower() or name == name.upper(), \
                f"Signal name '{name}' has unexpected case"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
