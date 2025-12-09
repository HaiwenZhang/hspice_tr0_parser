"""
Tests for streaming API functionality.
"""

import pytest
import numpy as np
from pathlib import Path

from tests.conftest import (
    read_hspice_file,
    get_data_dict,
    EXAMPLE_DIR,
    EXAMPLE_TR0,
)


class TestStreamingBasic:
    """Test basic streaming functionality"""

    def test_import_stream_function(self):
        """Test that streaming function can be imported"""
        from hspice_tr0_parser import hspice_tr0_stream
        assert callable(hspice_tr0_stream)

    def test_stream_returns_generator(self):
        """Test that streaming returns a generator"""
        from hspice_tr0_parser import hspice_tr0_stream
        result = hspice_tr0_stream(str(EXAMPLE_TR0))
        # Generator should be iterable
        assert hasattr(result, '__iter__')
        assert hasattr(result, '__next__')

    def test_stream_yields_chunks(self):
        """Test that streaming yields chunks"""
        from hspice_tr0_parser import hspice_tr0_stream
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0)))
        assert len(chunks) > 0, "Should yield at least one chunk"

    def test_chunk_structure(self):
        """Test that chunks have expected structure"""
        from hspice_tr0_parser import hspice_tr0_stream
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0)))
        
        for chunk in chunks:
            assert 'chunk_index' in chunk, "Chunk should have chunk_index"
            assert 'time_range' in chunk, "Chunk should have time_range"
            assert 'data' in chunk, "Chunk should have data"
            
            # Check time_range is a tuple of two floats
            assert isinstance(chunk['time_range'], tuple)
            assert len(chunk['time_range']) == 2
            
            # Check data is a dict
            assert isinstance(chunk['data'], dict)


class TestStreamingChunkSize:
    """Test chunk size control"""

    def test_default_chunk_size(self):
        """Test default chunk size (10000)"""
        from hspice_tr0_parser import hspice_tr0_stream
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0)))
        
        # With small test file, should be 1 chunk if < 10000 points
        # or multiple chunks if larger
        assert len(chunks) >= 1

    def test_custom_chunk_size(self):
        """Test custom chunk size"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        # Use very small chunk size to force multiple chunks
        chunks_small = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=10))
        chunks_large = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=100000))
        
        # Smaller chunk size should produce more chunks
        assert len(chunks_small) >= len(chunks_large)

    def test_chunk_index_sequential(self):
        """Test that chunk indices are sequential"""
        from hspice_tr0_parser import hspice_tr0_stream
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=100))
        
        for i, chunk in enumerate(chunks):
            assert chunk['chunk_index'] == i, f"Chunk index should be {i}"


class TestStreamingSignalFilter:
    """Test signal filtering functionality"""

    def test_filter_specific_signals(self):
        """Test filtering to specific signals"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        # First, get all signal names from regular read
        full_result = read_hspice_file(EXAMPLE_TR0)
        all_signals = list(get_data_dict(full_result).keys())
        
        if len(all_signals) < 3:
            pytest.skip("Need at least 3 signals for filter test")
        
        # Filter to 2 non-scale signals (skip first which is usually scale)
        filter_signals = all_signals[1:3]
        chunks = list(hspice_tr0_stream(
            str(EXAMPLE_TR0), 
            signals=filter_signals
        ))
        
        for chunk in chunks:
            # Scale signal is always included, plus filtered signals
            # So we should have filter_signals as subset
            for sig in filter_signals:
                assert sig in chunk['data'], f"Filtered signal {sig} should be in chunk"
            
            # Should have fewer signals than full read
            assert len(chunk['data']) <= len(all_signals)


class TestStreamingDataIntegrity:
    """Test data integrity between streaming and full read"""

    def test_total_points_match(self):
        """Test that total points match between stream and full read"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        # Get full data
        full_result = read_hspice_file(EXAMPLE_TR0)
        full_data = get_data_dict(full_result)
        scale_name = list(full_data.keys())[0]  # First key is usually scale
        total_points_full = len(full_data[scale_name])
        
        # Count streamed points
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=100))
        total_points_stream = sum(
            len(chunk['data'].get(scale_name, []))
            for chunk in chunks
        )
        
        assert total_points_stream == total_points_full, \
            f"Streamed points ({total_points_stream}) should match full ({total_points_full})"

    def test_time_range_continuous(self):
        """Test that time ranges are continuous across chunks"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=100))
        
        if len(chunks) > 1:
            for i in range(len(chunks) - 1):
                current_end = chunks[i]['time_range'][1]
                next_start = chunks[i + 1]['time_range'][0]
                
                # Next chunk should start after current ends
                assert next_start >= current_end, \
                    f"Chunk {i+1} start ({next_start}) should be >= chunk {i} end ({current_end})"

    def test_data_values_match(self):
        """Test that data values match between stream and full read"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        # Get full data
        full_result = read_hspice_file(EXAMPLE_TR0)
        full_data = get_data_dict(full_result)
        
        # Get first signal
        signal_names = list(full_data.keys())
        test_signal = signal_names[0]
        full_values = full_data[test_signal]
        
        # Collect streamed values
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=100))
        streamed_values = np.concatenate([
            chunk['data'][test_signal] 
            for chunk in chunks
        ])
        
        # Values should match
        np.testing.assert_array_almost_equal(
            streamed_values, full_values,
            err_msg=f"Streamed values should match full read for {test_signal}"
        )


class TestStreamingFormats:
    """Test streaming with different file formats"""

    @pytest.mark.parametrize("filename", [
        "test_9601.tr0",
        "test_2001.tr0",
        "test_9601.ac0",
        "test_9601.sw0",
    ])
    def test_stream_different_formats(self, filename):
        """Test streaming works with different HSPICE formats"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        filepath = EXAMPLE_DIR / filename
        if not filepath.exists():
            pytest.skip(f"Test file {filename} not found")
        
        chunks = list(hspice_tr0_stream(str(filepath)))
        assert len(chunks) > 0, f"Should stream {filename}"
        
        # Verify chunk structure
        for chunk in chunks:
            assert 'data' in chunk
            assert len(chunk['data']) > 0


class TestStreamingErrorHandling:
    """Test error handling in streaming"""

    def test_nonexistent_file(self):
        """Test streaming nonexistent file returns empty"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        chunks = list(hspice_tr0_stream("/nonexistent/path/file.tr0"))
        assert len(chunks) == 0, "Nonexistent file should return empty stream"

    def test_invalid_chunk_size(self):
        """Test that very small chunk size still works"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        # Chunk size of 1 should still work
        chunks = list(hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=1))
        assert len(chunks) > 0


class TestStreamingMemoryEfficiency:
    """Test memory efficiency aspects of streaming"""

    def test_generator_not_list(self):
        """Test that stream returns a generator, not a pre-computed list"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        stream = hspice_tr0_stream(str(EXAMPLE_TR0))
        
        # Should be a generator type, not a list
        assert not isinstance(stream, list)
        
        # Should be able to iterate
        first_chunk = next(stream)
        assert first_chunk is not None

    def test_can_break_early(self):
        """Test that we can break out of streaming early"""
        from hspice_tr0_parser import hspice_tr0_stream
        
        chunks_read = 0
        for chunk in hspice_tr0_stream(str(EXAMPLE_TR0), chunk_size=10):
            chunks_read += 1
            if chunks_read >= 2:
                break
        
        # Should have read exactly 2 chunks (or fewer if file is small)
        assert chunks_read <= 2
