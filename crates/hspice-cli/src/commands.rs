//! Subcommand implementations.

use std::fs::File;
use std::io::{self, BufWriter, Write};

use hspice_core::{
    parse_header_only, read, read_and_convert, read_raw, read_stream_chunked,
    read_stream_signals, Result, VectorData, WaveformError, WaveformResult, COMPLEX_VAR,
};
use memmap2::Mmap;

use crate::output::{print_result, print_signal, ChunkView, ResultView};

// ---------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------

pub fn cmd_info(file: &str) -> Result<()> {
    let f = File::open(file)?;
    let mmap = unsafe { Mmap::map(&f)? };
    let (meta, _data_offset) = parse_header_only(&mmap)?;

    let mut out = io::stdout().lock();
    let _ = writeln!(out, "File:        {}", file);
    let _ = writeln!(out, "Title:       {}", meta.title);
    let _ = writeln!(out, "Date:        {}", meta.date);
    let _ = writeln!(out, "Post format: {:?}", meta.post_version);
    let _ = writeln!(out, "Scale:       {}", meta.scale_name);
    let _ = writeln!(
        out,
        "Data kind:   {}",
        if meta.var_type == COMPLEX_VAR {
            "complex"
        } else {
            "real"
        }
    );
    match &meta.sweep_name {
        Some(s) => {
            let _ = writeln!(out, "Sweep:       {} ({} point(s))", s, meta.sweep_size);
        }
        None => {
            let _ = writeln!(out, "Sweep:       (none)");
        }
    }
    let _ = writeln!(
        out,
        "Variables:   {} (signals: {})",
        meta.num_variables,
        meta.names.len()
    );
    for (i, name) in meta.names.iter().enumerate() {
        let _ = writeln!(out, "  [{:>3}] {}", i, name);
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

pub fn cmd_convert(input: &str, output: &str) -> Result<()> {
    read_and_convert(input, output)?;
    eprintln!("Converted {} -> {}", input, output);
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
            if file.to_lowercase().ends_with(".raw") {
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
            let idx = r.var_index(name).ok_or_else(|| {
                WaveformError::ParseError(format!("signal '{}' not found", name))
            })?;
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
        let name = &r.variables[col].name;
        if is_cx {
            header_parts.push(format!("{}.re", name));
            header_parts.push(format!("{}.im", name));
        } else {
            header_parts.push(name.clone());
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
                    WaveformError::ParseError(format!(
                        "column index {} out of range in table",
                        col
                    ))
                })?;
                if !first {
                    write!(w, "{}", d)?;
                }
                first = false;
                match (vec, is_cx) {
                    (VectorData::Real(v), false) => write!(w, "{}", v[row])?,
                    (VectorData::Complex(v), true) => {
                        let z = &v[row];
                        write!(w, "{}{}{}", z.re, d, z.im)?;
                    }
                    // Type changed across sweep tables — shouldn't happen, but be safe.
                    (VectorData::Real(v), true) => write!(w, "{}{}0", v[row], d)?,
                    (VectorData::Complex(v), false) => write!(w, "{}", v[row].re)?,
                }
            }
            writeln!(w)?;
        }
    }
    Ok(())
}
