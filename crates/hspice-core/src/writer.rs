//! SPICE3 Binary Raw File Writer

use crate::types::{AnalysisType, Result, VectorData, WaveformError, WaveformResult};
use std::fs::File;
use std::io::{BufWriter, Write};
use tracing::{debug, info, instrument};

/// Write SPICE3 binary raw file header
fn write_raw_header<W: Write>(
    writer: &mut W,
    title: &str,
    date: &str,
    plot_name: &str,
    result: &WaveformResult,
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
    writeln!(writer, "No. Variables: {}", result.variables.len())?;
    writeln!(writer, "No. Points: {}", num_points)?;
    writeln!(writer, "Variables:")?;

    // Write variables
    for (i, var) in result.variables.iter().enumerate() {
        writeln!(writer, "\t{}\t{}\t{}", i, var.name, var.var_type)?;
    }

    writeln!(writer, "Binary:")?;

    Ok(())
}

/// Write SPICE3 binary data section
fn write_raw_data<W: Write>(
    writer: &mut W,
    table: &crate::types::DataTable,
    num_points: usize,
) -> Result<()> {
    for i in 0..num_points {
        for vector in &table.vectors {
            match vector {
                VectorData::Real(data) => {
                    let val = data.get(i).copied().unwrap_or(0.0);
                    writer.write_all(&val.to_le_bytes())?;
                }
                VectorData::Complex(data) => {
                    // SPICE3 complex format: write real part then imaginary part (16 bytes total)
                    let c = data.get(i).copied().unwrap_or_default();
                    writer.write_all(&c.re.to_le_bytes())?;
                    writer.write_all(&c.im.to_le_bytes())?;
                }
            }
        }
    }

    Ok(())
}

/// Convert WaveformResult to SPICE3 binary raw format
#[instrument(skip(result), fields(output = %output_path))]
pub fn write_spice3_raw(result: &WaveformResult, output_path: &str) -> Result<()> {
    info!("Writing SPICE3 raw file");

    // Get the first data table
    let table = result
        .tables
        .first()
        .ok_or_else(|| WaveformError::ParseError("No data tables found".into()))?;

    let num_points = table.len();
    let num_vars = result.variables.len();

    debug!(points = num_points, variables = num_vars, "Data info");

    // Check for complex data
    let is_complex = table.vectors.iter().any(|v| v.is_complex());

    // Create output file
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Determine plot name based on analysis type
    let plot_name = match result.analysis {
        AnalysisType::Transient => "Transient Analysis",
        AnalysisType::AC => "AC Analysis",
        AnalysisType::DC => "DC Analysis",
        AnalysisType::Operating => "Operating Point",
        AnalysisType::Noise => "Noise Analysis",
        AnalysisType::Unknown => "Analysis",
    };

    // Write header
    write_raw_header(
        &mut writer,
        &result.title,
        &result.date,
        plot_name,
        result,
        num_points,
        is_complex,
    )?;

    // Write binary data
    write_raw_data(&mut writer, table, num_points)?;

    writer.flush()?;

    let bytes_written = std::fs::metadata(output_path)?.len();
    info!(bytes = bytes_written, "Write complete");

    Ok(())
}

/// Convert HSPICE .tr0 file to SPICE3 binary raw format
#[instrument(skip_all, fields(input = %input_path, output = %output_path))]
pub fn hspice_to_raw_impl(input_path: &str, output_path: &str) -> Result<()> {
    use crate::parser::hspice_read_impl;

    info!("Converting HSPICE to SPICE3 raw format");
    let result = hspice_read_impl(input_path)?;
    write_spice3_raw(&result, output_path)?;
    info!("Conversion complete");

    Ok(())
}
