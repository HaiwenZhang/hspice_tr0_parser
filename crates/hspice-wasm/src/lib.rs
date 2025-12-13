//! WebAssembly bindings for HSPICE binary file parser
//!
//! This crate provides WASM bindings for parsing HSPICE binary files
//! in browser and Node.js environments.
//!
//! # Usage
//!
//! ```javascript
//! import init, { parseHspice } from '@haiwen/hspice-parser';
//!
//! await init();
//! const bytes = new Uint8Array(await file.arrayBuffer());
//! const result = parseHspice(bytes);
//! console.log(result.title);
//! ```

use hspice_core::{self, VectorData};
use js_sys::{Array, Float64Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Parse an HSPICE binary file from a Uint8Array
///
/// # Arguments
/// * `data` - Binary content of the HSPICE file
///
/// # Returns
/// A JavaScript object containing the parsed result
#[wasm_bindgen(js_name = parseHspice)]
pub fn parse_hspice(data: &[u8]) -> Result<JsValue, JsValue> {
    let result = parse_from_bytes(data)?;
    create_js_result(&result)
}

/// Internal struct to hold parsed data
struct HspiceData {
    title: String,
    date: String,
    scale_name: String,
    sweep_name: Option<String>,
    sweep_values: Option<Vec<f64>>,
    tables: Vec<std::collections::HashMap<String, VectorData>>,
}

fn parse_from_bytes(data: &[u8]) -> Result<HspiceData, JsValue> {
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("hspice_wasm_{}.tr0", js_sys::Math::random()));
    let temp_path_str = temp_path.to_string_lossy().to_string();

    // Write data to temp file
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| JsValue::from_str(&format!("Failed to create temp file: {}", e)))?;
    file.write_all(data)
        .map_err(|e| JsValue::from_str(&format!("Failed to write temp file: {}", e)))?;
    drop(file);

    // Parse the file
    let result = hspice_core::read(&temp_path_str)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {:?}", e)))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(HspiceData {
        title: result.title,
        date: result.date,
        scale_name: result.scale_name,
        sweep_name: result.sweep_name,
        sweep_values: result.sweep_values,
        tables: result.data_tables,
    })
}

fn create_js_result(data: &HspiceData) -> Result<JsValue, JsValue> {
    let result = Object::new();

    // Set basic properties
    Reflect::set(&result, &"title".into(), &data.title.clone().into())?;
    Reflect::set(&result, &"date".into(), &data.date.clone().into())?;
    Reflect::set(
        &result,
        &"scaleName".into(),
        &data.scale_name.clone().into(),
    )?;
    Reflect::set(
        &result,
        &"tableCount".into(),
        &(data.tables.len() as u32).into(),
    )?;

    // Sweep info
    match &data.sweep_name {
        Some(name) => Reflect::set(&result, &"sweepName".into(), &name.clone().into())?,
        None => Reflect::set(&result, &"sweepName".into(), &JsValue::NULL)?,
    };

    match &data.sweep_values {
        Some(values) => {
            let arr = Float64Array::new_with_length(values.len() as u32);
            for (i, &v) in values.iter().enumerate() {
                arr.set_index(i as u32, v);
            }
            Reflect::set(&result, &"sweepValues".into(), &arr.into())?;
        }
        None => {
            Reflect::set(&result, &"sweepValues".into(), &JsValue::NULL)?;
        }
    };

    // Create tables array
    let tables = Array::new();
    for table in &data.tables {
        let table_obj = Object::new();

        // Get signal names
        let names = Array::new();
        for name in table.keys() {
            names.push(&name.clone().into());
        }
        Reflect::set(&table_obj, &"signalNames".into(), &names)?;

        // Create data object
        let data_obj = Object::new();
        for (name, vector) in table {
            match vector {
                VectorData::Real(values) => {
                    let arr = Float64Array::new_with_length(values.len() as u32);
                    for (i, &v) in values.iter().enumerate() {
                        arr.set_index(i as u32, v);
                    }
                    Reflect::set(&data_obj, &name.clone().into(), &arr.into())?;
                }
                VectorData::Complex(values) => {
                    // For complex, return magnitude
                    let arr = Float64Array::new_with_length(values.len() as u32);
                    for (i, c) in values.iter().enumerate() {
                        let mag = (c.re * c.re + c.im * c.im).sqrt();
                        arr.set_index(i as u32, mag);
                    }
                    Reflect::set(&data_obj, &name.clone().into(), &arr.into())?;
                }
            }
        }
        Reflect::set(&table_obj, &"data".into(), &data_obj)?;

        tables.push(&table_obj);
    }
    Reflect::set(&result, &"tables".into(), &tables)?;

    Ok(result.into())
}

/// Get signal data from a parsed result
#[wasm_bindgen(js_name = getSignalData)]
pub fn get_signal_data(
    result: &JsValue,
    table_index: u32,
    signal_name: &str,
) -> Result<Float64Array, JsValue> {
    let tables = Reflect::get(result, &"tables".into())?;
    let table = Reflect::get(&tables, &table_index.into())?;
    let data = Reflect::get(&table, &"data".into())?;
    let signal = Reflect::get(&data, &signal_name.into())?;

    if signal.is_undefined() {
        return Err(JsValue::from_str(&format!(
            "Signal '{}' not found",
            signal_name
        )));
    }

    Ok(Float64Array::from(signal))
}

/// Get all signal names from a table
#[wasm_bindgen(js_name = getSignalNames)]
pub fn get_signal_names(result: &JsValue, table_index: u32) -> Result<Array, JsValue> {
    let tables = Reflect::get(result, &"tables".into())?;
    let table = Reflect::get(&tables, &table_index.into())?;
    let names = Reflect::get(&table, &"signalNames".into())?;

    Ok(Array::from(&names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);
}
