//! # Waveform File Reader - Core Library
//!
//! A high-performance library for reading SPICE waveform files.
//!
//! ## Supported Formats
//!
//! - HSPICE binary (.tr0, .ac0, .sw0)
//! - SPICE3/ngspice raw (binary and ASCII)
//!
//! ## Features
//!
//! - Memory-mapped file I/O for efficient large file handling
//! - Support for both 9601 (float32) and 2001 (float64) formats
//! - Streaming reader for processing very large files
//! - Format conversion to SPICE3 binary raw format
//! - Structured logging via `tracing` for diagnostics
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
//!
//! ## Enabling Logging
//!
//! This library uses `tracing` for structured logging. To see log output,
//! initialize a tracing subscriber in your application:
//!
//! ```rust,ignore
//! // Add tracing-subscriber to your Cargo.toml
//! tracing_subscriber::fmt::init();
//!
//! // Now library logs will be visible
//! let result = hspice_core::read("simulation.tr0").unwrap();
//! ```

mod block_reader;
mod parser;
mod raw_parser;
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
    parser::hspice_read_impl(filename)
}

/// Read a waveform file with debug output.
///
/// # Deprecated
/// This function is deprecated. Use `read()` with a tracing subscriber instead.
///
/// # Arguments
/// * `filename` - Path to the waveform file
/// * `debug` - Debug level (ignored, use tracing levels instead)
#[deprecated(since = "1.4.0", note = "Use read() with tracing subscriber instead")]
pub fn read_debug(filename: &str, _debug: i32) -> Result<WaveformResult> {
    parser::hspice_read_impl(filename)
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
    writer::hspice_to_raw_impl(input_path, output_path)
}

/// Convert an HSPICE binary file to SPICE3 raw format with debug output.
///
/// # Deprecated
/// This function is deprecated. Use `read_and_convert()` with a tracing subscriber instead.
#[deprecated(
    since = "1.4.0",
    note = "Use read_and_convert() with tracing subscriber instead"
)]
pub fn read_and_convert_debug(input_path: &str, output_path: &str, _debug: i32) -> Result<()> {
    writer::hspice_to_raw_impl(input_path, output_path)
}

// Re-export header parsing for advanced use
pub use parser::{parse_header_only, HeaderMetadata};

// Re-export SPICE3 raw file reader
pub use raw_parser::{read_raw, read_raw_debug};
