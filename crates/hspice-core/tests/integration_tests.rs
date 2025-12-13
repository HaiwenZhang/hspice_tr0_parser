//! Integration tests for hspice-core
//!
//! Tests based on Python test suite structure:
//! - test_read: Basic reading functionality
//! - test_formats: Format variants (9601/2001, tr0/ac0/sw0)
//! - test_stream: Streaming API
//! - test_convert: SPICE3 raw conversion

use hspice_core::{read, read_and_convert, read_debug, VectorData};
use hspice_core::{read_stream, read_stream_chunked};
use std::collections::HashSet;
use std::path::PathBuf;

// =============================================================================
// Test helpers
// =============================================================================

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("example")
}

fn example_tr0() -> PathBuf {
    example_dir().join("PinToPinSim.tr0")
}

fn test_file(name: &str) -> PathBuf {
    example_dir().join(name)
}

fn skip_if_missing(path: &PathBuf) -> bool {
    if !path.exists() {
        eprintln!("Skipping test: file not found: {:?}", path);
        return true;
    }
    false
}

// =============================================================================
// Test: Basic Reading
// =============================================================================

#[test]
fn test_read_returns_result() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap());
    assert!(result.is_ok(), "read() should succeed for valid file");
}

#[test]
fn test_result_structure() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap()).unwrap();

    // Check metadata fields are non-empty
    assert!(!result.title.is_empty(), "title should not be empty");
    assert!(
        !result.scale_name.is_empty(),
        "scale_name should not be empty"
    );

    // Check data tables exist
    assert!(
        !result.data_tables.is_empty(),
        "should have at least one data table"
    );
}

#[test]
fn test_data_dictionary_structure() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap()).unwrap();
    let data = &result.data_tables[0];

    assert!(!data.is_empty(), "data table should have signals");

    for (name, values) in data {
        assert!(!name.is_empty(), "signal name should not be empty");
        match values {
            VectorData::Real(v) => assert!(!v.is_empty(), "signal {} should have data", name),
            VectorData::Complex(v) => assert!(!v.is_empty(), "signal {} should have data", name),
        }
    }
}

#[test]
fn test_time_signal_exists() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap()).unwrap();

    assert_eq!(
        result.scale_name.to_uppercase(),
        "TIME",
        "scale should be TIME"
    );

    let data = &result.data_tables[0];
    assert!(
        data.contains_key("TIME") || data.contains_key("time"),
        "TIME signal should exist"
    );
}

#[test]
fn test_data_consistency() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap()).unwrap();
    let data = &result.data_tables[0];

    let lengths: HashSet<usize> = data
        .values()
        .map(|v| match v {
            VectorData::Real(vec) => vec.len(),
            VectorData::Complex(vec) => vec.len(),
        })
        .collect();

    assert_eq!(lengths.len(), 1, "all signals should have same length");
}

#[test]
fn test_debug_modes() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    for debug_level in [0, 1, 2] {
        let result = read_debug(path.to_str().unwrap(), debug_level);
        assert!(result.is_ok(), "should work with debug={}", debug_level);
    }
}

// =============================================================================
// Test: Error Handling
// =============================================================================

#[test]
fn test_nonexistent_file() {
    let result = read("/nonexistent/path/file.tr0");
    assert!(result.is_err(), "should return error for nonexistent file");
}

#[test]
fn test_empty_path() {
    let result = read("");
    assert!(result.is_err(), "should return error for empty path");
}

// =============================================================================
// Test: Edge Cases
// =============================================================================

#[test]
fn test_multiple_reads() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result1 = read(path.to_str().unwrap()).unwrap();
    let result2 = read(path.to_str().unwrap()).unwrap();

    assert_eq!(
        result1.data_tables[0].keys().collect::<HashSet<_>>(),
        result2.data_tables[0].keys().collect::<HashSet<_>>(),
        "signal names should match across reads"
    );
}

#[test]
fn test_data_values_valid() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap()).unwrap();

    for (name, values) in &result.data_tables[0] {
        if let VectorData::Real(vec) = values {
            for &v in vec {
                assert!(!v.is_nan(), "signal {} contains NaN", name);
                assert!(!v.is_infinite(), "signal {} contains Inf", name);
            }
        }
    }
}

// =============================================================================
// Test: Format Variants
// =============================================================================

#[test]
fn test_read_9601_tr0() {
    let path = test_file("test_9601.tr0");
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap());
    assert!(result.is_ok(), "9601 transient format should be readable");

    let data = result.unwrap();
    assert_eq!(data.scale_name.to_uppercase(), "TIME");
}

#[test]
fn test_read_2001_tr0() {
    let path = test_file("test_2001.tr0");
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap());
    assert!(result.is_ok(), "2001 transient format should be readable");

    let data = result.unwrap();
    assert_eq!(data.scale_name.to_uppercase(), "TIME");
}

#[test]
fn test_read_9601_ac0() {
    let path = test_file("test_9601.ac0");
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap());
    assert!(result.is_ok(), "AC format should be readable");

    let data = result.unwrap();
    assert_eq!(data.scale_name.to_uppercase(), "HERTZ");

    // AC analysis should have complex data
    let has_complex = data.data_tables[0]
        .values()
        .any(|v| matches!(v, VectorData::Complex(_)));
    assert!(has_complex, "AC analysis should have complex data");
}

#[test]
fn test_read_9601_sw0() {
    let path = test_file("test_9601.sw0");
    if skip_if_missing(&path) {
        return;
    }

    let result = read(path.to_str().unwrap());
    assert!(result.is_ok(), "DC sweep format should be readable");

    let data = result.unwrap();
    assert!(!data.scale_name.is_empty(), "scale name should exist");
}

#[test]
fn test_format_comparison_same_signals() {
    let path_9601 = test_file("test_9601.tr0");
    let path_2001 = test_file("test_2001.tr0");

    if skip_if_missing(&path_9601) || skip_if_missing(&path_2001) {
        return;
    }

    let result_9601 = read(path_9601.to_str().unwrap()).unwrap();
    let result_2001 = read(path_2001.to_str().unwrap()).unwrap();

    let signals_9601: HashSet<_> = result_9601.data_tables[0].keys().collect();
    let signals_2001: HashSet<_> = result_2001.data_tables[0].keys().collect();

    assert_eq!(
        signals_9601, signals_2001,
        "both formats should have same signals"
    );
}

#[test]
fn test_format_comparison_same_length() {
    let path_9601 = test_file("test_9601.tr0");
    let path_2001 = test_file("test_2001.tr0");

    if skip_if_missing(&path_9601) || skip_if_missing(&path_2001) {
        return;
    }

    let result_9601 = read(path_9601.to_str().unwrap()).unwrap();
    let result_2001 = read(path_2001.to_str().unwrap()).unwrap();

    let len_9601 = match result_9601.data_tables[0].values().next().unwrap() {
        VectorData::Real(v) => v.len(),
        VectorData::Complex(v) => v.len(),
    };
    let len_2001 = match result_2001.data_tables[0].values().next().unwrap() {
        VectorData::Real(v) => v.len(),
        VectorData::Complex(v) => v.len(),
    };

    assert_eq!(
        len_9601, len_2001,
        "both formats should have same data length"
    );
}

// =============================================================================
// Test: Streaming API
// =============================================================================

#[test]
fn test_stream_returns_iterator() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let reader = read_stream(path.to_str().unwrap());
    assert!(reader.is_ok(), "read_stream should succeed");
}

#[test]
fn test_stream_yields_chunks() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let reader = read_stream(path.to_str().unwrap()).unwrap();
    let chunks: Vec<_> = reader.collect();

    assert!(!chunks.is_empty(), "should yield at least one chunk");
}

#[test]
fn test_chunk_structure() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let reader = read_stream(path.to_str().unwrap()).unwrap();

    for chunk_result in reader {
        let chunk = chunk_result.unwrap();

        assert!(!chunk.data.is_empty(), "chunk should have data");
        assert!(
            chunk.time_range.0 <= chunk.time_range.1,
            "time range should be valid"
        );
    }
}

#[test]
fn test_custom_chunk_size() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let chunks_small: Vec<_> = read_stream_chunked(path.to_str().unwrap(), 10)
        .unwrap()
        .collect();
    let chunks_large: Vec<_> = read_stream_chunked(path.to_str().unwrap(), 100000)
        .unwrap()
        .collect();

    assert!(
        chunks_small.len() >= chunks_large.len(),
        "smaller chunk size should produce more chunks"
    );
}

#[test]
fn test_chunk_index_sequential() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let reader = read_stream_chunked(path.to_str().unwrap(), 100).unwrap();

    for (i, chunk_result) in reader.enumerate() {
        let chunk = chunk_result.unwrap();
        assert_eq!(chunk.chunk_index, i, "chunk index should be sequential");
    }
}

#[test]
fn test_stream_total_points_match() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    // Get full data
    let full_result = read(path.to_str().unwrap()).unwrap();
    let full_data = &full_result.data_tables[0];
    let total_points_full = match full_data.values().next().unwrap() {
        VectorData::Real(v) => v.len(),
        VectorData::Complex(v) => v.len(),
    };

    // Count streamed points
    let reader = read_stream_chunked(path.to_str().unwrap(), 100).unwrap();
    let total_points_stream: usize = reader
        .filter_map(|r| r.ok())
        .map(|chunk| {
            chunk
                .data
                .values()
                .next()
                .map(|v| match v {
                    VectorData::Real(vec) => vec.len(),
                    VectorData::Complex(vec) => vec.len(),
                })
                .unwrap_or(0)
        })
        .sum();

    assert_eq!(
        total_points_stream, total_points_full,
        "streamed points should match full read"
    );
}

#[test]
fn test_stream_time_range_continuous() {
    let path = example_tr0();
    if skip_if_missing(&path) {
        return;
    }

    let reader = read_stream_chunked(path.to_str().unwrap(), 100).unwrap();
    let chunks: Vec<_> = reader.filter_map(|r| r.ok()).collect();

    if chunks.len() > 1 {
        for i in 0..chunks.len() - 1 {
            let current_end = chunks[i].time_range.1;
            let next_start = chunks[i + 1].time_range.0;

            assert!(
                next_start >= current_end,
                "chunk {} end ({}) should be <= chunk {} start ({})",
                i,
                current_end,
                i + 1,
                next_start
            );
        }
    }
}

// =============================================================================
// Test: Conversion
// =============================================================================

#[test]
fn test_convert_to_raw() {
    let input = example_tr0();
    if skip_if_missing(&input) {
        return;
    }

    let output = std::env::temp_dir().join("hspice_test_output.raw");

    let result = read_and_convert(input.to_str().unwrap(), output.to_str().unwrap());

    assert!(result.is_ok(), "conversion should succeed");
    assert!(output.exists(), "output file should exist");

    // Cleanup
    let _ = std::fs::remove_file(&output);
}

#[test]
fn test_convert_creates_valid_file() {
    let input = example_tr0();
    if skip_if_missing(&input) {
        return;
    }

    let output = std::env::temp_dir().join("hspice_test_output2.raw");

    read_and_convert(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

    // Check file is not empty
    let metadata = std::fs::metadata(&output).unwrap();
    assert!(metadata.len() > 0, "output file should not be empty");

    // Check it starts with "Title" (SPICE3 raw header is text until "Binary:")
    let content = std::fs::read(&output).unwrap();
    let header = String::from_utf8_lossy(&content[..100.min(content.len())]);
    assert!(
        header.starts_with("Title"),
        "should start with Title header"
    );

    // Cleanup
    let _ = std::fs::remove_file(&output);
}
