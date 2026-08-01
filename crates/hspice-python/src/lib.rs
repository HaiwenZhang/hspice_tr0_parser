//! Python bindings for the waveform parser.

use std::sync::{Arc, Once};

use hspice_core::{
    self, DataChunk, DataTable, HspiceStreamReader, PostVersion, Variable, VectorData,
    WaveformResult,
};
use numpy::ndarray::Array1;
use numpy::IntoPyArray;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ============================================================================
// Logging Initialization
// ============================================================================

static LOGGING_INIT: Once = Once::new();

/// Initialize logging with specified level.
///
/// Args:
///     level: Log level ("trace", "debug", "info", "warn", "error")
///
/// Example:
///     >>> import hspicetr0parser
///     >>> hspicetr0parser.init_logging("info")
///     >>> result = hspicetr0parser.read("simulation.tr0")
#[pyfunction]
#[pyo3(signature = (level="info"))]
pub fn init_logging(level: &str) -> PyResult<()> {
    use tracing_subscriber::EnvFilter;

    LOGGING_INIT.call_once(|| {
        let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    });

    Ok(())
}

// ============================================================================
// Python Classes
// ============================================================================

/// Python wrapper for Variable
#[pyclass(name = "Variable", skip_from_py_object)]
#[derive(Clone)]
pub struct PyVariable {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub var_type: String,
}

#[pymethods]
impl PyVariable {
    fn __repr__(&self) -> String {
        format!("Variable(name='{}', type='{}')", self.name, self.var_type)
    }
}

impl From<&Variable> for PyVariable {
    fn from(v: &Variable) -> Self {
        PyVariable {
            name: v.name.clone(),
            var_type: v.var_type.to_string(),
        }
    }
}

/// Python wrapper for DataTable
#[pyclass(name = "DataTable")]
pub struct PyDataTable {
    #[pyo3(get)]
    pub sweep_value: Option<f64>,
    table: Arc<DataTable>,
    var_names: Arc<[String]>,
}

#[pymethods]
impl PyDataTable {
    /// Get signal data by name
    fn get<'py>(&self, py: Python<'py>, name: &str) -> Option<Py<PyAny>> {
        let idx = self.var_names.iter().position(|n| n == name)?;
        let vector = self.table.vectors.get(idx)?;
        Some(vector_to_numpy(py, vector))
    }

    /// Get number of data points
    fn __len__(&self) -> usize {
        self.table.len()
    }

    /// Get all signal names
    fn keys(&self) -> Vec<String> {
        self.var_names.to_vec()
    }

    fn __repr__(&self) -> String {
        format!(
            "DataTable(points={}, signals={})",
            self.__len__(),
            self.var_names.len()
        )
    }
}

/// Python wrapper for WaveformResult
#[pyclass(name = "WaveformResult")]
pub struct PyWaveformResult {
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub date: String,
    #[pyo3(get)]
    pub analysis: String,
    #[pyo3(get)]
    pub scale_name: String,
    #[pyo3(get)]
    pub sweep_param: Option<String>,

    variables: Arc<[Variable]>,
    tables: Vec<Arc<DataTable>>,
}

#[pymethods]
impl PyWaveformResult {
    /// Get list of variables
    #[getter]
    fn variables(&self) -> Vec<PyVariable> {
        self.variables.iter().map(PyVariable::from).collect()
    }

    /// Get list of data tables
    #[getter]
    fn tables(&self) -> Vec<PyDataTable> {
        let var_names: Arc<[String]> = self
            .variables
            .iter()
            .map(|variable| variable.name.clone())
            .collect();
        self.tables
            .iter()
            .map(|table| PyDataTable {
                sweep_value: table.sweep_value,
                table: Arc::clone(table),
                var_names: Arc::clone(&var_names),
            })
            .collect()
    }

    /// Get signal data by name (from first table)
    fn get<'py>(&self, py: Python<'py>, name: &str) -> Option<Py<PyAny>> {
        let idx = self.variables.iter().position(|v| v.name == name)?;
        let vector = self.tables.first()?.vectors.get(idx)?;
        Some(vector_to_numpy(py, vector))
    }

    /// Get number of data points
    fn __len__(&self) -> usize {
        self.tables.first().map_or(0, |table| table.len())
    }

    /// Get number of variables
    fn num_vars(&self) -> usize {
        self.variables.len()
    }

    /// Get number of sweep points
    fn num_sweeps(&self) -> usize {
        self.tables.len()
    }

    /// Get all variable names
    fn var_names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.name.clone()).collect()
    }

    /// Check if has sweep data
    fn has_sweep(&self) -> bool {
        self.sweep_param.is_some() && self.tables.len() > 1
    }

    fn __repr__(&self) -> String {
        format!(
            "WaveformResult(title='{}', analysis='{}', vars={}, points={})",
            self.title,
            self.analysis,
            self.variables.len(),
            self.__len__()
        )
    }
}

impl From<WaveformResult> for PyWaveformResult {
    fn from(r: WaveformResult) -> Self {
        let analysis = r.analysis.to_string();
        let scale_name = r.scale_name().to_string();
        PyWaveformResult {
            title: r.title,
            date: r.date,
            analysis,
            scale_name,
            sweep_param: r.sweep_param,
            variables: r.variables.into(),
            tables: r.tables.into_iter().map(Arc::new).collect(),
        }
    }
}

/// Lazy Python iterator over decoded waveform chunks.
#[pyclass(name = "WaveformStream")]
pub struct PyWaveformStream {
    reader: HspiceStreamReader,
}

#[pymethods]
impl PyWaveformStream {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyDict>>> {
        self.reader
            .next()
            .transpose()
            .map_err(core_error_to_python)?
            .map(|chunk| chunk_to_python(py, chunk))
            .transpose()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn vector_to_numpy(py: Python, vector: &VectorData) -> Py<PyAny> {
    match vector {
        VectorData::Real(v) => Array1::from_vec(v.clone())
            .into_pyarray(py)
            .into_any()
            .unbind(),
        VectorData::Complex(v) => Array1::from_vec(v.clone())
            .into_pyarray(py)
            .into_any()
            .unbind(),
    }
}

fn chunk_to_python(py: Python<'_>, chunk: DataChunk) -> PyResult<Py<PyDict>> {
    let chunk_dict = PyDict::new(py);
    chunk_dict.set_item("chunk_index", chunk.chunk_index)?;
    chunk_dict.set_item("time_range", chunk.time_range)?;

    let data_dict = PyDict::new(py);
    for (name, vector) in chunk.data {
        data_dict.set_item(name, vector_to_numpy(py, &vector))?;
    }
    chunk_dict.set_item("data", data_dict)?;
    Ok(chunk_dict.unbind())
}

fn core_error_to_python(error: hspice_core::WaveformError) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

// ============================================================================
// Python Functions
// ============================================================================

/// Read a waveform file
///
/// Args:
///     filename: Path to the waveform file (.tr0, .ac0, .sw0)
///
/// Returns:
///     WaveformResult object or None if failed
#[pyfunction]
#[pyo3(signature = (filename))]
pub fn read(_py: Python, filename: &str) -> PyResult<Option<PyWaveformResult>> {
    match hspice_core::read(filename) {
        Ok(result) => Ok(Some(result.into())),
        Err(e) => {
            tracing::error!("Read error: {:?}", e);
            Ok(None)
        }
    }
}

/// Convert HSPICE file to SPICE3 raw format
#[pyfunction]
#[pyo3(signature = (input_path, output_path))]
pub fn convert_to_raw(_py: Python, input_path: &str, output_path: &str) -> PyResult<bool> {
    match hspice_core::read_and_convert(input_path, output_path) {
        Ok(()) => Ok(true),
        Err(e) => {
            tracing::error!("Conversion error: {:?}", e);
            Ok(false)
        }
    }
}

/// Convert a SPICE3/ngspice raw file to HSPICE binary format.
#[pyfunction]
#[pyo3(signature = (input_path, output_path, post_version="9601"))]
pub fn convert_raw_to_hspice(
    input_path: &str,
    output_path: &str,
    post_version: &str,
) -> PyResult<bool> {
    let post_version = match post_version {
        "9601" => PostVersion::V9601,
        "2001" => PostVersion::V2001,
        value => {
            return Err(PyRuntimeError::new_err(format!(
                "unsupported post_version {value:?}; expected '9601' or '2001'"
            )))
        }
    };
    match hspice_core::convert_raw_to_hspice(input_path, output_path, post_version) {
        Ok(()) => Ok(true),
        Err(error) => {
            tracing::error!("Conversion error: {error:?}");
            Ok(false)
        }
    }
}

/// Stream a large waveform file in chunks
#[pyfunction]
#[pyo3(signature = (filename, chunk_size=10000, signals=None))]
pub fn stream(
    filename: &str,
    chunk_size: usize,
    signals: Option<Vec<String>>,
) -> PyResult<PyWaveformStream> {
    tracing::debug!("Opening stream: {filename} (chunk_size={chunk_size})");

    let reader =
        hspice_core::read_stream_chunked(filename, chunk_size).map_err(core_error_to_python)?;
    let reader = match signals {
        Some(selected) => reader.with_signals(selected),
        None => reader,
    };

    Ok(PyWaveformStream { reader })
}

/// Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format)
///
/// Args:
///     filename: Path to the raw file (.raw)
///
/// Returns:
///     WaveformResult object or None if failed
#[pyfunction]
#[pyo3(signature = (filename))]
pub fn read_raw(_py: Python, filename: &str) -> PyResult<Option<PyWaveformResult>> {
    match hspice_core::read_raw(filename) {
        Ok(result) => Ok(Some(result.into())),
        Err(e) => {
            tracing::error!("Read raw error: {:?}", e);
            Ok(None)
        }
    }
}

// ============================================================================
// Module Definition
// ============================================================================

#[pymodule]
pub fn hspicetr0parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Functions
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(read, m)?)?;
    m.add_function(wrap_pyfunction!(read_raw, m)?)?;
    m.add_function(wrap_pyfunction!(convert_to_raw, m)?)?;
    m.add_function(wrap_pyfunction!(convert_raw_to_hspice, m)?)?;
    m.add_function(wrap_pyfunction!(stream, m)?)?;

    // Classes
    m.add_class::<PyWaveformResult>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyDataTable>()?;
    m.add_class::<PyWaveformStream>()?;

    Ok(())
}
