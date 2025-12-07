"""
Test suite for HSPICE TR0 parser

Uses example/PinToPinSim.tr0 as the test case
"""

import pytest
import os
import tempfile
import numpy as np
from pathlib import Path

# Get the project root directory
PROJECT_ROOT = Path(__file__).parent.parent
EXAMPLE_TR0 = PROJECT_ROOT / "example" / "PinToPinSim.tr0"


class TestHspiceTr0Read:
    """Tests for the hspice_tr0_read function"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        assert EXAMPLE_TR0.exists(), f"Test file not found: {EXAMPLE_TR0}"
    
    def test_import_module(self):
        """Test that the module can be imported"""
        from hspice_tr0_parser import hspice_tr0_read
        assert callable(hspice_tr0_read)
    
    def test_read_tr0_file(self):
        """Test basic reading of TR0 file"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        
        # Result should not be None
        assert result is not None, "hspice_tr0_read returned None"
        
        # Result should be a list
        assert isinstance(result, list), f"Expected list, got {type(result)}"
        
        # Result should have at least one analysis tuple
        assert len(result) >= 1, "Result should contain at least one analysis"
    
    def test_result_structure(self):
        """Test the structure of the returned result"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        
        # Get the first analysis result
        analysis = result[0]
        
        # Should be a tuple with expected structure:
        # (simulation_results, scale_name, None, title, date, None)
        assert isinstance(analysis, tuple), f"Analysis should be tuple, got {type(analysis)}"
        assert len(analysis) >= 4, f"Analysis tuple should have at least 4 elements, got {len(analysis)}"
        
        # Extract components
        sim_results = analysis[0]
        scale_name = analysis[1]
        title = analysis[3]
        date = analysis[4]
        
        # Check simulation results structure: (sweep_name, sweep_values, [data_dict, ...])
        assert isinstance(sim_results, tuple), f"sim_results should be tuple, got {type(sim_results)}"
        assert len(sim_results) == 3, f"sim_results should have 3 elements, got {len(sim_results)}"
        
        # Scale name should be a string
        assert isinstance(scale_name, str), f"scale_name should be str, got {type(scale_name)}"
        
        # Title should be a string
        assert isinstance(title, str), f"title should be str, got {type(title)}"
        
        # Date should be a string
        assert isinstance(date, str), f"date should be str, got {type(date)}"
    
    def test_data_dictionary(self):
        """Test that data dictionary contains numpy arrays"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        
        # Get data dictionary
        sim_results = result[0][0]
        data_list = sim_results[2]
        
        assert isinstance(data_list, list), f"data_list should be list, got {type(data_list)}"
        assert len(data_list) >= 1, "data_list should have at least one dictionary"
        
        data_dict = data_list[0]
        assert isinstance(data_dict, dict), f"data_dict should be dict, got {type(data_dict)}"
        
        # Should have at least one signal
        assert len(data_dict) >= 1, "data_dict should have at least one signal"
        
        # All values should be numpy arrays
        for name, values in data_dict.items():
            assert isinstance(name, str), f"Signal name should be str, got {type(name)}"
            assert isinstance(values, np.ndarray), f"Signal '{name}' values should be numpy array, got {type(values)}"
            assert values.dtype in [np.float32, np.float64], f"Signal '{name}' array should be float type, got {values.dtype}"
    
    def test_time_signal_exists(self):
        """Test that TIME signal exists in the data"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        
        # Get scale name and data
        scale_name = result[0][1]
        data_dict = result[0][0][2][0]
        
        # Scale name should typically be "TIME" for transient analysis
        assert scale_name.upper() == "TIME", f"Expected scale name 'TIME', got '{scale_name}'"
        
        # TIME should exist in data dictionary (keys are lowercase)
        time_key = scale_name.lower() if scale_name.lower() in data_dict else scale_name.upper()
        assert time_key in data_dict or "TIME" in data_dict or "time" in data_dict, \
            f"TIME signal not found in data. Keys: {list(data_dict.keys())}"
    
    def test_data_consistency(self):
        """Test that all signals have the same length as TIME"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        data_dict = result[0][0][2][0]
        
        # Get the length from first signal
        first_key = list(data_dict.keys())[0]
        expected_length = len(data_dict[first_key])
        
        # All signals should have same length
        for name, values in data_dict.items():
            assert len(values) == expected_length, \
                f"Signal '{name}' has length {len(values)}, expected {expected_length}"
    
    def test_debug_mode(self):
        """Test reading with debug mode enabled"""
        from hspice_tr0_parser import hspice_tr0_read
        
        # Should not raise exception with debug enabled
        result = hspice_tr0_read(str(EXAMPLE_TR0), debug=1)
        assert result is not None
        
        result = hspice_tr0_read(str(EXAMPLE_TR0), debug=2)
        assert result is not None
    
    def test_nonexistent_file(self):
        """Test reading a non-existent file"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read("/nonexistent/path/file.tr0")
        # Should return None on error
        assert result is None, "Expected None for non-existent file"


class TestHspiceTr0ToRaw:
    """Tests for the hspice_tr0_to_raw conversion function"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        assert EXAMPLE_TR0.exists(), f"Test file not found: {EXAMPLE_TR0}"
    
    def test_import_function(self):
        """Test that the conversion function can be imported"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        assert callable(hspice_tr0_to_raw)
    
    def test_convert_to_raw(self):
        """Test basic conversion from TR0 to SPICE3 raw format"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        
        with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
            output_path = f.name
        
        try:
            success = hspice_tr0_to_raw(str(EXAMPLE_TR0), output_path)
            
            assert success is True, "Conversion should return True on success"
            assert os.path.exists(output_path), "Output file should exist"
            assert os.path.getsize(output_path) > 0, "Output file should not be empty"
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)
    
    def test_raw_file_header(self):
        """Test that generated raw file has correct header format"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        
        with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
            output_path = f.name
        
        try:
            hspice_tr0_to_raw(str(EXAMPLE_TR0), output_path)
            
            # Read the header (text part before Binary:)
            with open(output_path, 'rb') as f:
                content = f.read()
            
            # Find the Binary: marker
            binary_marker = b"Binary:\n"
            marker_pos = content.find(binary_marker)
            assert marker_pos > 0, "Raw file should contain 'Binary:' marker"
            
            # Parse header
            header = content[:marker_pos].decode('utf-8')
            
            # Check required header fields
            assert "Title:" in header, "Header should contain 'Title:'"
            assert "Date:" in header, "Header should contain 'Date:'"
            assert "Plotname:" in header, "Header should contain 'Plotname:'"
            assert "Flags:" in header, "Header should contain 'Flags:'"
            assert "No. Variables:" in header, "Header should contain 'No. Variables:'"
            assert "No. Points:" in header, "Header should contain 'No. Points:'"
            assert "Variables:" in header, "Header should contain 'Variables:'"
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)
    
    def test_convert_with_debug(self):
        """Test conversion with debug mode"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        
        with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
            output_path = f.name
        
        try:
            success = hspice_tr0_to_raw(str(EXAMPLE_TR0), output_path, debug=1)
            assert success is True
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)
    
    def test_convert_nonexistent_file(self):
        """Test converting a non-existent file"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        
        with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
            output_path = f.name
        
        try:
            success = hspice_tr0_to_raw("/nonexistent/path/file.tr0", output_path)
            assert success is False, "Conversion should return False for non-existent file"
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)
    
    def test_convert_to_readonly_path(self):
        """Test converting to an invalid output path"""
        from hspice_tr0_parser import hspice_tr0_to_raw
        
        # Try to write to root directory (should fail on most systems)
        success = hspice_tr0_to_raw(str(EXAMPLE_TR0), "/invalid_path_12345/output.raw")
        assert success is False, "Conversion should return False for invalid output path"


class TestDataIntegrity:
    """Tests for data integrity between read and write operations"""
    
    def test_roundtrip_data_count(self):
        """Test that converted file has same number of points as original"""
        from hspice_tr0_parser import hspice_tr0_read, hspice_tr0_to_raw
        
        # Read original file
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        data_dict = result[0][0][2][0]
        
        # Get original point count
        first_key = list(data_dict.keys())[0]
        original_points = len(data_dict[first_key])
        original_variables = len(data_dict)
        
        # Convert to raw
        with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
            output_path = f.name
        
        try:
            hspice_tr0_to_raw(str(EXAMPLE_TR0), output_path)
            
            # Read raw file header to check counts
            with open(output_path, 'rb') as f:
                content = f.read()
            
            header = content[:content.find(b"Binary:\n")].decode('utf-8')
            
            # Extract No. Points from header
            for line in header.split('\n'):
                if line.startswith("No. Points:"):
                    raw_points = int(line.split(':')[1].strip())
                    assert raw_points == original_points, \
                        f"Point count mismatch: {raw_points} vs {original_points}"
                elif line.startswith("No. Variables:"):
                    raw_variables = int(line.split(':')[1].strip())
                    assert raw_variables == original_variables, \
                        f"Variable count mismatch: {raw_variables} vs {original_variables}"
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)


class TestEdgeCases:
    """Tests for edge cases and special scenarios"""
    
    def test_multiple_reads(self):
        """Test that file can be read multiple times"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result1 = hspice_tr0_read(str(EXAMPLE_TR0))
        result2 = hspice_tr0_read(str(EXAMPLE_TR0))
        
        # Both results should be valid
        assert result1 is not None
        assert result2 is not None
        
        # Data should be identical
        data1 = result1[0][0][2][0]
        data2 = result2[0][0][2][0]
        
        assert data1.keys() == data2.keys()
        
        for key in data1.keys():
            np.testing.assert_array_equal(data1[key], data2[key])
    
    def test_signal_name_case(self):
        """Test signal name case handling"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        data_dict = result[0][0][2][0]
        
        # Signal names should be consistent (typically lowercase)
        for name in data_dict.keys():
            # Names should not be mixed case unexpectedly
            assert name == name.lower() or name == name.upper(), \
                f"Signal name '{name}' has unexpected case"
    
    def test_data_range_valid(self):
        """Test that data values are within valid ranges (not NaN or Inf)"""
        from hspice_tr0_parser import hspice_tr0_read
        
        result = hspice_tr0_read(str(EXAMPLE_TR0))
        data_dict = result[0][0][2][0]
        
        for name, values in data_dict.items():
            assert not np.any(np.isnan(values)), f"Signal '{name}' contains NaN values"
            assert not np.any(np.isinf(values)), f"Signal '{name}' contains Inf values"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
