//! WebAssembly bindings for waveform file parser
//!
//! Provides JavaScript-friendly API for parsing HSPICE binary files in the browser.

use hspice_core::{AnalysisType, VarType, VectorData, WaveformResult};
use js_sys::{Array, Float64Array, Object, Reflect};
use std::io::Write;
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

    vector_to_js(&table.vectors[idx])
}

// ============================================================================
// Internal Helpers
// ============================================================================

fn parse_from_bytes(data: &[u8]) -> Result<WaveformResult, JsValue> {
    // Create temp file for parsing (WASM can't access filesystem)
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("hspice_wasm_temp.tr0");

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| JsValue::from_str(&format!("Failed to create temp file: {}", e)))?;

    file.write_all(data)
        .map_err(|e| JsValue::from_str(&format!("Failed to write data: {}", e)))?;

    drop(file);

    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| JsValue::from_str("Invalid temp path"))?;

    let result = hspice_core::read(temp_path_str)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {:?}", e)))?;

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);

    Ok(result)
}

fn create_js_result(data: &WaveformResult) -> Result<JsValue, JsValue> {
    let result = Object::new();

    // Metadata
    Reflect::set(&result, &"title".into(), &data.title.clone().into())?;
    Reflect::set(&result, &"date".into(), &data.date.clone().into())?;
    Reflect::set(&result, &"scaleName".into(), &data.scale_name().into())?;

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
        Reflect::set(&var_obj, &"name".into(), &var.name.clone().into())?;
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
        Some(name) => Reflect::set(&result, &"sweepParam".into(), &name.clone().into())?,
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
            let js_array = vector_to_js(vector)?;
            Reflect::set(&signals, &var.name.clone().into(), &js_array)?;
        }
        Reflect::set(&table_obj, &"signals".into(), &signals)?;

        tables.push(&table_obj);
    }
    Reflect::set(&result, &"tables".into(), &tables)?;

    // Counts
    Reflect::set(&result, &"numPoints".into(), &(data.len() as u32).into())?;
    Reflect::set(&result, &"numVars".into(), &(data.num_vars() as u32).into())?;
    Reflect::set(
        &result,
        &"numSweeps".into(),
        &(data.num_sweeps() as u32).into(),
    )?;

    Ok(result.into())
}

fn vector_to_js(vector: &VectorData) -> Result<JsValue, JsValue> {
    match vector {
        VectorData::Real(vec) => {
            let array = Float64Array::new_with_length(vec.len() as u32);
            for (i, &v) in vec.iter().enumerate() {
                array.set_index(i as u32, v);
            }
            Ok(array.into())
        }
        VectorData::Complex(vec) => {
            // Return magnitude for complex data
            let array = Float64Array::new_with_length(vec.len() as u32);
            for (i, c) in vec.iter().enumerate() {
                array.set_index(i as u32, (c.re * c.re + c.im * c.im).sqrt());
            }
            Ok(array.into())
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests require wasm-pack test, not regular cargo test
}
