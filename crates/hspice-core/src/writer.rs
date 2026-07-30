//! SPICE3 binary raw-file writer.

use std::fs::File;
use std::io::{BufWriter, Write};

use tracing::{debug, info, instrument};

use crate::types::{AnalysisType, DataTable, Result, VectorData, WaveformError, WaveformResult};

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
    writeln!(writer, "Title: {title}")?;
    writeln!(writer, "Date: {date}")?;
    writeln!(writer, "Plotname: {plot_name}")?;
    writeln!(
        writer,
        "Flags: {}",
        if is_complex { "complex" } else { "real" }
    )?;
    writeln!(writer, "No. Variables: {}", result.variables.len())?;
    writeln!(writer, "No. Points: {num_points}")?;
    writeln!(writer, "Variables:")?;

    for (index, variable) in result.variables.iter().enumerate() {
        writeln!(
            writer,
            "\t{index}\t{}\t{}",
            variable.name, variable.var_type
        )?;
    }

    writeln!(writer, "Binary:")?;

    Ok(())
}

/// Write SPICE3 binary data section
fn write_raw_data<W: Write>(
    writer: &mut W,
    table: &DataTable,
    num_points: usize,
    is_complex: bool,
) -> Result<()> {
    for point_index in 0..num_points {
        for vector in &table.vectors {
            let (real, imaginary) = match vector {
                VectorData::Real(values) => {
                    let value = values.get(point_index).ok_or_else(|| {
                        WaveformError::FormatError(format!(
                            "real vector is missing point {point_index}"
                        ))
                    })?;
                    (*value, 0.0)
                }
                VectorData::Complex(values) => {
                    let value = values.get(point_index).ok_or_else(|| {
                        WaveformError::FormatError(format!(
                            "complex vector is missing point {point_index}"
                        ))
                    })?;
                    (value.re, value.im)
                }
            };
            writer.write_all(&real.to_le_bytes())?;
            if is_complex {
                // SPICE3's complex flag applies to every variable, including scale.
                writer.write_all(&imaginary.to_le_bytes())?;
            }
        }
    }

    Ok(())
}

/// Converts a [`WaveformResult`] to SPICE3 binary raw format.
///
/// # Errors
///
/// Returns an error if the result is inconsistent or the output cannot be written.
#[instrument(skip(result), fields(output = %output_path))]
pub fn write_spice3_raw(result: &WaveformResult, output_path: &str) -> Result<()> {
    info!("Writing SPICE3 raw file");

    let table = result
        .tables
        .first()
        .ok_or_else(|| WaveformError::ParseError("No data tables found".into()))?;

    let num_points = table.len();
    let num_vars = result.variables.len();

    debug!(points = num_points, variables = num_vars, "Data info");

    if table.vectors.len() != num_vars {
        return Err(WaveformError::FormatError(format!(
            "variable count ({num_vars}) does not match vector count ({})",
            table.vectors.len()
        )));
    }
    if let Some((index, length)) = table
        .vectors
        .iter()
        .enumerate()
        .map(|(index, vector)| (index, vector.len()))
        .find(|(_, length)| *length != num_points)
    {
        return Err(WaveformError::FormatError(format!(
            "vector {index} has {length} points; expected {num_points}"
        )));
    }

    let is_complex = table.vectors.iter().any(VectorData::is_complex);

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

    write_raw_data(&mut writer, table, num_points, is_complex)?;

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

#[cfg(test)]
mod tests {
    use num_complex::Complex64;

    use super::*;
    use crate::raw_parser::read_raw_bytes;
    use crate::types::{VarType, Variable};

    #[test]
    fn complex_round_trip_preserves_scale_and_signal() -> Result<()> {
        let path =
            std::env::temp_dir().join(format!("hspice_writer_complex_{}.raw", std::process::id()));
        let result = WaveformResult {
            title: "complex round trip".into(),
            date: String::new(),
            analysis: AnalysisType::AC,
            variables: vec![
                Variable::with_type("frequency", VarType::Frequency),
                Variable::with_type("v(out)", VarType::Voltage),
            ],
            sweep_param: None,
            tables: vec![DataTable {
                sweep_value: None,
                vectors: vec![
                    VectorData::Real(vec![1.0, 2.0]),
                    VectorData::Complex(vec![Complex64::new(3.0, 4.0), Complex64::new(5.0, 6.0)]),
                ],
            }],
        };

        write_spice3_raw(&result, path.to_string_lossy().as_ref())?;
        let bytes = std::fs::read(&path)?;
        let decoded = read_raw_bytes(&bytes)?;
        std::fs::remove_file(path)?;

        assert!(matches!(
            decoded.tables[0].vectors.as_slice(),
            [
                VectorData::Complex(scale),
                VectorData::Complex(signal)
            ] if scale == &[Complex64::new(1.0, 0.0), Complex64::new(2.0, 0.0)]
                && signal == &[Complex64::new(3.0, 4.0), Complex64::new(5.0, 6.0)]
        ));
        Ok(())
    }
}
