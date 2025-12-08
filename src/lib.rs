//! # HSPICE Binary File Reader
//!
//! A high-performance library for reading HSPICE binary output files (.tr0, .ac0, .sw0).
//!
//! ## Features
//!
//! - Memory-mapped file I/O for efficient large file handling
//! - Support for both 9601 (float32) and 2001 (float64) formats
//! - Real and complex data support
//! - Optional Python bindings via PyO3
//!
//! ## Rust API Usage
//!
//! ```rust,no_run
//! use hspicetr0parser::{read, read_and_convert};
//!
//! // Read HSPICE file
//! let result = read("simulation.tr0").unwrap();
//! println!("Title: {}", result.title);
//! println!("Scale: {}", result.scale_name);
//!
//! // Access data
//! for table in &result.data_tables {
//!     for (name, data) in table {
//!         println!("Signal: {}", name);
//!     }
//! }
//!
//! // Convert to SPICE3 raw format
//! read_and_convert("input.tr0", "output.raw").unwrap();
//! ```
//!
//! ## Python API
//!
//! When built with the `python` feature (default), provides Python bindings.

pub mod ffi;
mod parser;
mod reader;
pub mod types;
mod writer;

// Re-export core types for Rust API
pub use types::{Endian, HspiceError, HspiceResult, PostVersion, Result, VectorData};

// ============================================================================
// Rust Public API
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
/// let result = hspicetr0parser::read("simulation.tr0").unwrap();
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

// ============================================================================
// Python Bindings (conditional compilation)
// ============================================================================

#[cfg(feature = "python")]
mod python_bindings {
    use super::*;
    use numpy::ndarray::Array1;
    use numpy::IntoPyArray;
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyTuple};

    /// Build the result tuple structure (shared logic for numpy and native)
    fn build_result_structure<'py, F, G>(
        py: Python<'py>,
        result: HspiceResult,
        convert_table: F,
        convert_sweep: G,
    ) -> PyResult<Py<PyAny>>
    where
        F: Fn(
            Python<'py>,
            std::collections::HashMap<String, VectorData>,
        ) -> PyResult<Bound<'py, PyDict>>,
        G: Fn(Python<'py>, Vec<f64>) -> PyResult<Py<PyAny>>,
    {
        let data_list = PyList::empty(py);
        for table in result.data_tables {
            let dict = convert_table(py, table)?;
            data_list.append(dict)?;
        }

        let sweep_values_obj: Py<PyAny> = match result.sweep_values {
            Some(vec) => convert_sweep(py, vec)?,
            None => py.None(),
        };

        let sweep_name_obj: Py<PyAny> = match result.sweep_name {
            Some(name) => name.into_pyobject(py)?.into_any().unbind(),
            None => py.None(),
        };

        let sweeps = PyTuple::new(
            py,
            &[
                sweep_name_obj.bind(py),
                sweep_values_obj.bind(py),
                data_list.as_any(),
            ],
        )?;

        let main_tuple = PyTuple::new(
            py,
            &[
                sweeps.as_any(),
                result.scale_name.into_pyobject(py)?.as_any(),
                py.None().bind(py),
                result.title.into_pyobject(py)?.as_any(),
                result.date.into_pyobject(py)?.as_any(),
                py.None().bind(py),
            ],
        )?;

        let return_list = PyList::empty(py);
        return_list.append(main_tuple)?;

        Ok(return_list.unbind().into_any())
    }

    fn convert_table_numpy<'py>(
        py: Python<'py>,
        table: std::collections::HashMap<String, VectorData>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (name, data) in table {
            match data {
                VectorData::Real(vec) => {
                    let arr = Array1::from_vec(vec);
                    dict.set_item(name, arr.into_pyarray(py))?;
                }
                VectorData::Complex(vec) => {
                    let arr = Array1::from_vec(vec);
                    dict.set_item(name, arr.into_pyarray(py))?;
                }
            }
        }
        Ok(dict)
    }

    fn convert_table_native<'py>(
        py: Python<'py>,
        table: std::collections::HashMap<String, VectorData>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (name, data) in table {
            match data {
                VectorData::Real(vec) => {
                    dict.set_item(name, PyList::new(py, vec)?)?;
                }
                VectorData::Complex(vec) => {
                    let tuples: Vec<_> = vec.iter().map(|c| (c.re, c.im)).collect();
                    dict.set_item(name, PyList::new(py, tuples)?)?;
                }
            }
        }
        Ok(dict)
    }

    fn result_to_python_numpy(py: Python, result: HspiceResult) -> PyResult<Py<PyAny>> {
        build_result_structure(py, result, convert_table_numpy, |py, vec| {
            let arr = Array1::from_vec(vec);
            Ok(arr.into_pyarray(py).into_any().unbind())
        })
    }

    fn result_to_python_native(py: Python, result: HspiceResult) -> PyResult<Py<PyAny>> {
        build_result_structure(py, result, convert_table_native, |py, vec| {
            Ok(PyList::new(py, vec)?.into_any().unbind())
        })
    }

    #[pyfunction]
    #[pyo3(signature = (filename, debug=0))]
    pub fn tr0_read_numpy(py: Python, filename: &str, debug: i32) -> PyResult<Py<PyAny>> {
        match parser::hspice_read_impl(filename, debug) {
            Ok(result) => result_to_python_numpy(py, result),
            Err(e) => {
                if debug > 0 {
                    eprintln!("HSpiceRead error: {:?}", e);
                }
                Ok(py.None())
            }
        }
    }

    #[pyfunction]
    #[pyo3(signature = (filename, debug=0))]
    pub fn tr0_read_native(py: Python, filename: &str, debug: i32) -> PyResult<Py<PyAny>> {
        match parser::hspice_read_impl(filename, debug) {
            Ok(result) => result_to_python_native(py, result),
            Err(e) => {
                if debug > 0 {
                    eprintln!("HSpiceRead error: {:?}", e);
                }
                Ok(py.None())
            }
        }
    }

    #[pyfunction]
    #[pyo3(signature = (input_path, output_path, debug=0))]
    pub fn tr0_to_raw(
        _py: Python,
        input_path: &str,
        output_path: &str,
        debug: i32,
    ) -> PyResult<bool> {
        match writer::hspice_to_raw_impl(input_path, output_path, debug) {
            Ok(()) => Ok(true),
            Err(e) => {
                if debug > 0 {
                    eprintln!("Conversion error: {:?}", e);
                }
                Ok(false)
            }
        }
    }

    #[pymodule]
    pub fn hspicetr0parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(tr0_read_numpy, m)?)?;
        m.add_function(wrap_pyfunction!(tr0_read_native, m)?)?;
        m.add_function(wrap_pyfunction!(tr0_to_raw, m)?)?;
        Ok(())
    }
}

#[cfg(feature = "python")]
pub use python_bindings::*;
