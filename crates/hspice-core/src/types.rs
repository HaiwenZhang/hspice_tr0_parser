//! Common types, errors, and constants for waveform file operations
//!
//! This module provides unified data structures for parsing various SPICE
//! waveform formats including HSPICE TR0 and SPICE3 raw files.

use num_complex::Complex64;

// ============================================================================
// Constants (HSPICE format specific)
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
#[allow(clippy::excessive_precision)]
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

/// Generate endian-aware read methods
macro_rules! impl_endian_read {
    ($fn_name:ident, $ty:ty) => {
        #[inline]
        pub fn $fn_name(&self, bytes: [u8; std::mem::size_of::<$ty>()]) -> $ty {
            match self {
                Endian::Little => <$ty>::from_le_bytes(bytes),
                Endian::Big => <$ty>::from_be_bytes(bytes),
            }
        }
    };
}

impl Endian {
    impl_endian_read!(read_i32, i32);
    impl_endian_read!(read_f32, f32);
    impl_endian_read!(read_f64, f64);
}

/// Post format version - determines data precision
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostVersion {
    /// 9007/9601 format: 4-byte float32
    V9601,
    /// 2001 format: 8-byte float64 (double precision)
    V2001,
}

/// Analysis/simulation type
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AnalysisType {
    /// Transient analysis (.tr0)
    Transient,
    /// AC frequency analysis (.ac0)
    AC,
    /// DC sweep analysis (.sw0)
    DC,
    /// Operating point
    Operating,
    /// Noise analysis
    Noise,
    /// Unknown or unrecognized
    #[default]
    Unknown,
}

impl AnalysisType {
    /// Infer analysis type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "tr0" => AnalysisType::Transient,
            "ac0" => AnalysisType::AC,
            "sw0" => AnalysisType::DC,
            _ => AnalysisType::Unknown,
        }
    }

    /// Infer analysis type from scale name
    pub fn from_scale_name(name: &str) -> Self {
        match name.to_uppercase().as_str() {
            "TIME" => AnalysisType::Transient,
            "HERTZ" | "FREQ" | "FREQUENCY" => AnalysisType::AC,
            _ => AnalysisType::DC, // DC sweep uses parameter name as scale
        }
    }
}

// ============================================================================
// Standard Trait Implementations for AnalysisType
// ============================================================================

impl std::fmt::Display for AnalysisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AnalysisType::Transient => "transient",
            AnalysisType::AC => "ac",
            AnalysisType::DC => "dc",
            AnalysisType::Operating => "operating",
            AnalysisType::Noise => "noise",
            AnalysisType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for AnalysisType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "transient" | "tran" => AnalysisType::Transient,
            "ac" => AnalysisType::AC,
            "dc" => AnalysisType::DC,
            "operating" | "op" => AnalysisType::Operating,
            "noise" => AnalysisType::Noise,
            _ => AnalysisType::Unknown,
        })
    }
}

/// Variable type (voltage, current, time, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VarType {
    /// Time variable (scale for transient)
    Time,
    /// Frequency variable (scale for AC)
    Frequency,
    /// Voltage signal
    Voltage,
    /// Current signal
    Current,
    /// Unknown or other type
    #[default]
    Unknown,
}

impl VarType {
    /// Infer variable type from signal name
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower == "time" {
            VarType::Time
        } else if lower == "hertz" || lower == "freq" || lower == "frequency" {
            VarType::Frequency
        } else if lower.starts_with("v(") || lower.starts_with("v_") {
            VarType::Voltage
        } else if lower.starts_with("i(") || lower.starts_with("i_") {
            VarType::Current
        } else {
            VarType::Unknown
        }
    }
}

// ============================================================================
// Standard Trait Implementations for VarType
// ============================================================================

impl std::fmt::Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            VarType::Time => "time",
            VarType::Frequency => "frequency",
            VarType::Voltage => "voltage",
            VarType::Current => "current",
            VarType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for VarType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::from_name(s))
    }
}

/// Vector data - either real or complex
#[derive(Debug, Clone)]
pub enum VectorData {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}

impl VectorData {
    /// Get the number of data points
    pub fn len(&self) -> usize {
        match self {
            VectorData::Real(v) => v.len(),
            VectorData::Complex(v) => v.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if this is complex data
    pub fn is_complex(&self) -> bool {
        matches!(self, VectorData::Complex(_))
    }

    /// Get real data, returns None if complex
    pub fn as_real(&self) -> Option<&Vec<f64>> {
        match self {
            VectorData::Real(v) => Some(v),
            VectorData::Complex(_) => None,
        }
    }

    /// Get complex data, returns None if real
    pub fn as_complex(&self) -> Option<&Vec<Complex64>> {
        match self {
            VectorData::Real(_) => None,
            VectorData::Complex(v) => Some(v),
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Error type for waveform reading operations
#[derive(Debug, thiserror::Error)]
pub enum WaveformError {
    /// I/O error (file not found, permission denied, etc.)
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Parse error (invalid data format, unexpected values)
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Format error (unsupported file format, version mismatch)
    #[error("Format error: {0}")]
    FormatError(String),
}

pub type Result<T> = std::result::Result<T, WaveformError>;

// Keep old error name as alias for compatibility during transition
pub type HspiceError = WaveformError;

// ============================================================================
// Core Data Structures
// ============================================================================

/// Metadata for a single variable/signal
#[derive(Debug, Clone)]
pub struct Variable {
    /// Signal name (e.g., "TIME", "v(out)", "i(vdd)")
    pub name: String,
    /// Variable type inferred from name
    pub var_type: VarType,
}

impl Variable {
    /// Create a new variable with type inferred from name
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let var_type = VarType::from_name(&name);
        Self { name, var_type }
    }

    /// Create a new variable with explicit type
    pub fn with_type(name: impl Into<String>, var_type: VarType) -> Self {
        Self {
            name: name.into(),
            var_type,
        }
    }
}

/// A single data table (one per sweep point, or one if no sweep)
#[derive(Debug, Clone)]
pub struct DataTable {
    /// Sweep parameter value (None if no sweep)
    pub sweep_value: Option<f64>,
    /// Data vectors in variable order (index matches variables Vec)
    pub vectors: Vec<VectorData>,
}

impl DataTable {
    /// Get number of data points
    pub fn len(&self) -> usize {
        self.vectors.first().map(|v| v.len()).unwrap_or(0)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty() || self.len() == 0
    }
}

/// Waveform simulation result - format independent
///
/// This is the unified result type for all supported waveform formats:
/// - HSPICE binary (.tr0, .ac0, .sw0)
/// - SPICE3 raw (binary and ASCII)
///
/// # Structure
///
/// - `variables`: Ordered list of signal metadata. Index 0 is always the scale
///   variable (TIME for transient, HERTZ for AC, etc.)
/// - `tables`: One table per sweep point. Each table contains vectors in the
///   same order as `variables`.
///
/// # Example
///
/// ```rust,no_run
/// use hspice_core::read;
///
/// let result = read("simulation.tr0").unwrap();
/// println!("Title: {}", result.title);
/// println!("Analysis: {:?}", result.analysis);
///
/// // Access by name
/// if let Some(time) = result.get("TIME") {
///     println!("Time points: {}", time.len());
/// }
///
/// // Access by index (faster)
/// let scale = &result.tables[0].vectors[0];
/// ```
#[derive(Debug)]
pub struct WaveformResult {
    // === Metadata ===
    /// Simulation title
    pub title: String,
    /// Simulation date
    pub date: String,
    /// Analysis type (Transient, AC, DC, etc.)
    pub analysis: AnalysisType,

    // === Variable Definitions ===
    /// Ordered list of variables. Index 0 is the scale variable.
    pub variables: Vec<Variable>,

    // === Sweep Information ===
    /// Sweep parameter name (None if no sweep)
    pub sweep_param: Option<String>,

    // === Data ===
    /// Data tables (one per sweep point)
    pub tables: Vec<DataTable>,
}

impl WaveformResult {
    /// Get the scale variable name (first variable)
    pub fn scale_name(&self) -> &str {
        self.variables
            .first()
            .map(|v| v.name.as_str())
            .unwrap_or("")
    }

    /// Get variable index by name
    pub fn var_index(&self, name: &str) -> Option<usize> {
        self.variables.iter().position(|v| v.name == name)
    }

    /// Get signal data by name (from first table)
    pub fn get(&self, name: &str) -> Option<&VectorData> {
        self.var_index(name)
            .and_then(|i| self.tables.first().map(|t| &t.vectors[i]))
    }

    /// Get scale data (first variable of first table)
    pub fn scale(&self) -> Option<&VectorData> {
        self.tables.first().and_then(|t| t.vectors.first())
    }

    /// Get number of data points
    pub fn len(&self) -> usize {
        self.tables.first().map(|t| t.len()).unwrap_or(0)
    }

    /// Check if result is empty
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty() || self.len() == 0
    }

    /// Get number of variables
    pub fn num_vars(&self) -> usize {
        self.variables.len()
    }

    /// Get number of sweep points (tables)
    pub fn num_sweeps(&self) -> usize {
        self.tables.len()
    }

    /// Get all variable names
    pub fn var_names(&self) -> Vec<&str> {
        self.variables.iter().map(|v| v.name.as_str()).collect()
    }

    /// Check if result has sweep data
    pub fn has_sweep(&self) -> bool {
        self.sweep_param.is_some() && self.tables.len() > 1
    }
}

// Keep old name as alias during transition
pub type HspiceResult = WaveformResult;
