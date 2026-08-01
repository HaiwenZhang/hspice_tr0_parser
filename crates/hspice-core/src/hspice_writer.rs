//! HSPICE binary waveform writer.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use tracing::{info, instrument};

use crate::types::{
    AnalysisType, DataTable, PostVersion, Result, VarType, VectorData, WaveformError,
    WaveformResult, END_MARKER_2001, END_MARKER_9601,
};

const HEADER_FIXED_BYTES: usize = 256;
const MAX_RECORD_BYTES: usize = 8192;
const MAX_HEADER_VARIABLES: usize = 9999;
const NAME_ALIGNMENT: usize = 16;

/// Writes a waveform using the HSPICE binary record format.
///
/// The generated file uses little-endian records and the same fixed-width
/// header layout and 8192-byte data records found in the files under
/// `example/`. `V9601` stores scalar values as `f32`; `V2001` stores them as
/// `f64`.
///
/// # Errors
///
/// Returns an error if the waveform cannot be represented by HSPICE, contains
/// inconsistent vectors, or the output file cannot be written.
#[instrument(skip(result), fields(output = %output_path, ?post_version))]
pub fn write_hspice(
    result: &WaveformResult,
    output_path: &str,
    post_version: PostVersion,
) -> Result<()> {
    let prepared = prepare_waveform(result, post_version)?;
    validate_output_extension(output_path, prepared.analysis)?;
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    write_prepared(&mut writer, result, post_version, &prepared)?;
    writer.flush()?;
    info!("HSPICE waveform written");
    Ok(())
}

fn validate_output_extension(output_path: &str, analysis: AnalysisType) -> Result<()> {
    let extension = Path::new(output_path)
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            WaveformError::FormatError(
                "HSPICE output requires a .trN, .acN, or .swN extension".into(),
            )
        })?;
    let bytes = extension.as_bytes();
    if bytes.len() < 3 || !bytes[2..].iter().all(u8::is_ascii_digit) {
        return Err(WaveformError::FormatError(format!(
            "invalid HSPICE output extension .{extension}"
        )));
    }
    let expected_prefix = match analysis {
        AnalysisType::Transient => b"tr",
        AnalysisType::AC => b"ac",
        AnalysisType::DC => b"sw",
        _ => unreachable!("analysis was validated by resolve_analysis"),
    };
    if !bytes[..2].eq_ignore_ascii_case(expected_prefix) {
        return Err(WaveformError::FormatError(format!(
            "{analysis} waveform requires a .{}N extension, not .{extension}",
            String::from_utf8_lossy(expected_prefix)
        )));
    }
    Ok(())
}

#[cfg(test)]
fn write_hspice_stream<W: Write>(
    writer: &mut W,
    result: &WaveformResult,
    post_version: PostVersion,
) -> Result<()> {
    let prepared = prepare_waveform(result, post_version)?;
    write_prepared(writer, result, post_version, &prepared)
}

struct PreparedWaveform {
    analysis: AnalysisType,
    header: Vec<u8>,
}

fn prepare_waveform(
    result: &WaveformResult,
    post_version: PostVersion,
) -> Result<PreparedWaveform> {
    let analysis = resolve_analysis(result)?;
    let names = validated_names(result, analysis)?;
    validate_tables(result, analysis, post_version)?;
    let header = build_header(result, analysis, post_version, &names)?;
    Ok(PreparedWaveform { analysis, header })
}

fn write_prepared<W: Write>(
    writer: &mut W,
    result: &WaveformResult,
    post_version: PostVersion,
    prepared: &PreparedWaveform,
) -> Result<()> {
    for payload in prepared.header.chunks(MAX_RECORD_BYTES) {
        write_record(writer, payload, 8)?;
    }

    for table in &result.tables {
        write_data_table(
            writer,
            table,
            prepared.analysis,
            result.sweep_param.is_some(),
            post_version,
        )?;
    }
    Ok(())
}

fn resolve_analysis(result: &WaveformResult) -> Result<AnalysisType> {
    let analysis = match result.analysis {
        AnalysisType::Unknown => AnalysisType::from_scale_name(result.scale_name()),
        analysis => analysis,
    };
    match analysis {
        AnalysisType::Transient | AnalysisType::AC | AnalysisType::DC => Ok(analysis),
        unsupported => Err(WaveformError::FormatError(format!(
            "HSPICE output does not support {unsupported} analysis"
        ))),
    }
}

fn validated_names(result: &WaveformResult, analysis: AnalysisType) -> Result<Vec<String>> {
    if result.variables.is_empty() {
        return Err(WaveformError::FormatError(
            "waveform has no variables".into(),
        ));
    }
    if result.variables.len() > MAX_HEADER_VARIABLES {
        return Err(WaveformError::FormatError(format!(
            "HSPICE header supports at most {MAX_HEADER_VARIABLES} variables"
        )));
    }

    let mut names = Vec::with_capacity(result.variables.len());
    names.push(match analysis {
        AnalysisType::Transient => "TIME".to_owned(),
        AnalysisType::AC => "HERTZ".to_owned(),
        AnalysisType::DC => normalize_name(result.scale_name())?,
        _ => unreachable!("analysis was validated by resolve_analysis"),
    });
    names.extend(
        result
            .variables
            .iter()
            .skip(1)
            .map(|variable| normalize_name(&variable.name))
            .collect::<Result<Vec<_>>>()?,
    );

    let mut sorted = names.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(WaveformError::FormatError(
            "variable names are not unique after HSPICE normalization".into(),
        ));
    }
    Ok(names)
}

fn normalize_name(name: &str) -> Result<String> {
    if name.is_empty() {
        return Err(WaveformError::FormatError(
            "variable name cannot be empty".into(),
        ));
    }
    if !name.is_ascii() || name.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(WaveformError::FormatError(format!(
            "HSPICE variable name must be non-empty ASCII without whitespace: {name:?}"
        )));
    }

    let is_probe = name.get(..2).is_some_and(|prefix| {
        prefix.eq_ignore_ascii_case("v(") || prefix.eq_ignore_ascii_case("i(")
    });
    Ok(if is_probe && name.ends_with(')') {
        name[..name.len() - 1].to_owned()
    } else {
        name.to_owned()
    })
}

fn validate_tables(
    result: &WaveformResult,
    analysis: AnalysisType,
    post_version: PostVersion,
) -> Result<()> {
    if result.tables.is_empty() {
        return Err(WaveformError::FormatError(
            "waveform has no data tables".into(),
        ));
    }
    if result.tables.len() > 1 && result.sweep_param.is_none() {
        return Err(WaveformError::FormatError(
            "multiple data tables require a sweep parameter".into(),
        ));
    }
    if let Some(sweep_name) = &result.sweep_param {
        normalize_name(sweep_name)?;
    }

    for (table_index, table) in result.tables.iter().enumerate() {
        if table.vectors.len() != result.variables.len() {
            return Err(WaveformError::FormatError(format!(
                "table {table_index} has {} vectors; expected {}",
                table.vectors.len(),
                result.variables.len()
            )));
        }
        let point_count = table.vectors.first().map_or(0, VectorData::len);
        if point_count == 0 {
            return Err(WaveformError::FormatError(format!(
                "table {table_index} has no data points"
            )));
        }
        if let Some((vector_index, length)) = table
            .vectors
            .iter()
            .enumerate()
            .map(|(index, vector)| (index, vector.len()))
            .find(|(_, length)| *length != point_count)
        {
            return Err(WaveformError::FormatError(format!(
                "table {table_index} vector {vector_index} has {length} points; expected {point_count}"
            )));
        }
        if result.sweep_param.is_some() {
            let sweep_value = table.sweep_value.ok_or_else(|| {
                WaveformError::FormatError(format!(
                    "table {table_index} is missing its sweep value"
                ))
            })?;
            validate_scalar(sweep_value, post_version)?;
        }

        for (vector_index, vector) in table.vectors.iter().enumerate() {
            match vector {
                VectorData::Real(values) => {
                    for &value in values {
                        validate_scalar(value, post_version)?;
                    }
                }
                VectorData::Complex(values) => {
                    for value in values {
                        validate_scalar(value.re, post_version)?;
                        if analysis == AnalysisType::AC && vector_index > 0 {
                            validate_scalar(value.im, post_version)?;
                        } else if value.im != 0.0 {
                            return Err(WaveformError::FormatError(format!(
                                "complex value in table {table_index} vector {vector_index} cannot be written as {analysis} data"
                            )));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_scalar(value: f64, post_version: PostVersion) -> Result<()> {
    if !value.is_finite() {
        return Err(WaveformError::FormatError(format!(
            "HSPICE output requires finite values, found {value}"
        )));
    }
    let representable = match post_version {
        PostVersion::V9601 => {
            let encoded = value as f32;
            encoded.is_finite() && encoded < END_MARKER_9601
        }
        PostVersion::V2001 => value < END_MARKER_2001,
    };
    if !representable {
        return Err(WaveformError::FormatError(format!(
            "value {value} collides with the HSPICE end marker or selected precision"
        )));
    }
    Ok(())
}

fn build_header(
    result: &WaveformResult,
    analysis: AnalysisType,
    post_version: PostVersion,
    names: &[String],
) -> Result<Vec<u8>> {
    let mut header = vec![b' '; HEADER_FIXED_BYTES];
    put_ascii(&mut header, 0, 4, &format!("{:04}", names.len()))?;
    put_ascii(&mut header, 4, 4, "0000")?;
    put_ascii(
        &mut header,
        8,
        4,
        if result.sweep_param.is_some() {
            "0001"
        } else {
            "0000"
        },
    )?;
    put_ascii(&mut header, 12, 4, "0000")?;
    match post_version {
        PostVersion::V9601 => put_ascii(&mut header, 16, 8, "9601    ")?,
        PostVersion::V2001 => put_ascii(&mut header, 16, 8, "00002001")?,
    }

    put_truncated_ascii(&mut header, 24, 64, &result.title);
    put_truncated_ascii(&mut header, 88, 24, &result.date);
    let year = date_year(&result.date).unwrap_or("2026");
    let copyright = format!(" Copyright (c) 1986 - {year} by Synopsys, Inc. All Rights Reserved.");
    put_truncated_ascii(&mut header, 112, 65, &copyright);

    let sweep_size = if result.sweep_param.is_some() {
        result.tables.len()
    } else {
        0
    };
    if result.sweep_param.is_some() && post_version == PostVersion::V9601 {
        put_ascii(&mut header, 176, 10, &format!("{sweep_size:<10}"))?;
    }
    put_ascii(&mut header, 187, 10, &format!("{sweep_size:<10}"))?;

    header.extend_from_slice(b"   ");
    for (index, variable) in result.variables.iter().enumerate() {
        let code = if index == 0 {
            analysis_code(analysis)
        } else {
            signal_code(variable.var_type, &variable.name)
        };
        header.extend_from_slice(format!("{code:>8}").as_bytes());
    }
    pad_to_alignment(&mut header, NAME_ALIGNMENT);

    append_name_slot(&mut header, &names[0], slot_width(names[0].len()));
    let signal_width = names
        .iter()
        .skip(1)
        .map(|name| name.len())
        .max()
        .map_or(NAME_ALIGNMENT, slot_width);
    for name in names.iter().skip(1) {
        append_name_slot(&mut header, name, signal_width);
    }
    if let Some(sweep_name) = &result.sweep_param {
        let sweep_name = normalize_name(sweep_name)?;
        append_name_slot(&mut header, &sweep_name, slot_width(sweep_name.len()));
    }
    header.extend_from_slice(b"$&%#");
    pad_to_alignment(&mut header, 8);
    Ok(header)
}

fn put_ascii(buffer: &mut [u8], start: usize, width: usize, value: &str) -> Result<()> {
    if !value.is_ascii() || value.len() != width {
        return Err(WaveformError::FormatError(format!(
            "invalid fixed-width HSPICE header field {value:?}"
        )));
    }
    buffer[start..start + width].copy_from_slice(value.as_bytes());
    Ok(())
}

fn put_truncated_ascii(buffer: &mut [u8], start: usize, width: usize, value: &str) {
    for (written, &byte) in value.as_bytes().iter().take(width).enumerate() {
        buffer[start + written] = if byte.is_ascii() { byte } else { b'?' };
    }
}

fn date_year(date: &str) -> Option<&str> {
    date.as_bytes()
        .windows(4)
        .enumerate()
        .rev()
        .find(|(_, bytes)| bytes.iter().all(u8::is_ascii_digit))
        .map(|(start, _)| &date[start..start + 4])
}

fn analysis_code(analysis: AnalysisType) -> i32 {
    match analysis {
        AnalysisType::Transient => 1,
        AnalysisType::AC => 2,
        AnalysisType::DC => 3,
        _ => unreachable!("analysis was validated by resolve_analysis"),
    }
}

fn signal_code(var_type: VarType, name: &str) -> i32 {
    if var_type == VarType::Current || VarType::from_name(name) == VarType::Current {
        8
    } else {
        1
    }
}

const fn slot_width(name_len: usize) -> usize {
    (name_len / NAME_ALIGNMENT + 1) * NAME_ALIGNMENT
}

fn append_name_slot(header: &mut Vec<u8>, name: &str, width: usize) {
    header.extend_from_slice(name.as_bytes());
    header.resize(header.len() + width - name.len(), b' ');
}

fn pad_to_alignment(bytes: &mut Vec<u8>, alignment: usize) {
    let remainder = bytes.len() % alignment;
    if remainder != 0 {
        bytes.resize(bytes.len() + alignment - remainder, b' ');
    }
}

fn write_data_table<W: Write>(
    writer: &mut W,
    table: &DataTable,
    analysis: AnalysisType,
    has_sweep: bool,
    post_version: PostVersion,
) -> Result<()> {
    let mut payload = Vec::with_capacity(MAX_RECORD_BYTES);
    if has_sweep {
        push_scalar(
            writer,
            &mut payload,
            table
                .sweep_value
                .ok_or_else(|| WaveformError::FormatError("missing sweep value".into()))?,
            post_version,
        )?;
    }

    for point_index in 0..table.len() {
        for (vector_index, vector) in table.vectors.iter().enumerate() {
            match vector {
                VectorData::Real(values) => {
                    push_scalar(writer, &mut payload, values[point_index], post_version)?;
                    if analysis == AnalysisType::AC && vector_index > 0 {
                        push_scalar(writer, &mut payload, 0.0, post_version)?;
                    }
                }
                VectorData::Complex(values) => {
                    let value = values[point_index];
                    push_scalar(writer, &mut payload, value.re, post_version)?;
                    if analysis == AnalysisType::AC && vector_index > 0 {
                        push_scalar(writer, &mut payload, value.im, post_version)?;
                    }
                }
            }
        }
    }

    let marker = match post_version {
        PostVersion::V9601 => f64::from(END_MARKER_9601),
        PostVersion::V2001 => END_MARKER_2001,
    };
    push_scalar(writer, &mut payload, marker, post_version)?;
    flush_data_record(writer, &mut payload, post_version)
}

fn push_scalar<W: Write>(
    writer: &mut W,
    payload: &mut Vec<u8>,
    value: f64,
    post_version: PostVersion,
) -> Result<()> {
    let item_size = item_size(post_version);
    if payload.len() + item_size > MAX_RECORD_BYTES {
        flush_data_record(writer, payload, post_version)?;
    }
    match post_version {
        PostVersion::V9601 => payload.extend_from_slice(&(value as f32).to_le_bytes()),
        PostVersion::V2001 => payload.extend_from_slice(&value.to_le_bytes()),
    }
    if payload.len() == MAX_RECORD_BYTES {
        flush_data_record(writer, payload, post_version)?;
    }
    Ok(())
}

fn flush_data_record<W: Write>(
    writer: &mut W,
    payload: &mut Vec<u8>,
    post_version: PostVersion,
) -> Result<()> {
    if !payload.is_empty() {
        write_record(writer, payload, item_size(post_version))?;
        payload.clear();
    }
    Ok(())
}

const fn item_size(post_version: PostVersion) -> usize {
    match post_version {
        PostVersion::V9601 => 4,
        PostVersion::V2001 => 8,
    }
}

fn write_record<W: Write>(writer: &mut W, payload: &[u8], item_size: usize) -> Result<()> {
    if payload.is_empty() || !payload.len().is_multiple_of(item_size) {
        return Err(WaveformError::FormatError(format!(
            "invalid HSPICE record length {} for {item_size}-byte items",
            payload.len()
        )));
    }
    let byte_count = i32::try_from(payload.len())
        .map_err(|_| WaveformError::FormatError("HSPICE record is too large".into()))?;
    let item_count = i32::try_from(payload.len() / item_size)
        .map_err(|_| WaveformError::FormatError("HSPICE item count is too large".into()))?;

    writer.write_all(&4_i32.to_le_bytes())?;
    writer.write_all(&item_count.to_le_bytes())?;
    writer.write_all(&4_i32.to_le_bytes())?;
    writer.write_all(&byte_count.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.write_all(&byte_count.to_le_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use num_complex::Complex64;

    use super::*;
    use crate::parser::hspice_read_bytes_impl;
    use crate::types::Variable;

    fn transient_result(points: usize) -> WaveformResult {
        WaveformResult {
            title: "* generated test".into(),
            date: "08/01/2026      12:00:00".into(),
            analysis: AnalysisType::Transient,
            variables: vec![
                Variable::with_type("time", VarType::Time),
                Variable::with_type("v(out)", VarType::Voltage),
                Variable::with_type("i(vdd)", VarType::Current),
            ],
            sweep_param: None,
            tables: vec![DataTable {
                sweep_value: None,
                vectors: vec![
                    VectorData::Real((0..points).map(|value| value as f64).collect()),
                    VectorData::Real((0..points).map(|value| value as f64 * 2.0).collect()),
                    VectorData::Real((0..points).map(|value| -(value as f64)).collect()),
                ],
            }],
        }
    }

    #[test]
    fn transient_9601_round_trip_preserves_shape_and_names() -> Result<()> {
        let result = transient_result(4);
        let mut bytes = Vec::new();
        write_hspice_stream(&mut bytes, &result, PostVersion::V9601)?;
        let decoded = hspice_read_bytes_impl(&bytes, "generated.tr0")?;

        assert_eq!(decoded.var_names(), vec!["TIME", "v(out", "i(vdd"]);
        Ok(())
    }

    #[test]
    fn header_matches_reference_field_layout() -> Result<()> {
        let result = transient_result(1);
        let names = validated_names(&result, AnalysisType::Transient)?;
        let header = build_header(&result, AnalysisType::Transient, PostVersion::V9601, &names)?;

        assert_eq!(&header[..24], b"00030000000000009601    ");
        assert_eq!(&header[256..288], b"          1       1       8     ");
        assert_eq!(
            &header[288..336],
            b"TIME            v(out           i(vdd           "
        );
        Ok(())
    }

    #[test]
    fn writer_splits_large_data_into_8192_byte_records() -> Result<()> {
        let result = transient_result(3_000);
        let mut bytes = Vec::new();
        write_hspice_stream(&mut bytes, &result, PostVersion::V9601)?;

        let decoded = hspice_read_bytes_impl(&bytes, "large.tr0")?;
        assert_eq!(decoded.len(), 3_000);
        Ok(())
    }

    #[test]
    fn ac_round_trip_preserves_complex_signal() -> Result<()> {
        let result = WaveformResult {
            title: "AC test".into(),
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
                    VectorData::Complex(vec![Complex64::new(1.0, 0.0)]),
                    VectorData::Complex(vec![Complex64::new(2.0, -3.0)]),
                ],
            }],
        };
        let mut bytes = Vec::new();
        write_hspice_stream(&mut bytes, &result, PostVersion::V2001)?;
        let decoded = hspice_read_bytes_impl(&bytes, "generated.ac0")?;

        assert!(matches!(
            decoded.tables[0].vectors[1].as_complex_slice(),
            Some(values) if values == [Complex64::new(2.0, -3.0)]
        ));
        Ok(())
    }

    #[test]
    fn swept_waveform_round_trip_preserves_tables() -> Result<()> {
        let mut result = transient_result(2);
        result.sweep_param = Some("corner".into());
        result.tables[0].sweep_value = Some(1.0);
        let mut second = result.tables[0].clone();
        second.sweep_value = Some(2.0);
        result.tables.push(second);
        let mut bytes = Vec::new();
        write_hspice_stream(&mut bytes, &result, PostVersion::V9601)?;
        let decoded = hspice_read_bytes_impl(&bytes, "swept.tr0")?;

        assert_eq!(
            decoded
                .tables
                .iter()
                .map(|table| table.sweep_value)
                .collect::<Vec<_>>(),
            vec![Some(1.0), Some(2.0)]
        );
        Ok(())
    }

    #[test]
    fn writer_rejects_complex_transient_values() {
        let mut result = transient_result(1);
        result.tables[0].vectors[1] = VectorData::Complex(vec![Complex64::new(1.0, 2.0)]);
        let error = write_hspice_stream(&mut Vec::new(), &result, PostVersion::V9601)
            .expect_err("complex transient data should be rejected");

        assert!(error.to_string().contains("cannot be written as transient"));
    }
}
