//! HSPICE binary file parser

use crate::reader::MmapReader;
use crate::types::*;
use memmap2::Mmap;
use numpy::Complex64;
use std::collections::HashMap;
use std::fs::File;

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

/// Read data blocks until end marker found - unified for all formats
fn read_data_blocks(
    reader: &mut MmapReader,
    version: PostVersion,
    debug: bool,
) -> Result<Vec<f64>> {
    let (item_size, divisor) = match version {
        PostVersion::V9601 => (4usize, 5),
        PostVersion::V2001 => (8usize, 9),
    };

    let estimated_items = reader.remaining() / divisor;
    let mut raw_data = Vec::with_capacity(estimated_items);
    let mut num_blocks = 0usize;

    loop {
        let (num_items, trailer) = reader.read_block_header(item_size)?;
        num_blocks += 1;

        let is_end = match version {
            PostVersion::V9601 => {
                let block_floats = reader.read_floats_bulk(num_items)?;
                let end = block_floats
                    .last()
                    .map(|&v| v >= END_MARKER_9601)
                    .unwrap_or(false);
                raw_data.extend(block_floats.into_iter().map(|v| v as f64));
                end
            }
            PostVersion::V2001 => {
                let block_doubles = reader.read_doubles_bulk(num_items)?;
                let end = block_doubles
                    .last()
                    .map(|&v| v >= END_MARKER_2001)
                    .unwrap_or(false);
                raw_data.extend(block_doubles);
                end
            }
        };

        reader.read_block_trailer(trailer)?;

        if is_end {
            break;
        }
    }

    if debug {
        let format_name = match version {
            PostVersion::V9601 => "f32",
            PostVersion::V2001 => "f64",
        };
        eprintln!(
            "Read {} data blocks ({}), {} total values (capacity: {})",
            num_blocks,
            format_name,
            raw_data.len(),
            raw_data.capacity()
        );
    }

    Ok(raw_data)
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
fn extract_int(buf: &[u8], start: usize, end: usize) -> i32 {
    extract_string(buf, start, end).trim().parse().unwrap_or(0)
}

// ============================================================================
// Header parsing - split from hspice_read_impl
// ============================================================================

/// Parsed header metadata
struct HeaderMetadata {
    title: String,
    date: String,
    post_version: PostVersion,
    num_variables: i32,
    num_vectors: usize,
    var_type: i32,
    scale_name: String,
    names: Vec<String>,
    sweep_name: Option<String>,
    sweep_size: i32,
}

/// Parse vector names from header buffer
fn parse_vector_names(buf: &[u8], num_vectors: usize) -> Result<(String, Vec<String>)> {
    if buf.len() < VECTOR_DESCRIPTION_START_POSITION {
        return Err(HspiceError::ParseError("Buffer too short".into()));
    }

    let desc_section = &buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);
    let tokens: Vec<&str> = desc_str.split_whitespace().collect();

    if tokens.len() < num_vectors + 1 {
        return Err(HspiceError::ParseError("Not enough vector names".into()));
    }

    let scale_name = tokens.get(num_vectors).unwrap_or(&"time").to_string();

    let names: Vec<String> = ((num_vectors + 1)..(2 * num_vectors))
        .filter_map(|i| tokens.get(i))
        .map(|name| {
            let mut name = name.to_lowercase();
            if name.starts_with("v(") {
                name = name[2..].trim_end_matches(')').to_string();
            }
            name
        })
        .collect();

    Ok((scale_name, names))
}

/// Get sweep info from header tokens
fn get_sweep_info(buf: &[u8], tokens: &[&str], num_vectors: usize) -> Option<(String, i32)> {
    let sweep_name = tokens.get(2 * num_vectors)?.to_string();
    let post_str = extract_string(buf, POST_START_POSITION2, POST_START_POSITION2 + 4);
    let sweep_size = if post_str == POST_STRING21 {
        extract_int(buf, SWEEP_SIZE_POSITION2, SWEEP_SIZE_POSITION2 + 10)
    } else {
        extract_int(buf, SWEEP_SIZE_POSITION1, SWEEP_SIZE_POSITION1 + 10)
    };
    Some((sweep_name, sweep_size))
}

/// Parse all header metadata from buffer
fn parse_header_metadata(header_buf: &[u8]) -> Result<HeaderMetadata> {
    // Check post format version
    let post1 = extract_string(header_buf, POST_START_POSITION1, POST_START_POSITION1 + 4);
    let post2 = extract_string(header_buf, POST_START_POSITION2, POST_START_POSITION2 + 4);

    if post1 != POST_STRING11 && post1 != POST_STRING12 && post2 != POST_STRING21 {
        return Err(HspiceError::FormatError("Unknown post format".into()));
    }

    let post_version = if post2 == POST_STRING21 {
        PostVersion::V2001
    } else {
        PostVersion::V9601
    };

    // Extract title and date
    let date = extract_string(header_buf, DATE_START_POSITION, DATE_END_POSITION);
    let title_end = {
        let mut end = DATE_START_POSITION;
        while end > TITLE_START_POSITION && header_buf.get(end - 1) == Some(&b' ') {
            end -= 1;
        }
        end
    };
    let title = extract_string(header_buf, TITLE_START_POSITION, title_end);

    // Get counts
    let num_sweeps = extract_int(
        header_buf,
        NUM_OF_SWEEPS_POSITION,
        NUM_OF_SWEEPS_END_POSITION,
    );
    if num_sweeps < 0 || num_sweeps > 1 {
        return Err(HspiceError::FormatError(
            "Only one-dimensional sweep supported".into(),
        ));
    }

    let num_probes = extract_int(header_buf, NUM_OF_PROBES_POSITION, NUM_OF_SWEEPS_POSITION);
    let num_variables = extract_int(
        header_buf,
        NUM_OF_VARIABLES_POSITION,
        NUM_OF_PROBES_POSITION,
    );
    let num_vectors = (num_probes + num_variables) as usize;

    // Get variable type
    let desc_section = &header_buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);
    let tokens: Vec<&str> = desc_str.split_whitespace().collect();
    let var_type_num: i32 = tokens.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let var_type = if var_type_num == FREQUENCY_TYPE {
        COMPLEX_VAR
    } else {
        REAL_VAR
    };

    // Parse vector names
    let (scale_name, names) = parse_vector_names(header_buf, num_vectors)?;

    // Get sweep info
    let (sweep_name, sweep_size) = if num_sweeps == 1 {
        get_sweep_info(header_buf, &tokens, num_vectors)
            .map(|(n, s)| (Some(n), s.max(1)))
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

// ============================================================================
// Data processing - split from process_raw_data
// ============================================================================

/// Data layout information
#[allow(dead_code)]
struct DataLayout {
    num_columns: usize,
    num_rows: usize,
    data_start: usize,
    sweep_value: Option<f64>,
}

/// Calculate data layout from raw data
#[inline]
fn calculate_data_layout(
    raw_data: &[f64],
    num_vectors: usize,
    num_variables: i32,
    var_type: i32,
    has_sweep: bool,
) -> DataLayout {
    let num_columns = if var_type == COMPLEX_VAR {
        num_vectors + (num_variables - 1) as usize
    } else {
        num_vectors
    };

    let data_offset = if has_sweep { 2 } else { 1 };
    let num_rows = (raw_data.len() - data_offset) / num_columns;
    let data_start = if has_sweep { 1 } else { 0 };
    let sweep_value = if has_sweep { Some(raw_data[0]) } else { None };

    DataLayout {
        num_columns,
        num_rows,
        data_start,
        sweep_value,
    }
}

/// Extracted column data
struct ColumnData {
    real_columns: Vec<Vec<f64>>,
    complex_columns: Vec<Vec<Complex64>>,
}

/// Extract columns from raw data
fn extract_columns(
    raw_data: &[f64],
    layout: &DataLayout,
    num_vectors: usize,
    num_variables: i32,
    var_type: i32,
) -> ColumnData {
    let mut real_columns: Vec<Vec<f64>> = (0..num_vectors)
        .map(|_| Vec::with_capacity(layout.num_rows))
        .collect();

    let mut complex_columns: Vec<Vec<Complex64>> = (0..(num_variables - 1).max(0) as usize)
        .map(|_| Vec::with_capacity(layout.num_rows))
        .collect();

    let mut pos = layout.data_start;
    for _ in 0..layout.num_rows {
        for col in 0..num_vectors {
            let is_complex_col = var_type == COMPLEX_VAR && col > 0 && col < num_variables as usize;
            if is_complex_col {
                let real = raw_data[pos];
                let imag = raw_data[pos + 1];
                complex_columns[col - 1].push(Complex64::new(real, imag));
                pos += 2;
            } else {
                real_columns[col].push(raw_data[pos]);
                pos += 1;
            }
        }
    }

    ColumnData {
        real_columns,
        complex_columns,
    }
}

/// Build result HashMap from extracted columns
fn build_result_table(
    mut columns: ColumnData,
    names: &[String],
    scale_name: &str,
    num_variables: i32,
    var_type: i32,
) -> HashMap<String, VectorData> {
    let mut table = HashMap::with_capacity(names.len() + 1);

    // Scale is always first column
    table.insert(
        scale_name.to_string(),
        VectorData::Real(std::mem::take(&mut columns.real_columns[0])),
    );

    // Other variables
    for (i, name) in names.iter().enumerate() {
        let is_complex = var_type == COMPLEX_VAR && i < (num_variables - 1) as usize;
        if is_complex {
            table.insert(
                name.clone(),
                VectorData::Complex(std::mem::take(&mut columns.complex_columns[i])),
            );
        } else {
            let col_idx = i + 1;
            if col_idx < columns.real_columns.len() {
                table.insert(
                    name.clone(),
                    VectorData::Real(std::mem::take(&mut columns.real_columns[col_idx])),
                );
            }
        }
    }

    table
}

/// Process raw data into result table - orchestrates the pipeline
fn process_raw_data(
    raw_data: &[f64],
    num_vectors: usize,
    num_variables: i32,
    var_type: i32,
    has_sweep: bool,
    names: &[String],
    scale_name: &str,
) -> (Option<f64>, HashMap<String, VectorData>) {
    let layout = calculate_data_layout(raw_data, num_vectors, num_variables, var_type, has_sweep);
    let columns = extract_columns(raw_data, &layout, num_vectors, num_variables, var_type);
    let table = build_result_table(columns, names, scale_name, num_variables, var_type);
    (layout.sweep_value, table)
}

// ============================================================================
// Main entry point
// ============================================================================

/// Validate file format before parsing
fn validate_file_format(mmap: &Mmap) -> Result<()> {
    if mmap.is_empty() {
        return Err(HspiceError::FormatError("File is empty".into()));
    }
    if mmap[0] >= b' ' {
        return Err(HspiceError::FormatError(
            "File is ASCII format, only binary supported".into(),
        ));
    }
    Ok(())
}

/// Main HSPICE file reader
pub fn hspice_read_impl(filename: &str, debug: i32) -> Result<HspiceResult> {
    if debug > 0 {
        eprintln!("HSpiceRead: reading file {}", filename);
    }

    // Memory-map the file
    let file = File::open(filename)?;
    let mmap = unsafe { Mmap::map(&file)? };

    if debug > 0 {
        eprintln!(
            "File size: {} bytes ({:.2} MB)",
            mmap.len(),
            mmap.len() as f64 / 1_048_576.0
        );
    }

    validate_file_format(&mmap)?;

    let mut reader = MmapReader::new(&mmap);
    let header_buf = read_header_blocks(&mut reader)?;

    if debug > 1 {
        eprintln!("Header buffer size: {} bytes", header_buf.len());
    }

    let meta = parse_header_metadata(&header_buf)?;

    if debug > 0 {
        eprintln!("Post version: {:?}", meta.post_version);
        eprintln!(
            "Variables: {}, Vectors: {}",
            meta.num_variables, meta.num_vectors
        );
        eprintln!("Scale: {}", meta.scale_name);
        if let Some(ref name) = meta.sweep_name {
            eprintln!("Sweep: {} with {} points", name, meta.sweep_size);
        }
    }

    // Read and process data tables
    let mut data_tables = Vec::with_capacity(meta.sweep_size as usize);
    let mut sweep_values = if meta.sweep_name.is_some() {
        Vec::with_capacity(meta.sweep_size as usize)
    } else {
        Vec::new()
    };

    for sweep_idx in 0..meta.sweep_size {
        if debug > 1 {
            eprintln!("Reading sweep point {}/{}", sweep_idx + 1, meta.sweep_size);
        }

        let raw_data = read_data_blocks(&mut reader, meta.post_version, debug > 1)?;
        let (sweep_val, table) = process_raw_data(
            &raw_data,
            meta.num_vectors,
            meta.num_variables,
            meta.var_type,
            meta.sweep_name.is_some(),
            &meta.names,
            &meta.scale_name,
        );

        if let Some(val) = sweep_val {
            sweep_values.push(val);
        }
        data_tables.push(table);
    }

    Ok(HspiceResult {
        sweep_name: meta.sweep_name,
        sweep_values: if sweep_values.is_empty() {
            None
        } else {
            Some(sweep_values)
        },
        data_tables,
        scale_name: meta.scale_name,
        title: meta.title,
        date: meta.date,
    })
}
