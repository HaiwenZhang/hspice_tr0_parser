//! # HSPICE Binary File Reader - Core Library
//!
//! A high-performance library for reading HSPICE binary output files (.tr0, .ac0, .sw0).
//!
//! ## Features
//!
//! - Memory-mapped file I/O for efficient large file handling
//! - Support for both 9601 (float32) and 2001 (float64) formats
//! - Streaming reader for processing very large files
//! - Format conversion to SPICE3 binary raw format
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hspice_core::{read, VectorData};
//!
//! let result = read("simulation.tr0").unwrap();
//! println!("Title: {}", result.title);
//!
//! for table in &result.data_tables {
//!     for (name, data) in table {
//!         match data {
//!             VectorData::Real(vec) => println!("{}: {} points", name, vec.len()),
//!             VectorData::Complex(vec) => println!("{}: {} complex", name, vec.len()),
//!         }
//!     }
//! }
//! ```
//!
//! ## Streaming for Large Files
//!
//! ```rust,no_run
//! use hspice_core::read_stream;
//!
//! let reader = read_stream("large_file.tr0").unwrap();
//! for chunk in reader {
//!     let chunk = chunk.unwrap();
//!     println!("Chunk: {} points", chunk.time.len());
//! }
//! ```

mod parser;
mod reader;
mod stream;
mod types;
mod writer;

// Re-export public types
pub use types::{
    Endian,
    HspiceError,
    HspiceResult,
    PostVersion,
    Result,
    VectorData,
    // Constants
    COMPLEX_VAR,
    END_MARKER_2001,
    END_MARKER_9601,
    FREQUENCY_TYPE,
    REAL_VAR,
};

// Re-export streaming types
pub use stream::{
    read_stream, read_stream_chunked, read_stream_signals, DataChunk, HspiceStreamReader,
    StreamMetadata, DEFAULT_CHUNK_SIZE,
};

// Re-export writer
pub use writer::write_spice3_raw;

// ============================================================================
// Public API Functions
// ============================================================================

/// Read an HSPICE binary file.
///
/// # Arguments
/// * `filename` - Path to the HSPICE binary file (.tr0, .ac0, .sw0)
///
/// # Returns
/// * `Ok(HspiceResult)` - Parsed simulation data
/// * `Err(HspiceError)` - If file cannot be read or parsed
///
/// # Example
/// ```rust,no_run
/// let result = hspice_core::read("simulation.tr0").unwrap();
/// println!("Title: {}", result.title);
/// ```
pub fn read(filename: &str) -> Result<HspiceResult> {
    parser::hspice_read_impl(filename, 0)
}

/// Read an HSPICE binary file with debug output.
///
/// # Arguments
/// * `filename` - Path to the HSPICE binary file
/// * `debug` - Debug level (0=quiet, 1=info, 2=verbose)
pub fn read_debug(filename: &str, debug: i32) -> Result<HspiceResult> {
    parser::hspice_read_impl(filename, debug)
}

/// Convert an HSPICE binary file to SPICE3 raw format.
///
/// # Arguments
/// * `input_path` - Path to the input HSPICE file
/// * `output_path` - Path for the output SPICE3 .raw file
///
/// # Returns
/// * `Ok(())` - Conversion successful
/// * `Err(HspiceError)` - If conversion fails
pub fn read_and_convert(input_path: &str, output_path: &str) -> Result<()> {
    writer::hspice_to_raw_impl(input_path, output_path, 0)
}

/// Convert an HSPICE binary file to SPICE3 raw format with debug output.
pub fn read_and_convert_debug(input_path: &str, output_path: &str, debug: i32) -> Result<()> {
    writer::hspice_to_raw_impl(input_path, output_path, debug)
}

// Re-export header parsing for advanced use
pub use parser::{parse_header_only, HeaderMetadata};
