//! C Foreign Function Interface (FFI) for HSPICE parser
//!
//! This module provides a C-compatible API for using the HSPICE parser
//! from C, C++, and other languages that support C FFI.
//!
//! # Safety
//!
//! All functions in this module are marked as `unsafe` and require
//! careful handling of pointers for memory safety.

use crate::parser::hspice_read_impl;
use crate::types::{HspiceResult, VectorData};
use std::ffi::{c_char, c_double, c_int, CStr, CString};
use std::ptr;

// ============================================================================
// Opaque Types for C
// ============================================================================

/// Opaque handle to HspiceResult
/// Includes cached CStrings for safe C string returns
#[repr(C)]
pub struct CHspiceResult {
    inner: Box<HspiceResult>,
    // Cached CStrings for safe pointer returns to C
    cached_title: CString,
    cached_date: CString,
    cached_scale_name: CString,
    cached_sweep_name: Option<CString>,
}

/// Opaque handle to a data table (HashMap<String, VectorData>)
#[repr(C)]
pub struct CDataTable {
    inner: std::collections::HashMap<String, VectorData>,
}

/// Opaque handle to signal data
#[repr(C)]
pub struct CSignalData {
    name: CString,
    data: VectorData,
}

// ============================================================================
// Result Creation and Destruction
// ============================================================================

/// Read an HSPICE binary file and return a result handle.
///
/// # Arguments
/// * `filename` - Path to the HSPICE file (null-terminated C string)
/// * `debug` - Debug level (0=quiet, 1=info, 2=verbose)
///
/// # Returns
/// * Pointer to CHspiceResult on success
/// * NULL on error
///
/// # Safety
/// The caller must free the result using `hspice_result_free`.
#[no_mangle]
pub unsafe extern "C" fn hspice_read(filename: *const c_char, debug: c_int) -> *mut CHspiceResult {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let filename_cstr = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match hspice_read_impl(filename_cstr, debug) {
        Ok(result) => {
            // Cache CStrings for safe pointer returns
            let cached_title = CString::new(result.title.clone()).unwrap_or_default();
            let cached_date = CString::new(result.date.clone()).unwrap_or_default();
            let cached_scale_name = CString::new(result.scale_name.clone()).unwrap_or_default();
            let cached_sweep_name = result
                .sweep_name
                .as_ref()
                .and_then(|s| CString::new(s.clone()).ok());

            let boxed = Box::new(CHspiceResult {
                inner: Box::new(result),
                cached_title,
                cached_date,
                cached_scale_name,
                cached_sweep_name,
            });
            Box::into_raw(boxed)
        }
        Err(e) => {
            if debug > 0 {
                eprintln!("hspice_read error: {:?}", e);
            }
            ptr::null_mut()
        }
    }
}

/// Free an HSPICE result handle.
///
/// # Safety
/// Must only be called with a pointer returned by `hspice_read`.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_free(result: *mut CHspiceResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

// ============================================================================
// Metadata Accessors
// ============================================================================

/// Get the title string from the result.
///
/// # Returns
/// Pointer to null-terminated string (valid until result is freed)
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_title(result: *const CHspiceResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    let result = &*result;
    result.cached_title.as_ptr()
}

/// Get the date string from the result.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_date(result: *const CHspiceResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    let result = &*result;
    result.cached_date.as_ptr()
}

/// Get the scale name (e.g., "TIME") from the result.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_scale_name(
    result: *const CHspiceResult,
) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    let result = &*result;
    result.cached_scale_name.as_ptr()
}

/// Get the number of data tables (sweep points).
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_table_count(result: *const CHspiceResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    let result = &*result;
    result.inner.data_tables.len() as c_int
}

// ============================================================================
// Sweep Accessors
// ============================================================================

/// Check if the result has sweep data.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_has_sweep(result: *const CHspiceResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    let result = &*result;
    if result.inner.sweep_name.is_some() {
        1
    } else {
        0
    }
}

/// Get the sweep parameter name.
///
/// # Returns
/// Pointer to null-terminated string, or NULL if no sweep
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_sweep_name(
    result: *const CHspiceResult,
) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    let result = &*result;
    match &result.cached_sweep_name {
        Some(name) => name.as_ptr(),
        None => ptr::null(),
    }
}

/// Get the number of sweep values.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_sweep_count(result: *const CHspiceResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    let result = &*result;
    match &result.inner.sweep_values {
        Some(vec) => vec.len() as c_int,
        None => 0,
    }
}

/// Get sweep values as an array.
///
/// # Arguments
/// * `result` - Result handle
/// * `out_values` - Output buffer for values
/// * `max_count` - Maximum number of values to copy
///
/// # Returns
/// Number of values copied
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_sweep_values(
    result: *const CHspiceResult,
    out_values: *mut c_double,
    max_count: c_int,
) -> c_int {
    if result.is_null() || out_values.is_null() || max_count <= 0 {
        return 0;
    }
    let result = &*result;
    match &result.inner.sweep_values {
        Some(vec) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            for (i, &val) in vec.iter().take(count).enumerate() {
                *out_values.add(i) = val;
            }
            count as c_int
        }
        None => 0,
    }
}

// ============================================================================
// Signal Data Accessors
// ============================================================================

/// Get the number of signals in a data table.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_signal_count(
    result: *const CHspiceResult,
    table_index: c_int,
) -> c_int {
    if result.is_null() || table_index < 0 {
        return 0;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return 0;
    }
    result.inner.data_tables[idx].len() as c_int
}

/// Get signal names from a data table.
///
/// # Arguments
/// * `result` - Result handle
/// * `table_index` - Index of the data table
/// * `out_names` - Array of char* pointers to receive names
/// * `max_count` - Maximum number of names to retrieve
///
/// # Returns
/// Number of names copied
///
/// # Note
/// The returned strings are valid until the result is freed.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_signal_names(
    result: *const CHspiceResult,
    table_index: c_int,
    out_names: *mut *const c_char,
    max_count: c_int,
) -> c_int {
    if result.is_null() || out_names.is_null() || table_index < 0 || max_count <= 0 {
        return 0;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return 0;
    }

    let table = &result.inner.data_tables[idx];
    let count = std::cmp::min(table.len(), max_count as usize);
    for (i, name) in table.keys().take(count).enumerate() {
        *out_names.add(i) = name.as_ptr() as *const c_char;
    }
    count as c_int
}

/// Get signal data length.
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_signal_length(
    result: *const CHspiceResult,
    table_index: c_int,
    signal_name: *const c_char,
) -> c_int {
    if result.is_null() || signal_name.is_null() || table_index < 0 {
        return 0;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return 0;
    }

    let name = match CStr::from_ptr(signal_name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match result.inner.data_tables[idx].get(name) {
        Some(VectorData::Real(vec)) => vec.len() as c_int,
        Some(VectorData::Complex(vec)) => vec.len() as c_int,
        None => 0,
    }
}

/// Check if signal data is complex.
///
/// # Returns
/// 1 if complex, 0 if real, -1 on error
#[no_mangle]
pub unsafe extern "C" fn hspice_result_signal_is_complex(
    result: *const CHspiceResult,
    table_index: c_int,
    signal_name: *const c_char,
) -> c_int {
    if result.is_null() || signal_name.is_null() || table_index < 0 {
        return -1;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return -1;
    }

    let name = match CStr::from_ptr(signal_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match result.inner.data_tables[idx].get(name) {
        Some(VectorData::Real(_)) => 0,
        Some(VectorData::Complex(_)) => 1,
        None => -1,
    }
}

/// Get real signal data.
///
/// # Arguments
/// * `result` - Result handle
/// * `table_index` - Data table index
/// * `signal_name` - Signal name
/// * `out_values` - Output buffer for values
/// * `max_count` - Maximum number of values to copy
///
/// # Returns
/// Number of values copied, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_signal_real(
    result: *const CHspiceResult,
    table_index: c_int,
    signal_name: *const c_char,
    out_values: *mut c_double,
    max_count: c_int,
) -> c_int {
    if result.is_null() || signal_name.is_null() || out_values.is_null() {
        return -1;
    }
    if table_index < 0 || max_count <= 0 {
        return -1;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return -1;
    }

    let name = match CStr::from_ptr(signal_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match result.inner.data_tables[idx].get(name) {
        Some(VectorData::Real(vec)) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            for (i, &val) in vec.iter().take(count).enumerate() {
                *out_values.add(i) = val;
            }
            count as c_int
        }
        _ => -1,
    }
}

/// Get complex signal data (real and imaginary parts interleaved).
///
/// # Arguments
/// * `result` - Result handle
/// * `table_index` - Data table index
/// * `signal_name` - Signal name
/// * `out_real` - Output buffer for real parts
/// * `out_imag` - Output buffer for imaginary parts
/// * `max_count` - Maximum number of complex values to copy
///
/// # Returns
/// Number of complex values copied, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn hspice_result_get_signal_complex(
    result: *const CHspiceResult,
    table_index: c_int,
    signal_name: *const c_char,
    out_real: *mut c_double,
    out_imag: *mut c_double,
    max_count: c_int,
) -> c_int {
    if result.is_null() || signal_name.is_null() || out_real.is_null() || out_imag.is_null() {
        return -1;
    }
    if table_index < 0 || max_count <= 0 {
        return -1;
    }
    let result = &*result;
    let idx = table_index as usize;
    if idx >= result.inner.data_tables.len() {
        return -1;
    }

    let name = match CStr::from_ptr(signal_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match result.inner.data_tables[idx].get(name) {
        Some(VectorData::Complex(vec)) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            for (i, c) in vec.iter().take(count).enumerate() {
                *out_real.add(i) = c.re;
                *out_imag.add(i) = c.im;
            }
            count as c_int
        }
        _ => -1,
    }
}

// ============================================================================
// Streaming API for C
// ============================================================================

use crate::stream::{DataChunk, HspiceStreamReader};

/// Opaque handle to streaming reader
#[repr(C)]
pub struct CHspiceStream {
    reader: HspiceStreamReader,
    current_chunk: Option<DataChunk>,
    signal_names: Vec<CString>,
    scale_name: CString,
}

/// Open a file for streaming read.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_open(
    filename: *const c_char,
    chunk_size: c_int,
    debug: c_int,
) -> *mut CHspiceStream {
    if filename.is_null() || chunk_size <= 0 {
        return ptr::null_mut();
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    if debug > 0 {
        eprintln!(
            "hspice_stream_open: {} (chunk_size={})",
            filename_str, chunk_size
        );
    }

    let reader = match crate::stream::read_stream_chunked(filename_str, chunk_size as usize) {
        Ok(r) => r,
        Err(e) => {
            if debug > 0 {
                eprintln!("hspice_stream_open error: {:?}", e);
            }
            return ptr::null_mut();
        }
    };

    let metadata = reader.metadata();
    let signal_names: Vec<CString> = metadata
        .signal_names
        .iter()
        .filter_map(|s| CString::new(s.clone()).ok())
        .collect();
    let scale_name = CString::new(metadata.scale_name.clone()).unwrap_or_default();

    Box::into_raw(Box::new(CHspiceStream {
        reader,
        current_chunk: None,
        signal_names,
        scale_name,
    }))
}

/// Close a streaming reader.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_close(stream: *mut CHspiceStream) {
    if !stream.is_null() {
        drop(Box::from_raw(stream));
    }
}

/// Get the scale name.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_scale_name(
    stream: *const CHspiceStream,
) -> *const c_char {
    if stream.is_null() {
        return ptr::null();
    }
    (*stream).scale_name.as_ptr()
}

/// Get the number of signals.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_signal_count(stream: *const CHspiceStream) -> c_int {
    if stream.is_null() {
        return 0;
    }
    (*stream).signal_names.len() as c_int
}

/// Get a signal name by index.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_signal_name(
    stream: *const CHspiceStream,
    index: c_int,
) -> *const c_char {
    if stream.is_null() || index < 0 {
        return ptr::null();
    }
    let stream_ref = &*stream;
    let idx = index as usize;
    if idx >= stream_ref.signal_names.len() {
        return ptr::null();
    }
    stream_ref.signal_names[idx].as_ptr()
}

/// Read the next chunk. Returns 1 if success, 0 if EOF, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_next(stream: *mut CHspiceStream) -> c_int {
    if stream.is_null() {
        return -1;
    }
    let stream = &mut *stream;

    match stream.reader.next() {
        Some(Ok(chunk)) => {
            stream.current_chunk = Some(chunk);
            1
        }
        Some(Err(_)) => -1,
        None => 0,
    }
}

/// Get the current chunk's point count.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_chunk_size(stream: *const CHspiceStream) -> c_int {
    if stream.is_null() {
        return 0;
    }
    match &(*stream).current_chunk {
        Some(chunk) => chunk
            .data
            .values()
            .next()
            .map(|v| match v {
                VectorData::Real(d) => d.len() as c_int,
                VectorData::Complex(d) => d.len() as c_int,
            })
            .unwrap_or(0),
        None => 0,
    }
}

/// Get the current chunk's time range start.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_time_start(stream: *const CHspiceStream) -> c_double {
    if stream.is_null() {
        return 0.0;
    }
    match &(*stream).current_chunk {
        Some(chunk) => chunk.time_range.0,
        None => 0.0,
    }
}

/// Get the current chunk's time range end.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_time_end(stream: *const CHspiceStream) -> c_double {
    if stream.is_null() {
        return 0.0;
    }
    match &(*stream).current_chunk {
        Some(chunk) => chunk.time_range.1,
        None => 0.0,
    }
}

/// Copy signal data from the current chunk into buffer.
#[no_mangle]
pub unsafe extern "C" fn hspice_stream_get_signal_data(
    stream: *const CHspiceStream,
    signal_name: *const c_char,
    out_buffer: *mut c_double,
    max_count: c_int,
) -> c_int {
    if stream.is_null() || signal_name.is_null() || out_buffer.is_null() || max_count <= 0 {
        return -1;
    }

    let name = match CStr::from_ptr(signal_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let chunk = match &(*stream).current_chunk {
        Some(c) => c,
        None => return -1,
    };

    match chunk.data.get(name) {
        Some(VectorData::Real(vec)) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            std::ptr::copy_nonoverlapping(vec.as_ptr(), out_buffer, count);
            count as c_int
        }
        Some(VectorData::Complex(vec)) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            for (i, c) in vec.iter().take(count).enumerate() {
                *out_buffer.add(i) = (c.re * c.re + c.im * c.im).sqrt();
            }
            count as c_int
        }
        None => -1,
    }
}
