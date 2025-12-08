//! HSPICE Binary File Reader - Optimized for Large Files
//!
//! Rust implementation of HSPICE binary (.tr0) file parser with PyO3 bindings.
//! Optimized for parsing very large files (GB scale) using:
//! - Memory-mapped file I/O
//! - Bulk byte-to-float conversion
//! - Single-pass reading with capacity estimation
//!

mod parser;
mod reader;
mod types;
mod writer;

use numpy::ndarray::Array1;
use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

use parser::hspice_read_impl;
use types::{HspiceResult, VectorData};
use writer::hspice_to_raw_impl;

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
    // Convert data tables
    let data_list = PyList::empty(py);
    for table in result.data_tables {
        let dict = convert_table(py, table)?;
        data_list.append(dict)?;
    }

    // Convert sweep values
    let sweep_values_obj: Py<PyAny> = match result.sweep_values {
        Some(vec) => convert_sweep(py, vec)?,
        None => py.None(),
    };

    let sweep_name_obj: Py<PyAny> = match result.sweep_name {
        Some(name) => name.into_pyobject(py)?.into_any().unbind(),
        None => py.None(),
    };

    // Build tuples
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

/// Convert data table to PyDict with NumPy arrays
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

/// Convert data table to PyDict with Python lists
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

/// Convert HspiceResult to Python objects with NumPy arrays
fn result_to_python_numpy(py: Python, result: HspiceResult) -> PyResult<Py<PyAny>> {
    build_result_structure(py, result, convert_table_numpy, |py, vec| {
        let arr = Array1::from_vec(vec);
        Ok(arr.into_pyarray(py).into_any().unbind())
    })
}

/// Convert HspiceResult to Python objects with native Python lists
fn result_to_python_native(py: Python, result: HspiceResult) -> PyResult<Py<PyAny>> {
    build_result_structure(py, result, convert_table_native, |py, vec| {
        Ok(PyList::new(py, vec)?.into_any().unbind())
    })
}

// ============================================================================
// PyO3 exposed functions
// ============================================================================

/// Read HSPICE binary .tr0 file and return data as NumPy arrays.
#[pyfunction]
#[pyo3(signature = (filename, debug=0))]
fn tr0_read_numpy(py: Python, filename: &str, debug: i32) -> PyResult<Py<PyAny>> {
    match hspice_read_impl(filename, debug) {
        Ok(result) => result_to_python_numpy(py, result),
        Err(e) => {
            if debug > 0 {
                eprintln!("HSpiceRead error: {:?}", e);
            }
            Ok(py.None())
        }
    }
}

/// Read HSPICE binary .tr0 file and return data as Python lists.
#[pyfunction]
#[pyo3(signature = (filename, debug=0))]
fn tr0_read_native(py: Python, filename: &str, debug: i32) -> PyResult<Py<PyAny>> {
    match hspice_read_impl(filename, debug) {
        Ok(result) => result_to_python_native(py, result),
        Err(e) => {
            if debug > 0 {
                eprintln!("HSpiceRead error: {:?}", e);
            }
            Ok(py.None())
        }
    }
}

/// Convert HSPICE .tr0 to SPICE3 raw format.
#[pyfunction]
#[pyo3(signature = (input_path, output_path, debug=0))]
fn tr0_to_raw(_py: Python, input_path: &str, output_path: &str, debug: i32) -> PyResult<bool> {
    match hspice_to_raw_impl(input_path, output_path, debug) {
        Ok(()) => Ok(true),
        Err(e) => {
            if debug > 0 {
                eprintln!("Conversion error: {:?}", e);
            }
            Ok(false)
        }
    }
}

/// Python module definition
#[pymodule]
fn _hspcie_tr0_parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tr0_read_numpy, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_read_native, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_to_raw, m)?)?;
    Ok(())
}
