//! SPICE3/ngspice raw file parser
//!
//! Supports both ASCII and binary raw file formats with auto-detection.

use crate::types::{
    AnalysisType, DataTable, Result, VarType, Variable, VectorData, WaveformError, WaveformResult,
};
use byteorder::{LittleEndian, ReadBytesExt};
use num_complex::Complex64;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

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
    read_raw_impl(filename, 0)
}

/// Read a SPICE3/ngspice raw file with debug output
pub fn read_raw_debug(filename: &str, debug: i32) -> Result<WaveformResult> {
    read_raw_impl(filename, debug)
}

fn read_raw_impl(filename: &str, debug: i32) -> Result<WaveformResult> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    // Read and parse header
    let (header, format, data_start) = parse_header(&mut reader, debug)?;

    if debug > 0 {
        eprintln!("Raw file: {}", filename);
        eprintln!("  Format: {:?}", format);
        eprintln!("  Title: {}", header.title);
        eprintln!("  Variables: {}", header.num_variables);
        eprintln!("  Points: {}", header.num_points);
        eprintln!("  Complex: {}", header.is_complex);
    }

    // Seek to data start
    reader.seek(SeekFrom::Start(data_start))?;

    // Parse data based on format
    let vectors = match format {
        RawFormat::Binary => parse_binary_data(&mut reader, &header, debug)?,
        RawFormat::Ascii => parse_ascii_data(&mut reader, &header, debug)?,
    };

    // Build WaveformResult
    let analysis = infer_analysis_type(&header.plotname);
    let variables = build_variables(&header);

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

fn parse_header<R: BufRead + Seek>(
    reader: &mut R,
    _debug: i32,
) -> Result<(RawHeader, RawFormat, u64)> {
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
            return Ok((header, RawFormat::Binary, pos));
        }
        if trimmed == "Values:" {
            let pos = reader.stream_position()?;
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
            header.flags = value.split_whitespace().map(|s| s.to_string()).collect();
            header.is_complex = header.flags.iter().any(|f| f == "complex");
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("No. Variables:") {
            header.num_variables = value.trim().parse().unwrap_or(0);
            in_variables = false;
        } else if let Some(value) = trimmed.strip_prefix("No. Points:") {
            header.num_points = value.trim().parse().unwrap_or(0);
            in_variables = false;
        } else if trimmed.starts_with("Variables:") {
            in_variables = true;
            var_count = 0;
        } else if in_variables && !trimmed.is_empty() {
            // Parse variable line: "index name type"
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[1].to_string();
                let var_type = parts[2].to_string();
                header.variables.push((name, var_type));
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

fn parse_binary_data<R: Read>(
    reader: &mut R,
    header: &RawHeader,
    _debug: i32,
) -> Result<Vec<VectorData>> {
    let num_vars = header.num_variables;
    let num_points = header.num_points;

    if header.is_complex {
        // Complex data: all values are 16 bytes (two f64)
        let mut vectors: Vec<Vec<Complex64>> = vec![Vec::with_capacity(num_points); num_vars];

        for _point in 0..num_points {
            for var_idx in 0..num_vars {
                let re = reader.read_f64::<LittleEndian>()?;
                let im = reader.read_f64::<LittleEndian>()?;
                vectors[var_idx].push(Complex64::new(re, im));
            }
        }

        Ok(vectors.into_iter().map(VectorData::Complex).collect())
    } else {
        // Real data: scale is f64, others are f64 (ngspice default)
        // Some tools use f32 for non-scale, but ngspice uses f64 for all
        let mut vectors: Vec<Vec<f64>> = vec![Vec::with_capacity(num_points); num_vars];

        for _point in 0..num_points {
            for var_idx in 0..num_vars {
                let value = reader.read_f64::<LittleEndian>()?;
                vectors[var_idx].push(value);
            }
        }

        Ok(vectors.into_iter().map(VectorData::Real).collect())
    }
}

fn parse_ascii_data<R: BufRead>(
    reader: &mut R,
    header: &RawHeader,
    _debug: i32,
) -> Result<Vec<VectorData>> {
    let num_vars = header.num_variables;
    let num_points = header.num_points;

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
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Check if this is a new point (starts with number)
            if parts[0].parse::<usize>().is_ok() && parts.len() >= 2 {
                current_var = 0;
                current_point = parts[0].parse().unwrap_or(current_point);
            }

            // Parse complex value
            let value_str = parts.last().unwrap_or(&"0,0");
            let (re, im) = parse_complex_value(value_str);

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
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Check if starts with point index
            if let Ok(idx) = parts[0].parse::<usize>() {
                if parts.len() >= 2 {
                    current_point = idx;
                    current_var = 0;
                    let value: f64 = parts[1].parse().unwrap_or(0.0);
                    if current_var < num_vars {
                        vectors[current_var].push(value);
                    }
                    current_var = 1;
                }
            } else {
                // Continuation line - just a value
                let value: f64 = trimmed.parse().unwrap_or(0.0);
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

fn parse_complex_value(s: &str) -> (f64, f64) {
    // Handle formats: "1.0,2.0" or "(1.0,2.0)" or "1.0+2.0j"
    let s = s.trim_matches(|c| c == '(' || c == ')');

    if let Some(pos) = s.find(',') {
        let re = s[..pos].trim().parse().unwrap_or(0.0);
        let im = s[pos + 1..].trim().parse().unwrap_or(0.0);
        (re, im)
    } else {
        (s.parse().unwrap_or(0.0), 0.0)
    }
}

fn infer_analysis_type(plotname: &str) -> AnalysisType {
    let lower = plotname.to_lowercase();
    if lower.contains("transient") || lower.contains("tran") {
        AnalysisType::Transient
    } else if lower.contains("ac") {
        AnalysisType::AC
    } else if lower.contains("dc") {
        AnalysisType::DC
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
            let var_type = match type_str.to_lowercase().as_str() {
                "time" => VarType::Time,
                "frequency" => VarType::Frequency,
                "voltage" => VarType::Voltage,
                "current" => VarType::Current,
                _ => VarType::Unknown,
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
        assert_eq!(parse_complex_value("1.0,2.0"), (1.0, 2.0));
        assert_eq!(parse_complex_value("(1.5,-0.5)"), (1.5, -0.5));
        assert_eq!(parse_complex_value("3.14"), (3.14, 0.0));
    }
}
