//! WebAssembly bindings for the waveform parser.

use hspice_core::{AnalysisType, VarType, VectorData, WaveformResult};
use js_sys::{Array, Float64Array, Object, Reflect};
use wasm_bindgen::prelude::*;

// ============================================================================
// JavaScript Result Types
// ============================================================================

/// Parse HSPICE binary data from a Uint8Array
///
/// # Arguments
/// * `data` - Binary file content as Uint8Array
///
/// # Returns
/// JavaScript object with parsed waveform data
#[wasm_bindgen(js_name = parseHspice)]
pub fn parse_hspice(data: &[u8]) -> Result<JsValue, JsValue> {
    let result = parse_from_bytes(data)?;
    create_js_result(&result)
}

/// Get all signal names from parsed result
#[wasm_bindgen(js_name = getSignalNames)]
pub fn get_signal_names(data: &[u8]) -> Result<Array, JsValue> {
    let result = parse_from_bytes(data)?;

    let names = Array::new();
    for var in &result.variables {
        names.push(&JsValue::from_str(&var.name));
    }
    Ok(names)
}

/// Get signal data by name
#[wasm_bindgen(js_name = getSignalData)]
pub fn get_signal_data(data: &[u8], signal_name: &str) -> Result<JsValue, JsValue> {
    let result = parse_from_bytes(data)?;

    let idx = result
        .var_index(signal_name)
        .ok_or_else(|| JsValue::from_str(&format!("Signal not found: {}", signal_name)))?;

    let table = result
        .tables
        .first()
        .ok_or_else(|| JsValue::from_str("No data tables"))?;

    let vector = table
        .vectors
        .get(idx)
        .ok_or_else(|| JsValue::from_str("Signal metadata and data are inconsistent"))?;
    Ok(vector_to_js(vector))
}

// ============================================================================
// SPICE3 Raw File Parser
// ============================================================================

/// Parse SPICE3/ngspice raw file data (auto-detects binary/ASCII format)
///
/// # Arguments
/// * `data` - Raw file content as Uint8Array
///
/// # Returns
/// JavaScript object with parsed waveform data
#[wasm_bindgen(js_name = parseRaw)]
pub fn parse_raw(data: &[u8]) -> Result<JsValue, JsValue> {
    let result = parse_raw_from_bytes(data)?;
    create_js_result(&result)
}

// ============================================================================
// Internal Helpers
// ============================================================================

fn parse_raw_from_bytes(data: &[u8]) -> Result<WaveformResult, JsValue> {
    hspice_core::read_raw_bytes(data)
        .map_err(|error| JsValue::from_str(&format!("Parse raw error: {error}")))
}

fn parse_from_bytes(data: &[u8]) -> Result<WaveformResult, JsValue> {
    hspice_core::read_bytes(data, "waveform.tr0")
        .map_err(|error| JsValue::from_str(&format!("Parse error: {error}")))
}

fn create_js_result(data: &WaveformResult) -> Result<JsValue, JsValue> {
    let result = Object::new();

    Reflect::set(&result, &"title".into(), &JsValue::from_str(&data.title))?;
    Reflect::set(&result, &"date".into(), &JsValue::from_str(&data.date))?;
    Reflect::set(
        &result,
        &"scaleName".into(),
        &JsValue::from_str(data.scale_name()),
    )?;

    // Analysis type
    let analysis = match data.analysis {
        AnalysisType::Transient => "transient",
        AnalysisType::AC => "ac",
        AnalysisType::DC => "dc",
        AnalysisType::Operating => "operating",
        AnalysisType::Noise => "noise",
        AnalysisType::Unknown => "unknown",
    };
    Reflect::set(&result, &"analysis".into(), &analysis.into())?;

    // Variables
    let variables = Array::new();
    for var in &data.variables {
        let var_obj = Object::new();
        Reflect::set(&var_obj, &"name".into(), &JsValue::from_str(&var.name))?;
        let var_type = match var.var_type {
            VarType::Time => "time",
            VarType::Frequency => "frequency",
            VarType::Voltage => "voltage",
            VarType::Current => "current",
            VarType::Unknown => "unknown",
        };
        Reflect::set(&var_obj, &"type".into(), &var_type.into())?;
        variables.push(&var_obj);
    }
    Reflect::set(&result, &"variables".into(), &variables)?;

    // Sweep info
    match &data.sweep_param {
        Some(name) => Reflect::set(&result, &"sweepParam".into(), &JsValue::from_str(name))?,
        None => Reflect::set(&result, &"sweepParam".into(), &JsValue::NULL)?,
    };

    // Tables
    let tables = Array::new();
    for table in &data.tables {
        let table_obj = Object::new();

        // Sweep value
        match table.sweep_value {
            Some(v) => Reflect::set(&table_obj, &"sweepValue".into(), &v.into())?,
            None => Reflect::set(&table_obj, &"sweepValue".into(), &JsValue::NULL)?,
        };

        // Data as object {name: Float64Array}
        let signals = Object::new();
        for (var, vector) in data.variables.iter().zip(table.vectors.iter()) {
            let js_array = vector_to_js(vector);
            Reflect::set(&signals, &JsValue::from_str(&var.name), &js_array)?;
        }
        Reflect::set(&table_obj, &"signals".into(), &signals)?;

        tables.push(&table_obj);
    }
    Reflect::set(&result, &"tables".into(), &tables)?;

    // Counts
    Reflect::set(
        &result,
        &"numPoints".into(),
        &JsValue::from_f64(data.len() as f64),
    )?;
    Reflect::set(
        &result,
        &"numVars".into(),
        &JsValue::from_f64(data.num_vars() as f64),
    )?;
    Reflect::set(
        &result,
        &"numSweeps".into(),
        &JsValue::from_f64(data.num_sweeps() as f64),
    )?;

    Ok(result.into())
}

fn vector_to_js(vector: &VectorData) -> JsValue {
    match vector {
        VectorData::Real(values) => Float64Array::from(values.as_slice()).into(),
        VectorData::Complex(values) => {
            let magnitudes: Vec<f64> = values.iter().map(|value| value.norm()).collect();
            Float64Array::from(magnitudes.as_slice()).into()
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests require wasm-pack test, not regular cargo test
}
