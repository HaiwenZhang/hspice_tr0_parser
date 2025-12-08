//! Common types, errors, and constants for HSPICE file operations

use num_complex::Complex64;
use std::collections::HashMap;

#[cfg(feature = "python")]
use pyo3::prelude::*;

// ============================================================================
// Constants
// ============================================================================

/// Header character positions (matching C implementation)
pub const NUM_OF_VARIABLES_POSITION: usize = 0;
pub const NUM_OF_PROBES_POSITION: usize = 4;
pub const NUM_OF_SWEEPS_POSITION: usize = 8;
pub const NUM_OF_SWEEPS_END_POSITION: usize = 12;
pub const POST_START_POSITION1: usize = 16;
pub const POST_START_POSITION2: usize = 20;
pub const DATE_START_POSITION: usize = 88;
pub const DATE_END_POSITION: usize = 112;
pub const TITLE_START_POSITION: usize = 24;
pub const SWEEP_SIZE_POSITION1: usize = 176;
pub const SWEEP_SIZE_POSITION2: usize = 187;
pub const VECTOR_DESCRIPTION_START_POSITION: usize = 256;

pub const POST_STRING11: &str = "9007";
pub const POST_STRING12: &str = "9601";
pub const POST_STRING21: &str = "2001";

pub const FREQUENCY_TYPE: i32 = 2;
pub const COMPLEX_VAR: i32 = 1;
pub const REAL_VAR: i32 = 0;

/// End-of-data marker for 9601 format (float32 representation of ~1e30)
pub const END_MARKER_9601: f32 = 1.0000000150474662e+30_f32;
/// End-of-data marker for 2001 format
pub const END_MARKER_2001: f64 = 1.0e+30_f64;

// ============================================================================
// Enums
// ============================================================================

/// Endianness detected from file
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endian {
    Little,
    Big,
}

/// Post format version - determines data precision
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostVersion {
    /// 9007/9601 format: 4-byte float32
    V9601,
    /// 2001 format: 8-byte float64 (double precision)
    V2001,
}

/// Vector data - either real or complex
#[derive(Debug)]
pub enum VectorData {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}

// ============================================================================
// Error Types
// ============================================================================

/// Error type for HSPICE reading operations
#[derive(Debug)]
pub enum HspiceError {
    IoError(std::io::Error),
    ParseError(String),
    FormatError(String),
}

impl From<std::io::Error> for HspiceError {
    fn from(e: std::io::Error) -> Self {
        HspiceError::IoError(e)
    }
}

#[cfg(feature = "python")]
impl From<HspiceError> for pyo3::PyErr {
    fn from(e: HspiceError) -> pyo3::PyErr {
        match e {
            HspiceError::IoError(e) => pyo3::exceptions::PyIOError::new_err(e.to_string()),
            HspiceError::ParseError(s) => pyo3::exceptions::PyValueError::new_err(s),
            HspiceError::FormatError(s) => pyo3::exceptions::PyValueError::new_err(s),
        }
    }
}

pub type Result<T> = std::result::Result<T, HspiceError>;

// ============================================================================
// Data Structures
// ============================================================================

/// Result of HSPICE file read
pub struct HspiceResult {
    pub sweep_name: Option<String>,
    pub sweep_values: Option<Vec<f64>>,
    pub data_tables: Vec<HashMap<String, VectorData>>,
    pub scale_name: String,
    pub title: String,
    pub date: String,
}
