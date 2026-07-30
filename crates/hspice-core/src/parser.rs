//! HSPICE binary file parser.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use tracing::{debug, info, instrument, trace};

use crate::data_builder::DataTableBuilder;
use crate::reader::MmapReader;
use crate::types::{
    AnalysisType, DataTable, PostVersion, Result, Variable, WaveformError, WaveformResult,
    COMPLEX_VAR, DATE_END_POSITION, DATE_START_POSITION, FREQUENCY_TYPE, NUM_OF_PROBES_POSITION,
    NUM_OF_SWEEPS_END_POSITION, NUM_OF_SWEEPS_POSITION, NUM_OF_VARIABLES_POSITION,
    POST_START_POSITION1, POST_START_POSITION2, POST_STRING11, POST_STRING12, POST_STRING21,
    REAL_VAR, SWEEP_SIZE_POSITION1, SWEEP_SIZE_POSITION2, TITLE_START_POSITION,
    VECTOR_DESCRIPTION_START_POSITION,
};

/// Find subsequence in a byte slice
#[inline]
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Read header blocks until end marker found
fn read_header_blocks(reader: &mut MmapReader) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(4096);

    loop {
        let (num_items, trailer) = reader.read_block_header(1)?;
        let block_data = reader.read_bytes(num_items)?;
        reader.read_block_trailer(trailer)?;

        buffer.extend_from_slice(block_data);

        if let Some(pos) = find_subsequence(&buffer, b"$&%#") {
            buffer.truncate(pos);
            break;
        }
    }

    Ok(buffer)
}

// ============================================================================
// String extraction utilities
// ============================================================================

#[inline]
fn extract_string(buf: &[u8], start: usize, end: usize) -> String {
    if start >= buf.len() || end > buf.len() || start >= end {
        return String::new();
    }
    let slice = &buf[start..end];
    let end_pos = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end_pos])
        .trim()
        .to_string()
}

#[inline]
fn extract_int(buf: &[u8], start: usize, end: usize, field: &str) -> Result<i32> {
    let value = extract_string(buf, start, end);
    value.trim().parse().map_err(|error| {
        WaveformError::ParseError(format!("invalid {field} value {value:?}: {error}"))
    })
}

// ============================================================================
// Header parsing
// ============================================================================

/// Parsed header metadata
#[derive(Debug, Clone)]
pub struct HeaderMetadata {
    pub title: String,
    pub date: String,
    pub post_version: PostVersion,
    pub num_variables: i32,
    pub num_vectors: usize,
    pub var_type: i32,
    pub scale_name: String,
    pub names: Vec<String>,
    pub sweep_name: Option<String>,
    pub sweep_size: i32,
}

/// Parse vector names from header buffer
fn parse_vector_names(buf: &[u8], num_vectors: usize) -> Result<(String, Vec<String>)> {
    if buf.len() < VECTOR_DESCRIPTION_START_POSITION {
        return Err(WaveformError::ParseError("Buffer too short".into()));
    }

    let desc_section = &buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);
    let tokens: Vec<&str> = desc_str.split_whitespace().collect();

    if tokens.len() < num_vectors + 1 {
        return Err(WaveformError::ParseError("Not enough vector names".into()));
    }

    let scale_name = tokens.get(num_vectors).unwrap_or(&"time").to_string();

    let names: Vec<String> = ((num_vectors + 1)..(2 * num_vectors))
        .filter_map(|i| tokens.get(i))
        .map(|name| (*name).to_owned())
        .collect();

    Ok((scale_name, names))
}

/// Get sweep info from header tokens
fn get_sweep_info(
    buf: &[u8],
    tokens: &[&str],
    num_vectors: usize,
) -> Result<Option<(String, i32)>> {
    let Some(sweep_name) = tokens.get(2 * num_vectors) else {
        return Ok(None);
    };
    let post_str = extract_string(buf, POST_START_POSITION2, POST_START_POSITION2 + 4);
    let sweep_size = if post_str == POST_STRING21 {
        extract_int(
            buf,
            SWEEP_SIZE_POSITION2,
            SWEEP_SIZE_POSITION2 + 10,
            "sweep size",
        )?
    } else {
        extract_int(
            buf,
            SWEEP_SIZE_POSITION1,
            SWEEP_SIZE_POSITION1 + 10,
            "sweep size",
        )?
    };
    Ok(Some(((*sweep_name).to_owned(), sweep_size)))
}

/// Parse all header metadata from buffer
fn parse_header_metadata(header_buf: &[u8]) -> Result<HeaderMetadata> {
    let post1 = extract_string(header_buf, POST_START_POSITION1, POST_START_POSITION1 + 4);
    let post2 = extract_string(header_buf, POST_START_POSITION2, POST_START_POSITION2 + 4);

    if post1 != POST_STRING11 && post1 != POST_STRING12 && post2 != POST_STRING21 {
        return Err(WaveformError::FormatError("Unknown post format".into()));
    }

    let post_version = if post2 == POST_STRING21 {
        PostVersion::V2001
    } else {
        PostVersion::V9601
    };

    let date = extract_string(header_buf, DATE_START_POSITION, DATE_END_POSITION);
    let title_end = {
        let mut end = DATE_START_POSITION;
        while end > TITLE_START_POSITION && header_buf.get(end - 1) == Some(&b' ') {
            end -= 1;
        }
        end
    };
    let title = extract_string(header_buf, TITLE_START_POSITION, title_end);

    let num_sweeps = extract_int(
        header_buf,
        NUM_OF_SWEEPS_POSITION,
        NUM_OF_SWEEPS_END_POSITION,
        "number of sweeps",
    )?;
    if !(0..=1).contains(&num_sweeps) {
        return Err(WaveformError::FormatError(
            "Only one-dimensional sweep supported".into(),
        ));
    }

    let num_probes = extract_int(
        header_buf,
        NUM_OF_PROBES_POSITION,
        NUM_OF_SWEEPS_POSITION,
        "number of probes",
    )?;
    let num_variables = extract_int(
        header_buf,
        NUM_OF_VARIABLES_POSITION,
        NUM_OF_PROBES_POSITION,
        "number of variables",
    )?;
    let num_vectors_i32 = num_probes
        .checked_add(num_variables)
        .ok_or_else(|| WaveformError::FormatError("vector count overflow".into()))?;
    let num_vectors = usize::try_from(num_vectors_i32).map_err(|_| {
        WaveformError::FormatError(format!("invalid vector count: {num_vectors_i32}"))
    })?;

    let desc_section = &header_buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);
    let tokens: Vec<&str> = desc_str.split_whitespace().collect();
    let var_type_num = tokens
        .first()
        .ok_or_else(|| WaveformError::ParseError("missing variable type".into()))?
        .parse::<i32>()
        .map_err(|error| WaveformError::ParseError(format!("invalid variable type: {error}")))?;
    let var_type = if var_type_num == FREQUENCY_TYPE {
        COMPLEX_VAR
    } else {
        REAL_VAR
    };

    let (scale_name, names) = parse_vector_names(header_buf, num_vectors)?;

    let (sweep_name, sweep_size) = if num_sweeps == 1 {
        get_sweep_info(header_buf, &tokens, num_vectors)?
            .map(|(name, size)| (Some(name), size.max(1)))
            .unwrap_or((None, 1))
    } else {
        (None, 1)
    };

    Ok(HeaderMetadata {
        title,
        date,
        post_version,
        num_variables,
        num_vectors,
        var_type,
        scale_name,
        names,
        sweep_name,
        sweep_size,
    })
}

/// Estimate rows before decoding so every output vector allocates only once.
/// Record headers make this a small overestimate, which is preferable to
/// reallocating gigabyte-scale vectors.
fn estimate_table_rows(
    data_bytes: usize,
    version: PostVersion,
    num_columns: usize,
    remaining_sweeps: usize,
) -> usize {
    if num_columns == 0 {
        return 0;
    }
    let item_size = match version {
        PostVersion::V9601 => 4,
        PostVersion::V2001 => 8,
    };
    data_bytes / remaining_sweeps.max(1) / item_size / num_columns
}

/// Read one table directly into its final vectors.
fn read_data_table(
    reader: &mut MmapReader,
    meta: &HeaderMetadata,
    remaining_sweeps: usize,
) -> Result<DataTable> {
    use crate::block_reader::BlockReader;

    let (table, consumed, blocks, format, trailing_values) = {
        let data = reader.remaining_bytes();
        let num_columns = if meta.var_type == COMPLEX_VAR {
            meta.num_vectors
                + usize::try_from(meta.num_variables - 1).map_err(|_| {
                    WaveformError::FormatError(format!(
                        "invalid complex variable count: {}",
                        meta.num_variables
                    ))
                })?
        } else {
            meta.num_vectors
        };
        let capacity =
            estimate_table_rows(data.len(), meta.post_version, num_columns, remaining_sweeps);
        let num_complex_signals = num_columns - meta.num_vectors;
        let mut table_builder = DataTableBuilder::new(
            meta.post_version,
            meta.num_vectors,
            num_complex_signals,
            meta.sweep_name.is_some(),
            capacity,
        );
        let mut block_reader = BlockReader::new(data, meta.post_version);

        while let Some(block) = block_reader.next_raw_block()? {
            table_builder.push_raw_block(block.bytes, block.endian, block.is_end);
            if block.is_end {
                break;
            }
        }

        let trailing_values = table_builder.trailing_value_count();
        (
            table_builder.finish(),
            block_reader.bytes_consumed(),
            block_reader.block_count(),
            block_reader.format_name(),
            trailing_values,
        )
    };

    reader.advance(consumed)?;

    debug!(
        blocks,
        format,
        points = table.len(),
        trailing_values,
        "Read data table"
    );

    Ok(table)
}

// ============================================================================
// Main entry point
// ============================================================================

/// Validate file format before parsing
fn validate_file_format(data: &[u8]) -> Result<()> {
    if data.is_empty() {
        return Err(WaveformError::FormatError("File is empty".into()));
    }
    if data[0] >= b' ' {
        return Err(WaveformError::FormatError(
            "File is ASCII format, only binary supported".into(),
        ));
    }
    Ok(())
}

/// Parse only the header, return metadata and data start position
pub fn parse_header_only(mmap: &Mmap) -> Result<(HeaderMetadata, usize)> {
    parse_header_from_bytes(mmap)
}

fn parse_header_from_bytes(data: &[u8]) -> Result<(HeaderMetadata, usize)> {
    validate_file_format(data)?;
    let mut reader = MmapReader::new(data);
    let header_buf = read_header_blocks(&mut reader)?;
    let metadata = parse_header_metadata(&header_buf)?;

    let data_position = data.len() - reader.remaining();
    Ok((metadata, data_position))
}

/// Infer analysis type from filename
fn infer_analysis_type(filename: &str) -> AnalysisType {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(AnalysisType::from_extension)
        .unwrap_or(AnalysisType::Unknown)
}

/// Main HSPICE file reader - returns WaveformResult
#[instrument(skip_all, fields(file = %filename))]
pub fn hspice_read_impl(filename: &str) -> Result<WaveformResult> {
    info!("Reading HSPICE file");

    let file = File::open(filename)?;
    // SAFETY: the returned read-only mapping owns its OS mapping and is not
    // mutated through this process while parsing.
    let mmap = unsafe { Mmap::map(&file)? };
    #[cfg(unix)]
    if let Err(error) = mmap.advise(memmap2::Advice::Sequential) {
        trace!(%error, "Could not set sequential mapping advice");
    }

    let file_size = mmap.len();
    let file_size_mb = file_size as f64 / 1_048_576.0;
    debug!(size_bytes = file_size, size_mb = %format!("{:.2}", file_size_mb), "File mapped");

    parse_hspice_bytes(&mmap, filename)
}

/// Parses HSPICE bytes using a filename only as an analysis-type hint.
pub(crate) fn hspice_read_bytes_impl(data: &[u8], filename_hint: &str) -> Result<WaveformResult> {
    info!("Reading HSPICE data from memory");
    parse_hspice_bytes(data, filename_hint)
}

fn parse_hspice_bytes(data: &[u8], filename_hint: &str) -> Result<WaveformResult> {
    validate_file_format(data)?;

    let mut reader = MmapReader::new(data);
    let header_buf = read_header_blocks(&mut reader)?;
    let meta = parse_header_metadata(&header_buf)?;

    info!(
        version = ?meta.post_version,
        vectors = meta.num_vectors,
        scale = %meta.scale_name,
        "Header parsed"
    );

    if let Some(ref name) = meta.sweep_name {
        info!(sweep_param = %name, sweep_points = meta.sweep_size, "Sweep detected");
    }

    // Infer analysis type
    let analysis = if meta.var_type == COMPLEX_VAR {
        AnalysisType::AC
    } else {
        let from_scale = AnalysisType::from_scale_name(&meta.scale_name);
        if from_scale != AnalysisType::Unknown {
            from_scale
        } else {
            infer_analysis_type(filename_hint)
        }
    };
    debug!(analysis = %analysis, "Analysis type inferred");

    // Build variable list
    let mut variables = Vec::with_capacity(meta.num_vectors);
    variables.push(Variable::new(&meta.scale_name));
    for name in &meta.names {
        variables.push(Variable::new(name));
    }
    trace!(count = variables.len(), "Variables built");

    // Read data tables
    let sweep_count = usize::try_from(meta.sweep_size).map_err(|_| {
        WaveformError::FormatError(format!("invalid sweep count: {}", meta.sweep_size))
    })?;
    let mut tables = Vec::with_capacity(sweep_count);

    for sweep_idx in 0..sweep_count {
        trace!(sweep = sweep_idx + 1, total = sweep_count, "Reading sweep");

        let remaining_sweeps = sweep_count - sweep_idx;
        tables.push(read_data_table(&mut reader, &meta, remaining_sweeps)?);
    }

    info!(
        tables = tables.len(),
        points = tables.first().map_or(0, DataTable::len),
        "Parsing complete"
    );

    Ok(WaveformResult {
        title: meta.title,
        date: meta.date,
        analysis,
        variables,
        sweep_param: meta.sweep_name,
        tables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{VarType, VectorData, END_MARKER_9601};

    fn append_f32_block(bytes: &mut Vec<u8>, values: &[f32]) {
        let byte_count = (values.len() * 4) as i32;
        for value in [4_i32, byte_count, 4_i32, byte_count] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&byte_count.to_le_bytes());
    }

    fn real_sweep_metadata() -> HeaderMetadata {
        HeaderMetadata {
            title: String::new(),
            date: String::new(),
            post_version: PostVersion::V9601,
            num_variables: 2,
            num_vectors: 3,
            var_type: REAL_VAR,
            scale_name: "TIME".into(),
            names: vec!["a".into(), "b".into()],
            sweep_name: Some("corner".into()),
            sweep_size: 2,
        }
    }

    #[test]
    fn parse_vector_names_preserves_voltage_descriptor_exactly() {
        let mut header = vec![b' '; VECTOR_DESCRIPTION_START_POSITION];
        header.extend_from_slice(b"1 1 TIME V(Out");

        let (_, names) = parse_vector_names(&header, 2).unwrap();

        assert_eq!(names, ["V(Out"]);
    }

    #[test]
    fn parse_vector_names_preserves_current_descriptor_exactly() {
        let mut header = vec![b' '; VECTOR_DESCRIPTION_START_POSITION];
        header.extend_from_slice(b"1 1 TIME i(VDD");

        let (_, names) = parse_vector_names(&header, 2).unwrap();

        assert_eq!(names, ["i(VDD"]);
    }

    #[test]
    fn preserved_voltage_descriptor_infers_voltage_type() {
        let mut header = vec![b' '; VECTOR_DESCRIPTION_START_POSITION];
        header.extend_from_slice(b"1 1 TIME V(Out");
        let (_, names) = parse_vector_names(&header, 2).unwrap();

        let variable = Variable::new(&names[0]);

        assert_eq!(variable.var_type, VarType::Voltage);
    }

    #[test]
    fn reads_multiple_sweep_tables_without_consuming_following_table() {
        let mut bytes = Vec::new();
        append_f32_block(
            &mut bytes,
            &[1.0, 0.0, 10.0, 20.0, 1.0, 11.0, 21.0, END_MARKER_9601],
        );
        append_f32_block(
            &mut bytes,
            &[2.0, 2.0, 12.0, 22.0, 3.0, 13.0, 23.0, END_MARKER_9601],
        );

        let meta = real_sweep_metadata();
        let mut reader = MmapReader::new(&bytes);
        let first = read_data_table(&mut reader, &meta, 2).unwrap();
        let second = read_data_table(&mut reader, &meta, 1).unwrap();

        assert_eq!(first.sweep_value, Some(1.0));
        assert_eq!(second.sweep_value, Some(2.0));
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        assert_eq!(reader.remaining(), 0);

        let VectorData::Real(first_time) = &first.vectors[0] else {
            panic!("expected real scale");
        };
        let VectorData::Real(second_b) = &second.vectors[2] else {
            panic!("expected real signal");
        };
        assert_eq!(first_time, &[0.0, 1.0]);
        assert_eq!(second_b, &[22.0, 23.0]);
    }
}
