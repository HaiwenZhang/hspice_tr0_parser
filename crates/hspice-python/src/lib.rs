//! Python bindings for HSPICE binary file parser
//!
//! This crate provides PyO3 bindings to expose hspice-core to Python.

use hspice_core::{self, HspiceResult, VectorData};
use numpy::ndarray::Array1;
use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;

/// Build the result tuple structure (shared logic for numpy and native)
fn build_result_structure<'py, F, G>(
    py: Python<'py>,
    result: HspiceResult,
    convert_table: F,
    convert_sweep: G,
) -> PyResult<Py<PyAny>>
where
    F: Fn(Python<'py>, HashMap<String, VectorData>) -> PyResult<Bound<'py, PyDict>>,
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
    table: HashMap<String, VectorData>,
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
    table: HashMap<String, VectorData>,
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
    match hspice_core::read_debug(filename, debug) {
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
    match hspice_core::read_debug(filename, debug) {
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
pub fn tr0_to_raw(_py: Python, input_path: &str, output_path: &str, debug: i32) -> PyResult<bool> {
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

/// Streaming reader for large files
/// Returns a list of chunks, each containing a subset of the data
#[pyfunction]
#[pyo3(signature = (filename, chunk_size=10000, signals=None, debug=0))]
pub fn tr0_stream(
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

    // Collect all chunks into a list
    let chunks_list = PyList::empty(py);

    for chunk_result in reader {
        match chunk_result {
            Ok(chunk) => {
                let chunk_dict = PyDict::new(py);
                chunk_dict.set_item("chunk_index", chunk.chunk_index)?;
                chunk_dict.set_item("time_range", (chunk.time_range.0, chunk.time_range.1))?;

                // Convert data to Python dict with numpy arrays
                let data_dict = PyDict::new(py);
                for (name, vector) in chunk.data {
                    let arr = match vector {
                        VectorData::Real(v) => Array1::from_vec(v).into_pyarray(py).into_any(),
                        VectorData::Complex(v) => {
                            // Convert complex to magnitude for numpy compatibility
                            let mags: Vec<f64> = v
                                .iter()
                                .map(|c| (c.re * c.re + c.im * c.im).sqrt())
                                .collect();
                            Array1::from_vec(mags).into_pyarray(py).into_any()
                        }
                    };
                    data_dict.set_item(name, arr)?;
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

#[pymodule]
pub fn hspicetr0parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tr0_read_numpy, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_read_native, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_to_raw, m)?)?;
    m.add_function(wrap_pyfunction!(tr0_stream, m)?)?;
    Ok(())
}
