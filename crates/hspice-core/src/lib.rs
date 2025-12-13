//! # Waveform File Reader - Core Library
//!
//! A high-performance library for reading SPICE waveform files.
//!
//! ## Supported Formats
//!
//! - HSPICE binary (.tr0, .ac0, .sw0)
//! - (Future) SPICE3 raw (binary and ASCII)
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
//! println!("Analysis: {:?}", result.analysis);
//!
//! // Access by name
//! if let Some(time) = result.get("TIME") {
//!     println!("Time points: {}", time.len());
//! }
//!
//! // Access by index (faster)
//! for (i, var) in result.variables.iter().enumerate() {
//!     println!("{}: {:?}", var.name, var.var_type);
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
//!     println!("Chunk {}: {:?}", chunk.chunk_index, chunk.time_range);
//! }
//! ```

mod parser;
mod reader;
mod stream;
mod types;
mod writer;

// Re-export public types
pub use types::{
    // Core result types
    AnalysisType,
    DataTable,
    // Endianness
    Endian,
    // Aliases for compatibility
    HspiceError,
    HspiceResult,
    PostVersion,
    // Error types
    Result,
    VarType,
    Variable,
    VectorData,
    WaveformError,
    WaveformResult,
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

/// Read a waveform file.
///
/// # Arguments
/// * `filename` - Path to the waveform file (.tr0, .ac0, .sw0)
///
/// # Returns
/// * `Ok(WaveformResult)` - Parsed simulation data
/// * `Err(WaveformError)` - If file cannot be read or parsed
///
/// # Example
/// ```rust,no_run
/// let result = hspice_core::read("simulation.tr0").unwrap();
/// println!("Title: {}", result.title);
/// println!("Scale: {}", result.scale_name());
///
/// // Access signal by name
/// if let Some(vout) = result.get("v(out)") {
///     println!("v(out): {} points", vout.len());
/// }
/// ```
pub fn read(filename: &str) -> Result<WaveformResult> {
    parser::hspice_read_impl(filename, 0)
}

/// Read a waveform file with debug output.
///
/// # Arguments
/// * `filename` - Path to the waveform file
/// * `debug` - Debug level (0=quiet, 1=info, 2=verbose)
pub fn read_debug(filename: &str, debug: i32) -> Result<WaveformResult> {
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
/// * `Err(WaveformError)` - If conversion fails
pub fn read_and_convert(input_path: &str, output_path: &str) -> Result<()> {
    writer::hspice_to_raw_impl(input_path, output_path, 0)
}

/// Convert an HSPICE binary file to SPICE3 raw format with debug output.
pub fn read_and_convert_debug(input_path: &str, output_path: &str, debug: i32) -> Result<()> {
    writer::hspice_to_raw_impl(input_path, output_path, debug)
}

// Re-export header parsing for advanced use
pub use parser::{parse_header_only, HeaderMetadata};
