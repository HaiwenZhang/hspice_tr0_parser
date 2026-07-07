//! Output helpers: serde views for `--json` mode and human-readable printers.

use std::collections::BTreeMap;
use std::io::{self, Write};

use hspice_core::{DataChunk, VectorData, WaveformResult};
use serde::Serialize;

// ============================================================================
// JSON Views
// ============================================================================

#[derive(Serialize)]
pub struct VarView<'a> {
    pub index: usize,
    pub name: &'a str,
    pub var_type: String,
}

#[derive(Serialize)]
pub struct TableSummary {
    pub index: usize,
    pub sweep_value: Option<f64>,
    pub points: usize,
}

#[derive(Serialize)]
pub struct ResultView<'a> {
    pub title: &'a str,
    pub date: &'a str,
    pub analysis: String,
    pub scale_name: &'a str,
    pub sweep_param: Option<&'a str>,
    pub num_variables: usize,
    pub num_points: usize,
    pub num_sweeps: usize,
    pub variables: Vec<VarView<'a>>,
    pub tables: Vec<TableSummary>,
}

impl<'a> ResultView<'a> {
    pub fn from_result(r: &'a WaveformResult) -> Self {
        let variables = r
            .variables
            .iter()
            .enumerate()
            .map(|(i, v)| VarView {
                index: i,
                name: v.name.as_str(),
                var_type: v.var_type.to_string(),
            })
            .collect();

        let tables = r
            .tables
            .iter()
            .enumerate()
            .map(|(i, t)| TableSummary {
                index: i,
                sweep_value: t.sweep_value,
                points: t.len(),
            })
            .collect();

        ResultView {
            title: &r.title,
            date: &r.date,
            analysis: r.analysis.to_string(),
            scale_name: r.scale_name(),
            sweep_param: r.sweep_param.as_deref(),
            num_variables: r.variables.len(),
            num_points: r.len(),
            num_sweeps: r.tables.len(),
            variables,
            tables,
        }
    }
}

/// Per-signal JSON payload for stream chunks.
/// Complex values are emitted as `[real, imag]` pairs.
#[derive(Serialize)]
#[serde(untagged)]
pub enum SignalValues<'a> {
    Real(&'a [f64]),
    Complex(Vec<[f64; 2]>),
}

impl<'a> SignalValues<'a> {
    pub fn from_vector(v: &'a VectorData) -> Self {
        match v {
            VectorData::Real(r) => SignalValues::Real(r.as_slice()),
            VectorData::Complex(c) => {
                SignalValues::Complex(c.iter().map(|z| [z.re, z.im]).collect())
            }
        }
    }
}

#[derive(Serialize)]
pub struct ChunkView<'a> {
    pub chunk_index: usize,
    pub time_range: [f64; 2],
    pub data: BTreeMap<&'a str, SignalValues<'a>>,
}

impl<'a> ChunkView<'a> {
    pub fn from_chunk(c: &'a DataChunk) -> Self {
        let data = c
            .data
            .iter()
            .map(|(name, vec)| (name.as_str(), SignalValues::from_vector(vec)))
            .collect();
        ChunkView {
            chunk_index: c.chunk_index,
            time_range: [c.time_range.0, c.time_range.1],
            data,
        }
    }
}

// ============================================================================
// Human-readable printers
// ============================================================================

pub fn print_result(file: &str, r: &WaveformResult) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "File:       {}", file)?;
    writeln!(out, "Title:      {}", r.title)?;
    writeln!(out, "Date:       {}", r.date)?;
    writeln!(out, "Analysis:   {}", r.analysis)?;
    writeln!(out, "Scale:      {}", r.scale_name())?;
    match &r.sweep_param {
        Some(s) => writeln!(out, "Sweep:      {} ({} table(s))", s, r.tables.len())?,
        None => writeln!(out, "Sweep:      (none)")?,
    }
    writeln!(out, "Variables:  {}", r.variables.len())?;
    let name_w = r.variables.iter().map(|v| v.name.len()).max().unwrap_or(0);
    for (i, v) in r.variables.iter().enumerate() {
        writeln!(
            out,
            "  [{:>3}] {:<width$}  ({})",
            i,
            v.name,
            v.var_type,
            width = name_w
        )?;
    }
    writeln!(out, "Tables:     {}", r.tables.len())?;
    for (i, t) in r.tables.iter().enumerate() {
        match t.sweep_value {
            Some(sv) => writeln!(out, "  table {}: {} points, sweep={}", i, t.len(), sv)?,
            None => writeln!(out, "  table {}: {} points", i, t.len())?,
        }
    }
    Ok(())
}

/// Print a single signal's values (one per line). For complex signals,
/// each line is `re,im`. Reads from the first table only.
pub fn print_signal(r: &WaveformResult, signal: &str) -> hspice_core::Result<()> {
    let idx = r.var_index(signal).ok_or_else(|| {
        hspice_core::WaveformError::ParseError(format!("signal '{}' not found", signal))
    })?;
    let table = r.tables.first().ok_or_else(|| {
        hspice_core::WaveformError::ParseError("result has no data tables".into())
    })?;
    let vec = &table.vectors[idx];

    let mut out = io::stdout().lock();
    match vec {
        VectorData::Real(v) => {
            for x in v {
                writeln!(out, "{}", x)?;
            }
        }
        VectorData::Complex(v) => {
            for z in v {
                writeln!(out, "{},{}", z.re, z.im)?;
            }
        }
    }
    Ok(())
}
