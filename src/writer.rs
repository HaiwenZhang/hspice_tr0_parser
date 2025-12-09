//! SPICE3 Binary Raw File Writer

use crate::types::{HspiceError, HspiceResult, Result, VectorData};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Write SPICE3 binary raw file header
fn write_raw_header<W: Write>(
    writer: &mut W,
    title: &str,
    date: &str,
    plot_name: &str,
    scale_name: &str,
    variable_names: &[String],
    num_points: usize,
    is_complex: bool,
) -> Result<()> {
    // Write text header
    writeln!(writer, "Title: {}", title)?;
    writeln!(writer, "Date: {}", date)?;
    writeln!(writer, "Plotname: {}", plot_name)?;
    writeln!(
        writer,
        "Flags: {}",
        if is_complex { "complex" } else { "real" }
    )?;
    writeln!(writer, "No. Variables: {}", variable_names.len() + 1)?; // +1 for scale
    writeln!(writer, "No. Points: {}", num_points)?;
    writeln!(writer, "Variables:")?;

    // Write scale variable (index 0)
    writeln!(writer, "\t0\t{}\ttime", scale_name)?;

    // Write other variables
    for (i, name) in variable_names.iter().enumerate() {
        // Determine variable type from name
        let var_type = if name.starts_with("i(") || name.starts_with("i_") {
            "current"
        } else {
            "voltage"
        };
        writeln!(writer, "\t{}\t{}\t{}", i + 1, name, var_type)?;
    }

    writeln!(writer, "Binary:")?;

    Ok(())
}

/// Write SPICE3 binary data section
/// Format: For each time point, write time (f64) followed by all signal values (f64)
fn write_raw_data<W: Write>(
    writer: &mut W,
    scale_data: &[f64],
    signal_data: &[Vec<f64>],
) -> Result<()> {
    let num_points = scale_data.len();

    for i in 0..num_points {
        // Write time value as f64
        writer.write_all(&scale_data[i].to_le_bytes())?;

        // Write all signal values as f64 for compatibility
        for signal in signal_data {
            if i < signal.len() {
                writer.write_all(&signal[i].to_le_bytes())?;
            } else {
                writer.write_all(&0.0f64.to_le_bytes())?;
            }
        }
    }

    Ok(())
}

/// Convert HSPICE result to SPICE3 binary raw format
pub fn write_spice3_raw(result: &HspiceResult, output_path: &str, debug: i32) -> Result<()> {
    if debug > 0 {
        eprintln!("Writing SPICE3 raw file: {}", output_path);
    }

    // Get the first data table (handle single sweep for now)
    let table = result
        .data_tables
        .first()
        .ok_or_else(|| HspiceError::ParseError("No data tables found".into()))?;

    // Extract scale data
    let scale_data = match table.get(&result.scale_name) {
        Some(VectorData::Real(data)) => data,
        Some(VectorData::Complex(_)) => {
            return Err(HspiceError::FormatError(
                "Complex scale not supported".into(),
            ));
        }
        None => {
            return Err(HspiceError::ParseError("Scale data not found".into()));
        }
    };

    let num_points = scale_data.len();

    if debug > 0 {
        eprintln!("  Points: {}", num_points);
        eprintln!("  Variables: {}", table.len());
    }

    // Collect variable names (excluding scale)
    let mut variable_names: Vec<String> = table
        .keys()
        .filter(|k| *k != &result.scale_name)
        .cloned()
        .collect();
    variable_names.sort(); // Consistent ordering

    // Collect signal data in order
    // TODO: Performance optimization opportunity
    // - Real data clone can be avoided using Cow<'_, [f64]> references
    // - Complex data must be converted to magnitude, so clone is necessary
    // - Current impact: ~2x memory for real signals during write
    // - Priority: Low (only affects write operation, not parsing)
    let signal_data: Vec<Vec<f64>> = variable_names
        .iter()
        .map(|name| match table.get(name) {
            Some(VectorData::Real(data)) => data.clone(),
            Some(VectorData::Complex(data)) => {
                // For complex, just use magnitude for now
                data.iter()
                    .map(|c| (c.re * c.re + c.im * c.im).sqrt())
                    .collect()
            }
            None => vec![0.0; num_points],
        })
        .collect();

    // Check for complex data
    let is_complex = table.values().any(|v| matches!(v, VectorData::Complex(_)));

    // Create output file
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Determine plot name based on scale
    let plot_name = match result.scale_name.to_uppercase().as_str() {
        "TIME" => "Transient Analysis",
        "FREQUENCY" | "FREQ" | "HERTZ" => "AC Analysis",
        _ => "DC Analysis",
    };

    // Write header
    write_raw_header(
        &mut writer,
        &result.title,
        &result.date,
        plot_name,
        &result.scale_name,
        &variable_names,
        num_points,
        is_complex,
    )?;

    // Write binary data
    write_raw_data(&mut writer, scale_data, &signal_data)?;

    writer.flush()?;

    if debug > 0 {
        eprintln!("  Wrote {} bytes", std::fs::metadata(output_path)?.len());
    }

    Ok(())
}

/// Convert HSPICE .tr0 file to SPICE3 binary raw format
pub fn hspice_to_raw_impl(input_path: &str, output_path: &str, debug: i32) -> Result<()> {
    use crate::parser::hspice_read_impl;

    // Read HSPICE file
    let result = hspice_read_impl(input_path, debug)?;

    // Write SPICE3 raw file
    write_spice3_raw(&result, output_path, debug)?;

    Ok(())
}
