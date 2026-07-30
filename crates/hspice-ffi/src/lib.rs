//! C Foreign Function Interface (FFI) for waveform parser
//!
//! This module provides a C-compatible API for using the waveform parser
//! from C, C++, and other languages that support C FFI.

#![deny(unsafe_op_in_unsafe_fn)]

use hspice_core::{
    read, read_raw, read_stream_chunked, DataChunk, HspiceStreamReader, VectorData, WaveformResult,
};
use std::ffi::{c_char, c_double, c_int, CStr, CString};
use std::ptr;
use std::sync::Once;

// ============================================================================
// Logging Initialization
// ============================================================================

static LOGGING_INIT: Once = Once::new();

/// Initialize logging with specified level.
///
/// Call this once before any other waveform functions to enable logging.
///
/// # Arguments
/// * `level` - Log level string: "trace", "debug", "info", "warn", "error"
///
/// # Returns
/// * 0 on success
/// * -1 if level string is null or invalid
///
/// # Example (C)
/// ```c
/// waveform_init_logging("info");
/// void* result = waveform_read("simulation.tr0", 0);
/// ```
///
/// # Safety
///
/// `level` must be null or point to a valid NUL-terminated string for the
/// duration of this call.
#[no_mangle]
pub unsafe extern "C" fn waveform_init_logging(level: *const c_char) -> c_int {
    if level.is_null() {
        return -1;
    }

    // SAFETY: the caller guarantees `level` points to a valid NUL-terminated string.
    let level_str = match unsafe { CStr::from_ptr(level) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    LOGGING_INIT.call_once(|| {
        use tracing_subscriber::EnvFilter;
        let filter = EnvFilter::try_new(level_str).unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    });

    0
}

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

impl CWaveformResult {
    fn into_raw(result: WaveformResult) -> *mut Self {
        let cached_title = c_string(&result.title);
        let cached_date = c_string(&result.date);
        let cached_scale_name = c_string(result.scale_name());
        let cached_sweep_param = result.sweep_param.as_deref().map(c_string);
        let cached_var_names = result
            .variables
            .iter()
            .map(|variable| c_string(&variable.name))
            .collect();

        Box::into_raw(Box::new(Self {
            inner: Box::new(result),
            cached_title,
            cached_date,
            cached_scale_name,
            cached_sweep_param,
            cached_var_names,
        }))
    }
}

fn c_string(value: &str) -> CString {
    let bytes = value
        .as_bytes()
        .iter()
        .copied()
        .filter(|byte| *byte != 0)
        .collect();
    // SAFETY: the iterator above removes every interior NUL byte.
    unsafe { CString::from_vec_unchecked(bytes) }
}

fn c_count(value: usize) -> c_int {
    match c_int::try_from(value) {
        Ok(count) => count,
        Err(_) => c_int::MAX,
    }
}

fn c_index(value: c_int) -> Option<usize> {
    usize::try_from(value).ok()
}

fn get_vector(
    result: &WaveformResult,
    table_index: c_int,
    variable_index: c_int,
) -> Option<&VectorData> {
    let table = result.tables.get(c_index(table_index)?)?;
    table.vectors.get(c_index(variable_index)?)
}

// ============================================================================
// Result Creation and Destruction
// ============================================================================

/// Read a waveform file and return a result handle.
///
/// The debug parameter is deprecated and ignored. Use waveform_init_logging() instead.
///
/// # Safety
///
/// `filename` must be null or point to a valid NUL-terminated string for the
/// duration of this call.
#[no_mangle]
pub unsafe extern "C" fn waveform_read(
    filename: *const c_char,
    _debug: c_int,
) -> *mut CWaveformResult {
    if filename.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: the caller guarantees `filename` is a valid C string.
    let filename_cstr = match unsafe { CStr::from_ptr(filename) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match read(filename_cstr) {
        Ok(result) => CWaveformResult::into_raw(result),
        Err(e) => {
            tracing::error!("waveform_read error: {:?}", e);
            ptr::null_mut()
        }
    }
}

/// Free a waveform result handle.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library. A live
/// handle may be freed exactly once and must not be used afterward.
#[no_mangle]
pub unsafe extern "C" fn waveform_free(result: *mut CWaveformResult) {
    if !result.is_null() {
        // SAFETY: the pointer was returned by `Box::into_raw` in this library
        // and the API contract requires it to be freed exactly once.
        drop(unsafe { Box::from_raw(result) });
    }
}

/// Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).
///
/// The debug parameter is deprecated and ignored. Use waveform_init_logging() instead.
///
/// # Safety
///
/// `filename` must be null or point to a valid NUL-terminated string for the
/// duration of this call.
#[no_mangle]
pub unsafe extern "C" fn waveform_read_raw(
    filename: *const c_char,
    _debug: c_int,
) -> *mut CWaveformResult {
    if filename.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: the caller guarantees `filename` is a valid C string.
    let filename_cstr = match unsafe { CStr::from_ptr(filename) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match read_raw(filename_cstr) {
        Ok(result) => CWaveformResult::into_raw(result),
        Err(e) => {
            tracing::error!("waveform_read_raw error: {:?}", e);
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Metadata Accessors
// ============================================================================

/// Returns the cached title owned by `result`.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_title(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    unsafe { (*result).cached_title.as_ptr() }
}

/// Returns the cached date owned by `result`.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_date(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    unsafe { (*result).cached_date.as_ptr() }
}

/// Returns the cached scale name owned by `result`.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_scale_name(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    unsafe { (*result).cached_scale_name.as_ptr() }
}

/// Returns the numeric analysis type.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_analysis_type(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return -1;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    match unsafe { (*result).inner.analysis } {
        hspice_core::AnalysisType::Transient => 0,
        hspice_core::AnalysisType::AC => 1,
        hspice_core::AnalysisType::DC => 2,
        hspice_core::AnalysisType::Operating => 3,
        hspice_core::AnalysisType::Noise => 4,
        hspice_core::AnalysisType::Unknown => -1,
    }
}

/// Returns the number of tables.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_table_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    c_count(unsafe { (*result).inner.tables.len() })
}

/// Returns the number of variables.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    c_count(unsafe { (*result).inner.variables.len() })
}

/// Returns the number of points in the first table.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_point_count(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    c_count(unsafe { (*result).inner.len() })
}

// ============================================================================
// Variable Accessors
// ============================================================================

/// Returns a cached variable name.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_name(
    result: *const CWaveformResult,
    index: c_int,
) -> *const c_char {
    if result.is_null() || index < 0 {
        return ptr::null();
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &*result };
    let Some(idx) = c_index(index) else {
        return ptr::null();
    };
    if idx >= r.cached_var_names.len() {
        return ptr::null();
    }
    r.cached_var_names[idx].as_ptr()
}

/// Returns the numeric type for a variable.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_var_type(
    result: *const CWaveformResult,
    index: c_int,
) -> c_int {
    if result.is_null() || index < 0 {
        return -1;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };
    let Some(variable) = c_index(index).and_then(|idx| r.variables.get(idx)) else {
        return -1;
    };
    match variable.var_type {
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

/// Reports whether the result contains multiple sweep tables.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_has_sweep(result: *const CWaveformResult) -> c_int {
    if result.is_null() {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    if unsafe { (*result).inner.has_sweep() } {
        1
    } else {
        0
    }
}

/// Returns the cached sweep parameter name.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_sweep_param(result: *const CWaveformResult) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    match unsafe { &(*result).cached_sweep_param } {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    }
}

/// Returns one sweep value.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_sweep_value(
    result: *const CWaveformResult,
    table_index: c_int,
) -> c_double {
    if result.is_null() || table_index < 0 {
        return 0.0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };
    c_index(table_index)
        .and_then(|index| r.tables.get(index))
        .and_then(|table| table.sweep_value)
        .unwrap_or(0.0)
}

// ============================================================================
// Data Accessors
// ============================================================================

/// Returns the length of one data vector.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_get_data_length(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
) -> c_int {
    if result.is_null() || table_index < 0 || var_index < 0 {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };
    get_vector(r, table_index, var_index).map_or(0, |vector| c_count(vector.len()))
}

/// Reports whether one data vector is complex.
///
/// # Safety
///
/// `result` must be null or a live handle returned by this library.
#[no_mangle]
pub unsafe extern "C" fn waveform_is_complex(
    result: *const CWaveformResult,
    table_index: c_int,
    var_index: c_int,
) -> c_int {
    if result.is_null() || table_index < 0 || var_index < 0 {
        return -1;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };
    let Some(vector) = get_vector(r, table_index, var_index) else {
        return -1;
    };
    if vector.is_complex() {
        1
    } else {
        0
    }
}

/// Get real data by variable index.
///
/// # Safety
///
/// `result` must be a live handle and `out_buffer` must be writable for at
/// least `max_count` doubles. The buffers must not alias live Rust storage.
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
    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };

    match get_vector(r, table_index, var_index) {
        Some(VectorData::Real(vec)) => {
            let count = vec.len().min(c_index(max_count).unwrap_or(0));
            // SAFETY: the caller guarantees `out_buffer` is writable for
            // `max_count` doubles; `count` does not exceed either buffer.
            unsafe { std::ptr::copy_nonoverlapping(vec.as_ptr(), out_buffer, count) };
            c_count(count)
        }
        Some(VectorData::Complex(_)) | None => -1,
    }
}

/// Get complex data by variable index.
///
/// # Safety
///
/// `result` must be a live handle. `out_real` and `out_imag` must each be
/// writable for at least `max_count` doubles and must not overlap.
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

    // SAFETY: non-null handle validity is guaranteed by the caller.
    let r = unsafe { &(*result).inner };

    match get_vector(r, table_index, var_index) {
        Some(VectorData::Complex(vec)) => {
            let count = vec.len().min(c_index(max_count).unwrap_or(0));
            for (index, value) in vec.iter().take(count).enumerate() {
                // SAFETY: the caller guarantees both output buffers are writable
                // for `max_count` doubles, and `index < count <= max_count`.
                unsafe {
                    *out_real.add(index) = value.re;
                    *out_imag.add(index) = value.im;
                }
            }
            c_count(count)
        }
        Some(VectorData::Real(_)) | None => -1,
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

/// Open a file for streaming.
///
/// The debug parameter is deprecated and ignored. Use waveform_init_logging() instead.
///
/// # Safety
///
/// `filename` must be null or point to a valid NUL-terminated string for the
/// duration of this call.
#[no_mangle]
pub unsafe extern "C" fn waveform_stream_open(
    filename: *const c_char,
    chunk_size: c_int,
    _debug: c_int,
) -> *mut CWaveformStream {
    if filename.is_null() || chunk_size <= 0 {
        return ptr::null_mut();
    }

    // SAFETY: the caller guarantees `filename` is a valid C string.
    let filename_str = match unsafe { CStr::from_ptr(filename) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    tracing::debug!(
        "waveform_stream_open: {} (chunk_size={})",
        filename_str,
        chunk_size
    );

    let Some(chunk_size) = c_index(chunk_size) else {
        return ptr::null_mut();
    };
    let reader = match read_stream_chunked(filename_str, chunk_size) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("stream open error: {:?}", e);
            return ptr::null_mut();
        }
    };

    let metadata = reader.metadata();
    let signal_names = metadata
        .signal_names
        .iter()
        .map(|name| c_string(name))
        .collect();
    let scale_name = c_string(&metadata.scale_name);

    Box::into_raw(Box::new(CWaveformStream {
        reader,
        current_chunk: None,
        signal_names,
        scale_name,
    }))
}

/// Closes a streaming handle.
///
/// # Safety
///
/// `stream` must be null or a live handle returned by this library. A live
/// handle may be closed exactly once.
#[no_mangle]
pub unsafe extern "C" fn waveform_stream_close(stream: *mut CWaveformStream) {
    if !stream.is_null() {
        // SAFETY: the pointer was allocated by this library and the API
        // contract requires it to be closed exactly once.
        drop(unsafe { Box::from_raw(stream) });
    }
}

/// Advances a streaming handle to its next chunk.
///
/// # Safety
///
/// `stream` must be null or a live, exclusively accessed streaming handle.
#[no_mangle]
pub unsafe extern "C" fn waveform_stream_next(stream: *mut CWaveformStream) -> c_int {
    if stream.is_null() {
        return -1;
    }
    // SAFETY: non-null exclusive handle validity is guaranteed by the caller.
    let stream = unsafe { &mut *stream };

    match stream.reader.next() {
        Some(Ok(chunk)) => {
            stream.current_chunk = Some(chunk);
            1
        }
        Some(Err(_)) => -1,
        None => 0,
    }
}

/// Returns the current chunk size.
///
/// # Safety
///
/// `stream` must be null or a live streaming handle.
#[no_mangle]
pub unsafe extern "C" fn waveform_stream_get_chunk_size(stream: *const CWaveformStream) -> c_int {
    if stream.is_null() {
        return 0;
    }
    // SAFETY: non-null handle validity is guaranteed by the caller.
    match unsafe { &(*stream).current_chunk } {
        Some(chunk) => chunk
            .data
            .values()
            .next()
            .map_or(0, |vector| c_count(vector.len())),
        None => 0,
    }
}

/// Writes the current chunk's scale range.
///
/// # Safety
///
/// `stream` must be a live handle. `out_start` and `out_end` must point to
/// writable doubles and must not overlap.
#[no_mangle]
pub unsafe extern "C" fn waveform_stream_get_time_range(
    stream: *const CWaveformStream,
    out_start: *mut c_double,
    out_end: *mut c_double,
) -> c_int {
    if stream.is_null() || out_start.is_null() || out_end.is_null() {
        return -1;
    }
    // SAFETY: non-null handle and output-pointer validity are guaranteed by the caller.
    match unsafe { &(*stream).current_chunk } {
        Some(chunk) => {
            // SAFETY: both pointers were checked for null and the caller
            // guarantees they are writable doubles.
            unsafe {
                *out_start = chunk.time_range.0;
                *out_end = chunk.time_range.1;
            }
            0
        }
        None => -1,
    }
}

/// Copies a signal from the current streaming chunk.
///
/// # Safety
///
/// `stream` must be a live handle, `signal_name` must be a valid C string, and
/// `out_buffer` must be writable for at least `max_count` doubles.
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

    // SAFETY: the caller guarantees `signal_name` is a valid C string.
    let name = match unsafe { CStr::from_ptr(signal_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // SAFETY: non-null handle validity is guaranteed by the caller.
    let chunk = match unsafe { &(*stream).current_chunk } {
        Some(c) => c,
        None => return -1,
    };

    match chunk.data.get(name) {
        Some(VectorData::Real(vec)) => {
            let count = vec.len().min(c_index(max_count).unwrap_or(0));
            // SAFETY: the caller guarantees `out_buffer` is writable for
            // `max_count` doubles; `count` does not exceed either buffer.
            unsafe { std::ptr::copy_nonoverlapping(vec.as_ptr(), out_buffer, count) };
            c_count(count)
        }
        Some(VectorData::Complex(vec)) => {
            let count = vec.len().min(c_index(max_count).unwrap_or(0));
            for (index, value) in vec.iter().take(count).enumerate() {
                // SAFETY: `index < count <= max_count`, whose writable capacity
                // is guaranteed by the caller.
                unsafe { *out_buffer.add(index) = value.norm() };
            }
            c_count(count)
        }
        None => -1,
    }
}

// ============================================================================
// Legacy API aliases
// ============================================================================

/// Legacy alias for [`waveform_read`].
///
/// # Safety
///
/// Has the same requirements as [`waveform_read`].
#[no_mangle]
pub unsafe extern "C" fn hspice_read(
    filename: *const c_char,
    debug: c_int,
) -> *mut CWaveformResult {
    // SAFETY: this legacy alias has the same caller contract as `waveform_read`.
    unsafe { waveform_read(filename, debug) }
}

/// Legacy alias for [`waveform_free`].
///
/// # Safety
///
/// Has the same requirements as [`waveform_free`].
#[no_mangle]
pub unsafe extern "C" fn hspice_result_free(result: *mut CWaveformResult) {
    // SAFETY: this legacy alias has the same caller contract as `waveform_free`.
    unsafe { waveform_free(result) }
}

/// Legacy alias for waveform_init_logging
///
/// # Safety
///
/// Has the same requirements as [`waveform_init_logging`].
#[no_mangle]
pub unsafe extern "C" fn hspice_init_logging(level: *const c_char) -> c_int {
    // SAFETY: this legacy alias has the same caller contract as
    // `waveform_init_logging`.
    unsafe { waveform_init_logging(level) }
}
