//! SPICE3/ngspice raw file parser
//!
//! Supports both ASCII and binary raw file formats with auto-detection.

use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use num_complex::Complex64;
use tracing::{debug, info, instrument, trace};

use crate::types::{
    AnalysisType, DataTable, Result, VarType, Variable, VectorData, WaveformError, WaveformResult,
};

/// Raw file format type
#[derive(Debug, Clone, Copy, PartialEq)]
enum RawFormat {
    Binary,
    Ascii,
}

/// Parsed header information
#[derive(Debug, Default)]
struct RawHeader {
    title: String,
    date: String,
    plotname: String,
    flags: Vec<String>,
    num_variables: usize,
    num_points: usize,
    variables: Vec<(String, String)>, // (name, type)
    is_complex: bool,
}

/// Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format)
pub fn read_raw(filename: &str) -> Result<WaveformResult> {
    read_raw_impl(filename)
}

/// Reads a SPICE3/ngspice raw waveform from an in-memory byte slice.
///
/// # Errors
///
/// Returns an error if the header or waveform values are malformed.
pub fn read_raw_bytes(data: &[u8]) -> Result<WaveformResult> {
    let mut reader = BufReader::new(Cursor::new(data));
    parse_raw_reader(&mut reader)
}

/// Read a SPICE3/ngspice raw file with debug output
#[deprecated(
    since = "1.4.0",
    note = "Use read_raw() with tracing subscriber instead"
)]
pub fn read_raw_debug(filename: &str, _debug: i32) -> Result<WaveformResult> {
    read_raw_impl(filename)
}

#[instrument(skip_all, fields(file = %filename))]
fn read_raw_impl(filename: &str) -> Result<WaveformResult> {
    info!("Reading SPICE3 raw file");

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    parse_raw_reader(&mut reader)
}

fn parse_raw_reader<R: BufRead + Seek>(reader: &mut R) -> Result<WaveformResult> {
    let (header, format, data_start) = parse_header(reader)?;

    info!(
        format = ?format,
        variables = header.num_variables,
        points = header.num_points,
        complex = header.is_complex,
        "Header parsed"
    );

    debug!(title = %header.title, plotname = %header.plotname, "File info");

    reader.seek(SeekFrom::Start(data_start))?;

    let vectors = match format {
        RawFormat::Binary => parse_binary_data(reader, &header)?,
        RawFormat::Ascii => parse_ascii_data(reader, &header)?,
    };

    let analysis = infer_analysis_type(&header.plotname);
    let variables = build_variables(&header);

    info!(
        analysis = %analysis,
        vectors = vectors.len(),
        "Parsing complete"
    );

    Ok(WaveformResult {
        title: header.title,
        date: header.date,
        analysis,
        variables,
        sweep_param: None,
        tables: vec![DataTable {
            sweep_value: None,
            vectors,
        }],
    })
}

fn parse_header<R: BufRead + Seek>(reader: &mut R) -> Result<(RawHeader, RawFormat, u64)> {
    let mut header = RawHeader::default();
    let mut line = String::new();
    let mut in_variables = false;
    let mut var_count = 0;

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();

        // Check for data section markers
        if trimmed == "Binary:" {
            let pos = reader.stream_position()?;
            trace!(position = pos, "Found binary data section");
            return Ok((header, RawFormat::Binary, pos));
        }
        if trimmed == "Values:" {
            let pos = reader.stream_position()?;
            trace!(position = pos, "Found ASCII data section");
            return Ok((header, RawFormat::Ascii, pos));
        }

        // Parse header fields
        if let Some(value) = trimmed.strip_prefix("Title:") {
            header.title = value.trim().to_string();
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("Date:") {
            header.date = value.trim().to_string();
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("Plotname:") {
            header.plotname = value.trim().to_string();
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("Flags:") {
            header.flags = value.split_whitespace().map(str::to_owned).collect();
            header.is_complex = header.flags.iter().any(|f| f == "complex");
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("No. Variables:") {
            header.num_variables = parse_count(value, "variable count")?;
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("No. Points:") {
            header.num_points = parse_count(value, "point count")?;
            in_variables = false;
        } else if trimmed.starts_with("Variables:") {
            in_variables = true;
            var_count = 0;
        } else if in_variables && !trimmed.is_empty() {
            // Parse variable line: "index name type"
            let mut fields = trimmed.split_whitespace();
            if let (Some(_index), Some(name), Some(var_type)) =
                (fields.next(), fields.next(), fields.next())
            {
                header
                    .variables
                    .push((name.to_owned(), var_type.to_owned()));
                var_count += 1;
                if var_count >= header.num_variables {
                    in_variables = false;
                }
            }
        }
    }

    Err(WaveformError::ParseError(
        "No data section found in raw file".to_string(),
    ))
}

fn parse_count(value: &str, field: &str) -> Result<usize> {
    value.trim().parse().map_err(|error| {
        WaveformError::ParseError(format!("invalid {field} value {value:?}: {error}"))
    })
}

fn parse_binary_data<R: Read>(reader: &mut R, header: &RawHeader) -> Result<Vec<VectorData>> {
    let num_vars = header.num_variables;
    let num_points = header.num_points;

    trace!(
        num_vars = num_vars,
        num_points = num_points,
        complex = header.is_complex,
        "Parsing binary data"
    );

    if header.is_complex {
        // Complex data: all values are 16 bytes (two f64)
        let mut vectors: Vec<Vec<Complex64>> = vec![Vec::with_capacity(num_points); num_vars];

        for _ in 0..num_points {
            for vector in &mut vectors {
                let re = reader.read_f64::<LittleEndian>()?;
                let im = reader.read_f64::<LittleEndian>()?;
                vector.push(Complex64::new(re, im));
            }
        }

        Ok(vectors.into_iter().map(VectorData::Complex).collect())
    } else {
        // Real data: all values are f64 (ngspice default)
        let mut vectors: Vec<Vec<f64>> = vec![Vec::with_capacity(num_points); num_vars];

        for _ in 0..num_points {
            for vector in &mut vectors {
                vector.push(reader.read_f64::<LittleEndian>()?);
            }
        }

        Ok(vectors.into_iter().map(VectorData::Real).collect())
    }
}

fn parse_ascii_data<R: BufRead>(reader: &mut R, header: &RawHeader) -> Result<Vec<VectorData>> {
    let num_vars = header.num_variables;
    let num_points = header.num_points;

    trace!(
        num_vars = num_vars,
        num_points = num_points,
        complex = header.is_complex,
        "Parsing ASCII data"
    );

    if header.is_complex {
        let mut vectors: Vec<Vec<Complex64>> = vec![Vec::with_capacity(num_points); num_vars];
        let mut line = String::new();
        let mut current_point = 0;
        let mut current_var = 0;

        while current_point < num_points {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Format: "index\tvalue" or "index\treal,imag"
            let mut parts = trimmed.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };

            let value_str = if let Ok(point) = first.parse::<usize>() {
                current_var = 0;
                current_point = point;
                parts.next().ok_or_else(|| {
                    WaveformError::ParseError(format!(
                        "missing complex value for point {current_point}"
                    ))
                })?
            } else {
                parts.last().unwrap_or(first)
            };
            let (re, im) = parse_complex_value(value_str)?;

            if current_var < num_vars && current_point < num_points {
                vectors[current_var].push(Complex64::new(re, im));
            }
            current_var += 1;

            if current_var >= num_vars {
                current_point += 1;
                current_var = 0;
            }
        }

        Ok(vectors.into_iter().map(VectorData::Complex).collect())
    } else {
        let mut vectors: Vec<Vec<f64>> = vec![Vec::with_capacity(num_points); num_vars];
        let mut line = String::new();
        let mut current_point = 0;
        let mut current_var = 0;

        while current_point < num_points {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse values - format varies
            let mut parts = trimmed.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };

            if let Ok(point) = first.parse::<usize>() {
                current_point = point;
                current_var = 0;
                let value = parts
                    .next()
                    .ok_or_else(|| {
                        WaveformError::ParseError(format!(
                            "missing value for point {current_point}"
                        ))
                    })?
                    .parse::<f64>()
                    .map_err(|error| {
                        WaveformError::ParseError(format!(
                            "invalid value for point {current_point}: {error}"
                        ))
                    })?;
                if current_var < num_vars {
                    vectors[current_var].push(value);
                }
                current_var = 1;
            } else {
                let value = trimmed.parse::<f64>().map_err(|error| {
                    WaveformError::ParseError(format!("invalid ASCII value {trimmed:?}: {error}"))
                })?;
                if current_var < num_vars && vectors[current_var].len() < num_points {
                    vectors[current_var].push(value);
                }
                current_var += 1;

                if current_var >= num_vars {
                    current_point += 1;
                    current_var = 0;
                }
            }
        }

        Ok(vectors.into_iter().map(VectorData::Real).collect())
    }
}

fn parse_complex_value(s: &str) -> Result<(f64, f64)> {
    // Handle formats: "1.0,2.0" or "(1.0,2.0)" or "1.0+2.0j"
    let s = s.trim_matches(|c| c == '(' || c == ')');

    if let Some(pos) = s.find(',') {
        let re = s[..pos].trim().parse().map_err(|error| {
            WaveformError::ParseError(format!("invalid complex real part in {s:?}: {error}"))
        })?;
        let im = s[pos + 1..].trim().parse().map_err(|error| {
            WaveformError::ParseError(format!("invalid complex imaginary part in {s:?}: {error}"))
        })?;
        Ok((re, im))
    } else {
        let real = s.parse().map_err(|error| {
            WaveformError::ParseError(format!("invalid complex value {s:?}: {error}"))
        })?;
        Ok((real, 0.0))
    }
}

fn infer_analysis_type(plotname: &str) -> AnalysisType {
    let lower = plotname.to_lowercase();
    // Check DC before transient to avoid "tran" matching "characteristic"
    if lower.contains("dc") {
        AnalysisType::DC
    } else if lower.contains("transient") || lower.contains("tran") {
        AnalysisType::Transient
    } else if lower.contains("ac") {
        AnalysisType::AC
    } else if lower.contains("operating") || lower.contains("op") {
        AnalysisType::Operating
    } else if lower.contains("noise") {
        AnalysisType::Noise
    } else {
        AnalysisType::Unknown
    }
}

fn build_variables(header: &RawHeader) -> Vec<Variable> {
    header
        .variables
        .iter()
        .map(|(name, type_str)| {
            let var_type = if type_str.eq_ignore_ascii_case("time") {
                VarType::Time
            } else if type_str.eq_ignore_ascii_case("frequency") {
                VarType::Frequency
            } else if type_str.eq_ignore_ascii_case("voltage") {
                VarType::Voltage
            } else if type_str.eq_ignore_ascii_case("current") {
                VarType::Current
            } else {
                VarType::Unknown
            };
            Variable {
                name: name.clone(),
                var_type,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_analysis_type() {
        assert_eq!(
            infer_analysis_type("Transient Analysis"),
            AnalysisType::Transient
        );
        assert_eq!(infer_analysis_type("AC Analysis"), AnalysisType::AC);
        assert_eq!(
            infer_analysis_type("DC transfer characteristic"),
            AnalysisType::DC
        );
        assert_eq!(
            infer_analysis_type("Operating Point"),
            AnalysisType::Operating
        );
    }

    #[test]
    fn test_parse_complex_value() {
        assert_eq!(
            parse_complex_value("1.0,2.0").expect("valid complex value"),
            (1.0, 2.0)
        );
        assert_eq!(
            parse_complex_value("(1.5,-0.5)").expect("valid parenthesized complex value"),
            (1.5, -0.5)
        );
        assert_eq!(
            parse_complex_value("3.25").expect("valid real-only complex value"),
            (3.25, 0.0)
        );
    }
}
