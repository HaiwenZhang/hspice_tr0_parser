//! Subcommand implementations.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use hspice_core::{
    parse_header_only, read, read_and_convert, read_raw, read_stream_chunked, read_stream_signals,
    AnalysisType, PostVersion, Result, VectorData, WaveformError, WaveformResult, COMPLEX_VAR,
};
use memmap2::Mmap;

use crate::output::{print_result, print_signal, ChunkView, ResultView};

// ---------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------

pub fn cmd_info(file: &str) -> Result<()> {
    let input = File::open(file)?;
    // SAFETY: the CLI creates a read-only mapping and does not mutate the
    // underlying file during header parsing.
    let mmap = unsafe { Mmap::map(&input)? };
    let (meta, _data_offset) = parse_header_only(&mmap)?;

    let mut out = io::stdout().lock();
    writeln!(out, "File:        {file}")?;
    writeln!(out, "Title:       {}", meta.title)?;
    writeln!(out, "Date:        {}", meta.date)?;
    writeln!(out, "Post format: {:?}", meta.post_version)?;
    writeln!(out, "Scale:       {}", meta.scale_name)?;
    writeln!(
        out,
        "Data kind:   {}",
        if meta.var_type == COMPLEX_VAR {
            "complex"
        } else {
            "real"
        }
    )?;
    match &meta.sweep_name {
        Some(sweep) => writeln!(out, "Sweep:       {sweep} ({} point(s))", meta.sweep_size)?,
        None => writeln!(out, "Sweep:       (none)")?,
    }
    writeln!(
        out,
        "Variables:   {} (signals: {})",
        meta.num_variables,
        meta.names.len()
    )?;
    for (i, name) in meta.names.iter().enumerate() {
        writeln!(out, "  [{i:>3}] {name}")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// read / read-raw
// ---------------------------------------------------------------------------

pub fn cmd_read(file: &str, json: bool, signal: Option<&str>) -> Result<()> {
    let result = read(file)?;
    emit_result(file, &result, json, signal)
}

pub fn cmd_read_raw(file: &str, json: bool, signal: Option<&str>) -> Result<()> {
    let result = read_raw(file)?;
    emit_result(file, &result, json, signal)
}

fn emit_result(
    file: &str,
    result: &hspice_core::WaveformResult,
    json: bool,
    signal: Option<&str>,
) -> Result<()> {
    if let Some(name) = signal {
        return print_signal(result, name);
    }
    if json {
        let view = ResultView::from_result(result);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        serde_json::to_writer_pretty(&mut handle, &view)
            .map_err(|e| WaveformError::ParseError(format!("json serialize error: {e}")))?;
        writeln!(handle)?;
    } else {
        print_result(file, result)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// convert
// ---------------------------------------------------------------------------

pub fn cmd_convert(input: &str, output: &str, post_version: PostVersion) -> Result<()> {
    let input_extension = extension(input);
    let output_extension = extension(output);

    if is_raw_extension(input_extension) && is_hspice_extension(output_extension) {
        let result = read_raw(input)?;
        validate_output_analysis(output_extension, &result)?;
        hspice_core::write_hspice(&result, output, post_version)?;
    } else if is_hspice_extension(input_extension) && is_raw_extension(output_extension) {
        read_and_convert(input, output)?;
    } else {
        return Err(WaveformError::FormatError(
            "conversion requires .raw -> .tr0/.ac0/.sw0 or .tr0/.ac0/.sw0 -> .raw".into(),
        ));
    }
    eprintln!("Converted {} -> {}", input, output);
    Ok(())
}

fn extension(path: &str) -> Option<&str> {
    Path::new(path).extension()?.to_str()
}

fn is_raw_extension(extension: Option<&str>) -> bool {
    extension.is_some_and(|value| value.eq_ignore_ascii_case("raw"))
}

fn is_hspice_extension(extension: Option<&str>) -> bool {
    let Some(extension) = extension else {
        return false;
    };
    let bytes = extension.as_bytes();
    let prefix = &bytes[..bytes.len().min(2)];
    bytes.len() >= 3
        && (prefix.eq_ignore_ascii_case(b"tr")
            || prefix.eq_ignore_ascii_case(b"ac")
            || prefix.eq_ignore_ascii_case(b"sw"))
        && bytes[2..].iter().all(u8::is_ascii_digit)
}

fn validate_output_analysis(extension: Option<&str>, result: &WaveformResult) -> Result<()> {
    let extension = extension.ok_or_else(|| {
        WaveformError::FormatError("HSPICE output requires a file extension".into())
    })?;
    let expected = if extension
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("tr"))
    {
        AnalysisType::Transient
    } else if extension
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ac"))
    {
        AnalysisType::AC
    } else {
        AnalysisType::DC
    };
    let actual = if result.analysis == AnalysisType::Unknown {
        AnalysisType::from_scale_name(result.scale_name())
    } else {
        result.analysis
    };
    if actual != expected {
        return Err(WaveformError::FormatError(format!(
            "{actual} waveform must use the matching HSPICE extension, not .{extension}"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// stream
// ---------------------------------------------------------------------------

pub fn cmd_stream(file: &str, chunk_size: usize, signals: &[String]) -> Result<()> {
    let reader = if signals.is_empty() {
        read_stream_chunked(file, chunk_size)?
    } else {
        let refs: Vec<&str> = signals.iter().map(String::as_str).collect();
        read_stream_signals(file, &refs, chunk_size)?
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for chunk in reader {
        let chunk = chunk?;
        let view = ChunkView::from_chunk(&chunk);
        serde_json::to_writer(&mut handle, &view)
            .map_err(|e| WaveformError::ParseError(format!("json serialize error: {e}")))?;
        handle.write_all(b"\n")?;
        handle.flush()?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// scan
// ---------------------------------------------------------------------------

/// Decodes an entire file with bounded memory and prints a compact checksum.
pub fn cmd_scan(file: &str, chunk_size: usize) -> Result<()> {
    let reader = read_stream_chunked(file, chunk_size)?;
    let metadata = reader.metadata();
    let mut chunks = 0_usize;
    let mut points = 0_usize;
    let mut checksum = 0.0_f64;
    let mut scale_start = None;
    let mut scale_end = None;

    for chunk in reader {
        let chunk = chunk?;
        let scale = chunk
            .data
            .get(&metadata.scale_name)
            .and_then(VectorData::as_real_slice)
            .ok_or_else(|| WaveformError::ParseError("chunk has no real scale vector".into()))?;
        scale_start = scale_start.or_else(|| scale.first().copied());
        scale_end = scale.last().copied().or(scale_end);
        points += scale.len();
        chunks += 1;

        checksum += chunk
            .data
            .values()
            .map(|vector| match vector {
                VectorData::Real(values) => values.iter().sum::<f64>(),
                VectorData::Complex(values) => values.iter().map(|value| value.re + value.im).sum(),
            })
            .sum::<f64>();
    }

    let mut out = io::stdout().lock();
    writeln!(out, "File:       {file}")?;
    writeln!(out, "Signals:    {}", metadata.signal_names.len())?;
    writeln!(out, "Chunks:     {chunks}")?;
    writeln!(out, "Points:     {points}")?;
    writeln!(
        out,
        "Scale:      {} .. {}",
        scale_start.unwrap_or(0.0),
        scale_end.unwrap_or(0.0)
    )?;
    writeln!(out, "Checksum:   {checksum:.17e}")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// export (CSV)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ExportFormat {
    /// Auto-detect by file extension (.raw → SPICE3 raw, else HSPICE)
    Auto,
    /// Force HSPICE binary parser
    Hspice,
    /// Force SPICE3 / ngspice raw parser
    Raw,
}

pub fn cmd_export(
    file: &str,
    output: Option<&str>,
    format: ExportFormat,
    signals: &[String],
    delimiter: char,
) -> Result<()> {
    let result = match format {
        ExportFormat::Auto => {
            if Path::new(file)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("raw"))
            {
                read_raw(file)?
            } else {
                read(file)?
            }
        }
        ExportFormat::Hspice => read(file)?,
        ExportFormat::Raw => read_raw(file)?,
    };

    match output {
        Some(path) => {
            let mut w = BufWriter::new(File::create(path)?);
            write_csv(&mut w, &result, signals, delimiter)?;
            w.flush()?;
            eprintln!("Wrote CSV: {}", path);
        }
        None => {
            let stdout = io::stdout();
            let mut w = BufWriter::new(stdout.lock());
            write_csv(&mut w, &result, signals, delimiter)?;
            w.flush()?;
        }
    }
    Ok(())
}

fn write_csv<W: Write>(
    w: &mut W,
    r: &WaveformResult,
    filter: &[String],
    delim: char,
) -> Result<()> {
    // Resolve column indices.
    let columns: Vec<usize> = if filter.is_empty() {
        (0..r.variables.len()).collect()
    } else {
        let mut out = Vec::with_capacity(filter.len());
        for name in filter {
            let idx = r
                .var_index(name)
                .ok_or_else(|| WaveformError::ParseError(format!("signal '{}' not found", name)))?;
            out.push(idx);
        }
        out
    };

    let first_table = r
        .tables
        .first()
        .ok_or_else(|| WaveformError::ParseError("result has no data tables".into()))?;

    // Per-column type: real or complex (sampled from first table).
    let complex_flags: Vec<bool> = columns
        .iter()
        .map(|&i| matches!(first_table.vectors.get(i), Some(VectorData::Complex(_))))
        .collect();

    let has_sweep = r.has_sweep();
    let d = delim.to_string();

    // ---- Header ----
    let mut header_parts: Vec<String> = Vec::new();
    if has_sweep {
        header_parts.push(r.sweep_param.clone().unwrap_or_else(|| "sweep".into()));
    }
    for (&col, &is_cx) in columns.iter().zip(complex_flags.iter()) {
        let name = r
            .variables
            .get(col)
            .ok_or_else(|| {
                WaveformError::ParseError(format!("variable index {col} is out of range"))
            })?
            .name
            .as_str();
        if is_cx {
            header_parts.push(format!("{}.re", name));
            header_parts.push(format!("{}.im", name));
        } else {
            header_parts.push(name.to_owned());
        }
    }
    writeln!(w, "{}", header_parts.join(&d))?;

    // ---- Data ----
    for table in &r.tables {
        let n = table.len();
        for row in 0..n {
            let mut first = true;
            if has_sweep {
                write!(w, "{}", table.sweep_value.unwrap_or(0.0))?;
                first = false;
            }
            for (&col, &is_cx) in columns.iter().zip(complex_flags.iter()) {
                let vec = table.vectors.get(col).ok_or_else(|| {
                    WaveformError::ParseError(format!("column index {} out of range in table", col))
                })?;
                if !first {
                    write!(w, "{}", d)?;
                }
                first = false;
                match (vec, is_cx) {
                    (VectorData::Real(v), false) => write!(
                        w,
                        "{}",
                        v.get(row).ok_or_else(|| WaveformError::ParseError(format!(
                            "row {row} is missing from real column {col}"
                        )))?
                    )?,
                    (VectorData::Complex(v), true) => {
                        let z = v.get(row).ok_or_else(|| {
                            WaveformError::ParseError(format!(
                                "row {row} is missing from complex column {col}"
                            ))
                        })?;
                        write!(w, "{}{}{}", z.re, d, z.im)?;
                    }
                    // Type changed across sweep tables — shouldn't happen, but be safe.
                    (VectorData::Real(v), true) => {
                        let value = v.get(row).ok_or_else(|| {
                            WaveformError::ParseError(format!(
                                "row {row} is missing from real column {col}"
                            ))
                        })?;
                        write!(w, "{value}{d}0")?;
                    }
                    (VectorData::Complex(v), false) => {
                        let value = v.get(row).ok_or_else(|| {
                            WaveformError::ParseError(format!(
                                "row {row} is missing from complex column {col}"
                            ))
                        })?;
                        write!(w, "{}", value.re)?;
                    }
                }
            }
            writeln!(w)?;
        }
    }
    Ok(())
}
