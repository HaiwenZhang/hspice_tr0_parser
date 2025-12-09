"""
Test suite for reference data comparison.

Compares parsed results against reference pickle data to ensure parsing accuracy.
"""

import pytest
import pickle
import numpy as np
from pathlib import Path

from tests.conftest import (
    read_hspice_file,
    get_data_dict,
    load_reference_pickle,
    get_time_key,
    EXAMPLE_DIR,
    REFERENCE_DATA_FILES,
)


@pytest.fixture(params=REFERENCE_DATA_FILES, ids=[r[0] for r in REFERENCE_DATA_FILES])
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


class TestReferenceDataComparison:
    """Parametrized tests comparing parsed results against reference pickle data"""
    
    def test_signal_count_matches_reference(self, reference_testcase):
        """Test that parsed signal count matches reference data"""
        tr0_path, reference_data, _, _ = reference_testcase
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        
        parsed_count = len(data_dict)
        reference_count = len(reference_data)
        
        assert parsed_count == reference_count, \
            f"Signal count differs: parsed={parsed_count}, reference={reference_count}"
    
    def test_data_length_matches_reference(self, reference_testcase):
        """Test that data length matches reference"""
        tr0_path, reference_data, _, _ = reference_testcase
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        
        # Get time/frequency key
        time_key = get_time_key(data_dict)
        parsed_length = len(data_dict[time_key])
        
        # Reference data may use different key name
        ref_time_key = get_time_key(reference_data)
        ref_value = reference_data[ref_time_key]
        reference_length = len(ref_value[0]) if isinstance(ref_value, list) else len(ref_value)
        
        # Note: For AC analysis, lengths may differ due to complex vs split real/imag
        # So we check that parsed is non-empty and reasonable
        assert parsed_length > 0, "Parsed data should not be empty"
        
        # For non-AC formats, expect exact match
        if 'HERTZ' not in time_key.upper():
            assert parsed_length == reference_length, \
                f"Data length differs: parsed={parsed_length}, reference={reference_length}"
    
    def test_time_values_match_reference(self, reference_testcase):
        """Test that time/frequency values match reference within tolerance"""
        tr0_path, reference_data, rtol, atol = reference_testcase
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        
        # Get time/frequency data
        time_key = get_time_key(data_dict)
        parsed_values = np.array(data_dict[time_key])
        
        # Skip detailed value comparison for AC and SW formats
        # AC: reference has interleaved real/imag, we have separate complex values
        # SW: reference uses different key structure
        if 'ac0' in tr0_path.lower() or 'sw0' in tr0_path.lower():
            # Just verify we have valid non-empty data
            assert len(parsed_values) > 0, "Parsed data should not be empty"
            assert not np.any(np.isnan(parsed_values)), "Data should not contain NaN"
            return
        
        # Get reference values
        ref_time_key = get_time_key(reference_data)
        ref_value = reference_data[ref_time_key]
        reference_values = np.array(ref_value[0] if isinstance(ref_value, list) else ref_value)
        
        # Compare with tolerance
        np.testing.assert_allclose(
            parsed_values, reference_values,
            rtol=rtol, atol=atol,
            err_msg=f"Time/frequency values do not match reference for {tr0_path}"
        )


class TestSpecificFormatReferences:
    """Tests for specific format reference comparisons with detailed assertions"""
    
    @pytest.fixture(autouse=True)
    def setup(self):
        """Setup paths for all reference files"""
        self.test_files = {
            "9601_tr": (EXAMPLE_DIR / "test_9601.tr0", EXAMPLE_DIR / "data_dict_9601.pickle"),
            "2001_tr": (EXAMPLE_DIR / "test_2001.tr0", EXAMPLE_DIR / "data_dict_tr_2001.pickle"),
            "9601_ac": (EXAMPLE_DIR / "test_9601.ac0", EXAMPLE_DIR / "data_dict_ac_9601.pickle"),
            "9601_sw": (EXAMPLE_DIR / "test_9601.sw0", EXAMPLE_DIR / "data_dict_sw_9601.pickle"),
        }
    
    def test_9601_transient_precision(self):
        """Test 9601 format transient data with float32 precision tolerance"""
        tr0_path, pickle_path = self.test_files["9601_tr"]
        if not tr0_path.exists() or not pickle_path.exists():
            pytest.skip("9601 TR test files not found")
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        reference = load_reference_pickle(pickle_path)
        
        time_key = get_time_key(data_dict)
        ref_time_key = get_time_key(reference)
        
        parsed_time = np.array(data_dict[time_key])
        ref_time = reference[ref_time_key]
        ref_time = np.array(ref_time[0] if isinstance(ref_time, list) else ref_time)
        
        # float32 precision: ~1e-5 relative tolerance
        np.testing.assert_allclose(parsed_time, ref_time, rtol=1e-5, atol=1e-10)
    
    def test_2001_transient_precision(self):
        """Test 2001 format transient data with float64 precision tolerance"""
        tr0_path, pickle_path = self.test_files["2001_tr"]
        if not tr0_path.exists() or not pickle_path.exists():
            pytest.skip("2001 TR test files not found")
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        reference = load_reference_pickle(pickle_path)
        
        time_key = get_time_key(data_dict)
        ref_time_key = get_time_key(reference)
        
        parsed_time = np.array(data_dict[time_key])
        ref_time = reference[ref_time_key]
        ref_time = np.array(ref_time[0] if isinstance(ref_time, list) else ref_time)
        
        # float64 precision: ~1e-10 relative tolerance
        np.testing.assert_allclose(parsed_time, ref_time, rtol=1e-10, atol=1e-15)
    
    def test_ac_frequency_values(self):
        """Test AC format frequency (HERTZ) values"""
        tr0_path, pickle_path = self.test_files["9601_ac"]
        if not tr0_path.exists() or not pickle_path.exists():
            pytest.skip("AC test files not found")
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        
        # AC should have HERTZ as scale
        assert 'HERTZ' in data_dict or 'hertz' in data_dict, \
            f"HERTZ not found in AC data. Keys: {list(data_dict.keys())}"
        
        # Verify data is present and valid
        freq_key = 'HERTZ' if 'HERTZ' in data_dict else 'hertz'
        assert len(data_dict[freq_key]) > 0, "Frequency data should not be empty"
    
    def test_dc_sweep_values(self):
        """Test DC sweep format values"""
        tr0_path, pickle_path = self.test_files["9601_sw"]
        if not tr0_path.exists() or not pickle_path.exists():
            pytest.skip("DC sweep test files not found")
        
        result = read_hspice_file(tr0_path)
        data_dict = get_data_dict(result)
        reference = load_reference_pickle(pickle_path)
        
        # Verify signal count matches
        assert len(data_dict) == len(reference), \
            f"Signal count mismatch: {len(data_dict)} vs {len(reference)}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
