"""
Test suite for TR0 to SPICE3 raw format conversion.

Tests the convert_to_raw function for various input formats.
"""

import pytest
import os
import tempfile
import numpy as np
from pathlib import Path

from tests.conftest import (
    read_waveform,
    get_data_dict,
    convert_to_raw,
    EXAMPLE_DIR,
    EXAMPLE_TR0,
)


@pytest.fixture
def temp_raw_file():
    """Create a temporary file for raw output and clean up after test"""
    with tempfile.NamedTemporaryFile(suffix=".raw", delete=False) as f:
        output_path = f.name
    yield output_path
    if os.path.exists(output_path):
        os.unlink(output_path)


class TestBasicConversion:
    """Tests for basic TR0 to raw conversion functionality"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
    
    def test_convert_success(self, temp_raw_file):
        """Test that conversion succeeds and creates output file"""
        success = convert_to_raw(EXAMPLE_TR0, temp_raw_file)
        
        assert success is True, "Conversion should return True on success"
        assert os.path.exists(temp_raw_file), "Output file should exist"
        assert os.path.getsize(temp_raw_file) > 0, "Output file should not be empty"
    
    def test_raw_header_format(self, temp_raw_file):
        """Test that generated raw file has correct header format"""
        convert_to_raw(EXAMPLE_TR0, temp_raw_file)
        
        with open(temp_raw_file, 'rb') as f:
            content = f.read()
        
        # Find the Binary: marker
        binary_marker = b"Binary:\n"
        marker_pos = content.find(binary_marker)
        assert marker_pos > 0, "Raw file should contain 'Binary:' marker"
        
        # Parse header
        header = content[:marker_pos].decode('utf-8')
        
        # Check required header fields
        required_fields = [
            "Title:", "Date:", "Plotname:", "Flags:",
            "No. Variables:", "No. Points:", "Variables:"
        ]
        for field in required_fields:
            assert field in header, f"Header should contain '{field}'"
    
    def test_convert_with_debug(self, temp_raw_file):
        """Test conversion with debug mode"""
        success = convert_to_raw(EXAMPLE_TR0, temp_raw_file, debug=1)
        assert success is True


class TestConversionErrorHandling:
    """Tests for conversion error handling"""
    
    def test_nonexistent_input_file(self, temp_raw_file):
        """Test that conversion fails gracefully for non-existent input"""
        success = convert_to_raw("/nonexistent/path/file.tr0", temp_raw_file)
        assert success is False, "Should return False for non-existent file"
    
    def test_invalid_output_path(self):
        """Test that conversion fails gracefully for invalid output path"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
        
        success = convert_to_raw(EXAMPLE_TR0, "/invalid_path_12345/output.raw")
        assert success is False, "Should return False for invalid output path"


class TestMultiFormatConversion:
    """Tests for converting various file formats"""
    
    @pytest.mark.parametrize("input_file,expected_plotname", [
        ("PinToPinSim.tr0", "Transient Analysis"),
        ("test_9601.tr0", "Transient Analysis"),
        ("test_2001.tr0", "Transient Analysis"),
        ("test_9601.ac0", "AC Analysis"),
        ("test_9601.sw0", "DC Analysis"),
    ])
    def test_convert_format(self, input_file, expected_plotname, temp_raw_file):
        """Test conversion of different file formats"""
        input_path = EXAMPLE_DIR / input_file
        if not input_path.exists():
            pytest.skip(f"Test file not found: {input_path}")
        
        success = convert_to_raw(input_path, temp_raw_file)
        assert success is True, f"Conversion of {input_file} should succeed"
        
        # Verify plotname in header
        with open(temp_raw_file, 'rb') as f:
            content = f.read()
        header = content[:content.find(b"Binary:\n")].decode('utf-8')
        assert expected_plotname in header, \
            f"Header should contain '{expected_plotname}'"


class TestDataIntegrity:
    """Tests for data integrity between read and conversion"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup: verify test file exists"""
        if not EXAMPLE_TR0.exists():
            pytest.skip(f"Test file not found: {EXAMPLE_TR0}")
    
    def test_point_count_preserved(self, temp_raw_file):
        """Test that converted file has same number of points as original"""
        # Read original
        result = read_waveform(EXAMPLE_TR0)
        original_points = len(result)
        original_variables = result.num_vars()
        
        # Convert
        convert_to_raw(EXAMPLE_TR0, temp_raw_file)
        
        # Check header
        with open(temp_raw_file, 'rb') as f:
            content = f.read()
        header = content[:content.find(b"Binary:\n")].decode('utf-8')
        
        for line in header.split('\n'):
            if line.startswith("No. Points:"):
                raw_points = int(line.split(':')[1].strip())
                assert raw_points == original_points, \
                    f"Point count mismatch: {raw_points} vs {original_points}"
            elif line.startswith("No. Variables:"):
                raw_variables = int(line.split(':')[1].strip())
                assert raw_variables == original_variables, \
                    f"Variable count mismatch: {raw_variables} vs {original_variables}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
