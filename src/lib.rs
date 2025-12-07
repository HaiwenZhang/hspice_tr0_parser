//! HSPICE Binary File Reader - Optimized for Large Files
//!
//! Rust implementation of HSPICE binary (.tr0) file parser with PyO3 bindings.
//! Optimized for parsing very large files (GB scale) using:
//! - Memory-mapped file I/O
//! - Bulk byte-to-float conversion
//! - Single-pass reading with capacity estimation
//!
//! Based on the original C implementation by Janez Puhan (PyOPUS project).

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

/// Convert HspiceResult to Python objects
fn result_to_python(py: Python, result: HspiceResult) -> PyResult<Py<PyAny>> {
    let data_list = PyList::empty(py);

    for table in result.data_tables {
        let dict = PyDict::new(py);

        for (name, data) in table {
            match data {
                VectorData::Real(vec) => {
                    let arr = Array1::from_vec(vec);
                    let py_arr = arr.into_pyarray(py);
                    dict.set_item(name, py_arr)?;
                }
                VectorData::Complex(vec) => {
                    let arr = Array1::from_vec(vec);
                    let py_arr = arr.into_pyarray(py);
                    dict.set_item(name, py_arr)?;
                }
            }
        }

        data_list.append(dict)?;
    }

    let sweep_values_obj: Py<PyAny> = match result.sweep_values {
        Some(vec) => {
            let arr = Array1::from_vec(vec);
            arr.into_pyarray(py).into_any().unbind()
        }
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

/// Python-exposed hspice tr0 file read function
///
/// Read HSPICE binary .tr0 file and return data as Python objects.
///
/// # Arguments
/// * `filename` - Path to the .tr0 file
/// * `debug` - Debug level (0=quiet, 1=info, 2=verbose)
///
/// # Returns
/// A list containing simulation results, or None on error.
#[pyfunction]
#[pyo3(signature = (filename, debug=0))]
fn tr0_read(py: Python, filename: &str, debug: i32) -> PyResult<Py<PyAny>> {
    match hspice_read_impl(filename, debug) {
        Ok(result) => result_to_python(py, result),
        Err(e) => {
            if debug > 0 {
                eprintln!("HSpiceRead error: {:?}", e);
            }
            Ok(py.None())
        }
    }
}

/// Python-exposed function to convert HSPICE .tr0 to SPICE3 raw format
///
/// # Arguments
/// * `input_path` - Path to the input .tr0 file
/// * `output_path` - Path for the output .raw file
/// * `debug` - Debug level (0=quiet, 1=info, 2=verbose)
///
/// # Returns
/// True on success, False on error.
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
fn _tr0_parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tr0_read, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_to_raw, m)?)?;
    Ok(())
}
