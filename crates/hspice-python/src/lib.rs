//! Python bindings for waveform file parser
//!
//! This crate provides PyO3 bindings to expose hspice-core to Python.

use hspice_core::{self, AnalysisType, DataTable, VarType, Variable, VectorData, WaveformResult};
use numpy::ndarray::Array1;
use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

// ============================================================================
// Python Classes
// ============================================================================

/// Python wrapper for Variable
#[pyclass(name = "Variable")]
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
        let var_type = match v.var_type {
            VarType::Time => "time",
            VarType::Frequency => "frequency",
            VarType::Voltage => "voltage",
            VarType::Current => "current",
            VarType::Unknown => "unknown",
        };
        PyVariable {
            name: v.name.clone(),
            var_type: var_type.to_string(),
        }
    }
}

/// Python wrapper for DataTable
#[pyclass(name = "DataTable")]
pub struct PyDataTable {
    #[pyo3(get)]
    pub sweep_value: Option<f64>,
    vectors: Vec<VectorData>,
    var_names: Vec<String>,
}

#[pymethods]
impl PyDataTable {
    /// Get signal data by name
    fn get<'py>(&self, py: Python<'py>, name: &str) -> Option<Py<PyAny>> {
        let idx = self.var_names.iter().position(|n| n == name)?;
        let vector = self.vectors.get(idx)?;
        Some(vector_to_numpy(py, vector))
    }

    /// Get number of data points
    fn __len__(&self) -> usize {
        self.vectors.first().map(|v| v.len()).unwrap_or(0)
    }

    /// Get all signal names
    fn keys(&self) -> Vec<String> {
        self.var_names.clone()
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

    variables: Vec<Variable>,
    tables: Vec<DataTable>,
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
        self.tables
            .iter()
            .map(|t| PyDataTable {
                sweep_value: t.sweep_value,
                vectors: t.vectors.clone(),
                var_names: self.variables.iter().map(|v| v.name.clone()).collect(),
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
        self.tables.first().map(|t| t.len()).unwrap_or(0)
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
        let analysis = match r.analysis {
            AnalysisType::Transient => "transient",
            AnalysisType::AC => "ac",
            AnalysisType::DC => "dc",
            AnalysisType::Operating => "operating",
            AnalysisType::Noise => "noise",
            AnalysisType::Unknown => "unknown",
        };
        let scale_name = r.scale_name().to_string();
        PyWaveformResult {
            title: r.title,
            date: r.date,
            analysis: analysis.to_string(),
            scale_name,
            sweep_param: r.sweep_param,
            variables: r.variables,
            tables: r.tables,
        }
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

// ============================================================================
// Python Functions
// ============================================================================

/// Read a waveform file
///
/// Args:
///     filename: Path to the waveform file (.tr0, .ac0, .sw0)
///     debug: Debug level (0=quiet, 1=info, 2=verbose)
///
/// Returns:
///     WaveformResult object or None if failed
#[pyfunction]
#[pyo3(signature = (filename, debug=0))]
pub fn read(_py: Python, filename: &str, debug: i32) -> PyResult<Option<PyWaveformResult>> {
    match hspice_core::read_debug(filename, debug) {
        Ok(result) => Ok(Some(result.into())),
        Err(e) => {
            if debug > 0 {
                eprintln!("Read error: {:?}", e);
            }
            Ok(None)
        }
    }
}

/// Convert HSPICE file to SPICE3 raw format
#[pyfunction]
#[pyo3(signature = (input_path, output_path, debug=0))]
pub fn convert_to_raw(
    _py: Python,
    input_path: &str,
    output_path: &str,
    debug: i32,
) -> PyResult<bool> {
    match hspice_core::read_and_convert_debug(input_path, output_path, debug) {
        Ok(()) => Ok(true),
        Err(e) => {
            if debug > 0 {
                eprintln!("Conversion error: {:?}", e);
            }
            Ok(false)
        }
    }
}

/// Stream a large waveform file in chunks
#[pyfunction]
#[pyo3(signature = (filename, chunk_size=10000, signals=None, debug=0))]
pub fn stream(
    py: Python,
    filename: &str,
    chunk_size: usize,
    signals: Option<Vec<String>>,
    debug: i32,
) -> PyResult<Py<PyList>> {
    use hspice_core::{read_stream_chunked, read_stream_signals};

    if debug > 0 {
        eprintln!("Opening stream: {} (chunk_size={})", filename, chunk_size);
    }

    let reader = if let Some(ref sigs) = signals {
        let sig_refs: Vec<&str> = sigs.iter().map(|s| s.as_str()).collect();
        match read_stream_signals(filename, &sig_refs, chunk_size) {
            Ok(r) => r,
            Err(e) => {
                if debug > 0 {
                    eprintln!("Stream open error: {:?}", e);
                }
                return Ok(PyList::empty(py).unbind());
            }
        }
    } else {
        match read_stream_chunked(filename, chunk_size) {
            Ok(r) => r,
            Err(e) => {
                if debug > 0 {
                    eprintln!("Stream open error: {:?}", e);
                }
                return Ok(PyList::empty(py).unbind());
            }
        }
    };

    let chunks_list = PyList::empty(py);

    for chunk_result in reader {
        match chunk_result {
            Ok(chunk) => {
                let chunk_dict = PyDict::new(py);
                chunk_dict.set_item("chunk_index", chunk.chunk_index)?;
                chunk_dict.set_item("time_range", (chunk.time_range.0, chunk.time_range.1))?;

                let data_dict = PyDict::new(py);
                for (name, vector) in chunk.data {
                    data_dict.set_item(name, vector_to_numpy(py, &vector))?;
                }
                chunk_dict.set_item("data", data_dict)?;

                chunks_list.append(chunk_dict)?;
            }
            Err(e) => {
                if debug > 0 {
                    eprintln!("Stream chunk error: {:?}", e);
                }
                break;
            }
        }
    }

    Ok(chunks_list.unbind())
}

// ============================================================================
// Module Definition
// ============================================================================

#[pymodule]
pub fn hspicetr0parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Functions
    m.add_function(wrap_pyfunction!(read, m)?)?;
    m.add_function(wrap_pyfunction!(convert_to_raw, m)?)?;
    m.add_function(wrap_pyfunction!(stream, m)?)?;

    // Classes
    m.add_class::<PyWaveformResult>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyDataTable>()?;

    Ok(())
}
