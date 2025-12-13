//! C Foreign Function Interface (FFI) for waveform parser
//!
//! This module provides a C-compatible API for using the waveform parser
//! from C, C++, and other languages that support C FFI.

use hspice_core::{
    read_debug, read_raw_debug, read_stream_chunked, DataChunk, HspiceStreamReader, VectorData,
    WaveformResult,
};
use std::ffi::{c_char, c_double, c_int, CStr, CString};
use std::ptr;

// ============================================================================
// Opaque Types for C
// ============================================================================

/// Opaque handle to WaveformResult
#[repr(C)]
pub struct CWaveformResult {
    inner: Box<WaveformResult>,
    cached_title: CString,
    cached_date: CString,
    cached_scale_name: CString,
    cached_sweep_param: Option<CString>,
    cached_var_names: Vec<CString>,
}

// ============================================================================
// Result Creation and Destruction
// ============================================================================

/// Read a waveform file and return a result handle.
#[no_mangle]
pub unsafe extern "C" fn waveform_read(
    filename: *const c_char,
    debug: c_int,
) -> *mut CWaveformResult {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let filename_cstr = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match read_debug(filename_cstr, debug) {
        Ok(result) => {
            let cached_title = CString::new(result.title.clone()).unwrap_or_default();
            let cached_date = CString::new(result.date.clone()).unwrap_or_default();
            let cached_scale_name =
                CString::new(result.scale_name().to_string()).unwrap_or_default();
            let cached_sweep_param = result
                .sweep_param
                .as_ref()
                .and_then(|s| CString::new(s.clone()).ok());
            let cached_var_names: Vec<CString> = result
                .variables
                .iter()
                .filter_map(|v| CString::new(v.name.clone()).ok())
                .collect();

            Box::into_raw(Box::new(CWaveformResult {
                inner: Box::new(result),
                cached_title,
                cached_date,
                cached_scale_name,
                cached_sweep_param,
                cached_var_names,
            }))
        }
        Err(e) => {
            if debug > 0 {
                eprintln!("waveform_read error: {:?}", e);
            }
            ptr::null_mut()
        }
    }
}

/// Free a waveform result handle.
#[no_mangle]
pub unsafe extern "C" fn waveform_free(result: *mut CWaveformResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

/// Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).
#[no_mangle]
pub unsafe extern "C" fn waveform_read_raw(
    filename: *const c_char,
    debug: c_int,
) -> *mut CWaveformResult {
    if filename.is_null() {
        return ptr::null_mut();
    }

    let filename_cstr = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match read_raw_debug(filename_cstr, debug) {
        Ok(result) => {
            let cached_title = CString::new(result.title.clone()).unwrap_or_default();
            let cached_date = CString::new(result.date.clone()).unwrap_or_default();
            let cached_scale_name =
                CString::new(result.scale_name().to_string()).unwrap_or_default();
            let cached_sweep_param = result
                .sweep_param
                .as_ref()
                .and_then(|s| CString::new(s.clone()).ok());
            let cached_var_names: Vec<CString> = result
                .variables
                .iter()
                .filter_map(|v| CString::new(v.name.clone()).ok())
                .collect();

            Box::into_raw(Box::new(CWaveformResult {
                inner: Box::new(result),
                cached_title,
                cached_date,
                cached_scale_name,
                cached_sweep_param,
                cached_var_names,
            }))
        }
        Err(e) => {
            if debug > 0 {
                eprintln!("waveform_read_raw error: {:?}", e);
            }
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Metadata Accessors
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn waveform_get_title(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    (*result).cached_title.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_date(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    (*result).cached_date.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_scale_name(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    (*result).cached_scale_name.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_analysis_type(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return -1;
    }
    match (*result).inner.analysis {
        hspice_core::AnalysisType::Transient => 0,
        hspice_core::AnalysisType::AC => 1,
        hspice_core::AnalysisType::DC => 2,
        hspice_core::AnalysisType::Operating => 3,
        hspice_core::AnalysisType::Noise => 4,
        hspice_core::AnalysisType::Unknown => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_table_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    (*result).inner.tables.len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    (*result).inner.variables.len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_point_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    (*result).inner.len() as c_int
}

// ============================================================================
// Variable Accessors
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_name(
    result: *const CWaveformResult,
    index: c_int,
) -> *const c_char {
    if result.is_null() || index < 0 {
        return ptr::null();
    }
    let r = &*result;
    let idx = index as usize;
    if idx >= r.cached_var_names.len() {
        return ptr::null();
    }
    r.cached_var_names[idx].as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_type(
    result: *const CWaveformResult,
    index: c_int,
) -> c_int {
    if result.is_null() || index < 0 {
        return -1;
    }
    let r = &(*result).inner;
    let idx = index as usize;
    if idx >= r.variables.len() {
        return -1;
    }
    match r.variables[idx].var_type {
        hspice_core::VarType::Time => 0,
        hspice_core::VarType::Frequency => 1,
        hspice_core::VarType::Voltage => 2,
        hspice_core::VarType::Current => 3,
        hspice_core::VarType::Unknown => -1,
    }
}

// ============================================================================
// Sweep Accessors
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn waveform_has_sweep(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    if (*result).inner.has_sweep() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_sweep_param(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    match &(*result).cached_sweep_param {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_get_sweep_value(
    result: *const CWaveformResult,
    table_index: c_int,
) -> c_double {
    if result.is_null() || table_index < 0 {
        return 0.0;
    }
    let r = &(*result).inner;
    let idx = table_index as usize;
    if idx >= r.tables.len() {
        return 0.0;
    }
    r.tables[idx].sweep_value.unwrap_or(0.0)
}

// ============================================================================
// Data Accessors
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn waveform_get_data_length(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
) -> c_int {
    if result.is_null() || table_index < 0 || var_index < 0 {
        return 0;
    }
    let ti = table_index as usize;
    let vi = var_index as usize;
    let r = &(*result).inner;
    if ti >= r.tables.len() || vi >= r.variables.len() {
        return 0;
    }
    r.tables[ti].vectors[vi].len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn waveform_is_complex(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
) -> c_int {
    if result.is_null() || table_index < 0 || var_index < 0 {
        return -1;
    }
    let ti = table_index as usize;
    let vi = var_index as usize;
    let r = &(*result).inner;
    if ti >= r.tables.len() || vi >= r.variables.len() {
        return -1;
    }
    if r.tables[ti].vectors[vi].is_complex() {
        1
    } else {
        0
    }
}

/// Get real data by variable index.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_real_data(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
    out_buffer: *mut c_double,
    max_count: c_int,
) -> c_int {
    if result.is_null()
        || out_buffer.is_null()
        || table_index < 0
        || var_index < 0
        || max_count <= 0
    {
        return -1;
    }
    let ti = table_index as usize;
    let vi = var_index as usize;
    let r = &(*result).inner;
    if ti >= r.tables.len() || vi >= r.variables.len() {
        return -1;
    }

    match &r.tables[ti].vectors[vi] {
        VectorData::Real(vec) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            std::ptr::copy_nonoverlapping(vec.as_ptr(), out_buffer, count);
            count as c_int
        }
        VectorData::Complex(_) => -1,
    }
}

/// Get complex data by variable index.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_complex_data(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
    out_real: *mut c_double,
    out_imag: *mut c_double,
    max_count: c_int,
) -> c_int {
    if result.is_null() || out_real.is_null() || out_imag.is_null() {
        return -1;
    }
    if table_index < 0 || var_index < 0 || max_count <= 0 {
        return -1;
    }

    let ti = table_index as usize;
    let vi = var_index as usize;
    let r = &(*result).inner;
    if ti >= r.tables.len() || vi >= r.variables.len() {
        return -1;
    }

    match &r.tables[ti].vectors[vi] {
        VectorData::Complex(vec) => {
            let count = std::cmp::min(vec.len(), max_count as usize);
            for (i, c) in vec.iter().take(count).enumerate() {
                *out_real.add(i) = c.re;
                *out_imag.add(i) = c.im;
            }
            count as c_int
        }
        VectorData::Real(_) => -1,
    }
}

// ============================================================================
// Streaming API
// ============================================================================

#[repr(C)]
pub struct CWaveformStream {
    reader: HspiceStreamReader,
    current_chunk: Option<DataChunk>,
    signal_names: Vec<CString>,
    scale_name: CString,
}

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_open(
    filename: *const c_char,
    chunk_size: c_int,
    debug: c_int,
) -> *mut CWaveformStream {
    if filename.is_null() || chunk_size <= 0 {
        return ptr::null_mut();
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    if debug > 0 {
        eprintln!(
            "waveform_stream_open: {} (chunk_size={})",
            filename_str, chunk_size
        );
    }

    let reader = match read_stream_chunked(filename_str, chunk_size as usize) {
        Ok(r) => r,
        Err(e) => {
            if debug > 0 {
                eprintln!("stream open error: {:?}", e);
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

    Box::into_raw(Box::new(CWaveformStream {
        reader,
        current_chunk: None,
        signal_names,
        scale_name,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_close(stream: *mut CWaveformStream) {
    if !stream.is_null() {
        drop(Box::from_raw(stream));
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_next(stream: *mut CWaveformStream) -> c_int {
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

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_get_chunk_size(stream: *const CWaveformStream) -> c_int {
    if stream.is_null() {
        return 0;
    }
    match &(*stream).current_chunk {
        Some(chunk) => chunk
            .data
            .values()
            .next()
            .map(|v| v.len() as c_int)
            .unwrap_or(0),
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_get_time_range(
    stream: *const CWaveformStream,
    out_start: *mut c_double,
    out_end: *mut c_double,
) -> c_int {
    if stream.is_null() || out_start.is_null() || out_end.is_null() {
        return -1;
    }
    match &(*stream).current_chunk {
        Some(chunk) => {
            *out_start = chunk.time_range.0;
            *out_end = chunk.time_range.1;
            0
        }
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn waveform_stream_get_signal_data(
    stream: *const CWaveformStream,
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

// ============================================================================
// Legacy API aliases
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn hspice_read(
    filename: *const c_char,
    debug: c_int,
) -> *mut CWaveformResult {
    waveform_read(filename, debug)
}

#[no_mangle]
pub unsafe extern "C" fn hspice_result_free(result: *mut CWaveformResult) {
    waveform_free(result)
}
