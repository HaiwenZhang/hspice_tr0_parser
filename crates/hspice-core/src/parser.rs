//! HSPICE binary file parser

use crate::reader::MmapReader;
use crate::types::*;
use memmap2::Mmap;
use num_complex::Complex64;
use std::fs::File;
use std::path::Path;

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
                reader.read_floats_as_f64_into(num_items, &mut raw_data)?;
                raw_data
                    .last()
                    .map(|&v| v as f32 >= END_MARKER_9601)
                    .unwrap_or(false)
            }
            PostVersion::V2001 => {
                reader.read_doubles_into(num_items, &mut raw_data)?;
                raw_data
                    .last()
                    .map(|&v| v >= END_MARKER_2001)
                    .unwrap_or(false)
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
            "Read {} data blocks ({}), {} total values",
            num_blocks,
            format_name,
            raw_data.len()
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
    );
    if !(0..=1).contains(&num_sweeps) {
        return Err(WaveformError::FormatError(
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

    let desc_section = &header_buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);
    let tokens: Vec<&str> = desc_str.split_whitespace().collect();
    let var_type_num: i32 = tokens.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let var_type = if var_type_num == FREQUENCY_TYPE {
        COMPLEX_VAR
    } else {
        REAL_VAR
    };

    let (scale_name, names) = parse_vector_names(header_buf, num_vectors)?;

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
// Data processing
// ============================================================================

/// Process raw data into vectors
fn process_raw_data(
    raw_data: &[f64],
    num_vectors: usize,
    num_variables: i32,
    var_type: i32,
    has_sweep: bool,
) -> (Option<f64>, Vec<VectorData>) {
    let num_columns = if var_type == COMPLEX_VAR {
        num_vectors + (num_variables - 1) as usize
    } else {
        num_vectors
    };

    let data_offset = if has_sweep { 2 } else { 1 };
    let num_rows = (raw_data.len().saturating_sub(data_offset)) / num_columns.max(1);
    let data_start = if has_sweep { 1 } else { 0 };
    let sweep_value = if has_sweep { Some(raw_data[0]) } else { None };

    // Pre-allocate all vectors
    let mut vectors: Vec<VectorData> = Vec::with_capacity(num_vectors);

    // Scale (always real, first column)
    let mut scale_vec = Vec::with_capacity(num_rows);

    // Signal vectors
    let mut signal_bufs: Vec<SignalBuffer> = (0..num_vectors - 1)
        .map(|i| {
            let is_complex = var_type == COMPLEX_VAR && i < (num_variables - 1) as usize;
            if is_complex {
                SignalBuffer::Complex(Vec::with_capacity(num_rows))
            } else {
                SignalBuffer::Real(Vec::with_capacity(num_rows))
            }
        })
        .collect();

    // Single pass through raw data
    let mut pos = data_start;
    for _ in 0..num_rows {
        // First column is always scale (real)
        scale_vec.push(raw_data[pos]);
        pos += 1;

        // Remaining columns
        for (i, buf) in signal_bufs.iter_mut().enumerate() {
            let is_complex_col = var_type == COMPLEX_VAR && i < (num_variables - 1) as usize;
            match buf {
                SignalBuffer::Complex(vec) if is_complex_col => {
                    let real = raw_data[pos];
                    let imag = raw_data[pos + 1];
                    vec.push(Complex64::new(real, imag));
                    pos += 2;
                }
                SignalBuffer::Real(vec) => {
                    vec.push(raw_data[pos]);
                    pos += 1;
                }
                _ => {
                    pos += 1;
                }
            }
        }
    }

    // Build vectors in order
    vectors.push(VectorData::Real(scale_vec));
    for buf in signal_bufs {
        let vector_data = match buf {
            SignalBuffer::Real(vec) => VectorData::Real(vec),
            SignalBuffer::Complex(vec) => VectorData::Complex(vec),
        };
        vectors.push(vector_data);
    }

    (sweep_value, vectors)
}

/// Internal buffer type
enum SignalBuffer {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}

// ============================================================================
// Main entry point
// ============================================================================

/// Validate file format before parsing
fn validate_file_format(mmap: &Mmap) -> Result<()> {
    if mmap.is_empty() {
        return Err(WaveformError::FormatError("File is empty".into()));
    }
    if mmap[0] >= b' ' {
        return Err(WaveformError::FormatError(
            "File is ASCII format, only binary supported".into(),
        ));
    }
    Ok(())
}

/// Parse only the header, return metadata and data start position
pub fn parse_header_only(mmap: &Mmap) -> Result<(HeaderMetadata, usize)> {
    validate_file_format(mmap)?;

    let mut reader = MmapReader::new(mmap);
    let header_buf = read_header_blocks(&mut reader)?;
    let metadata = parse_header_metadata(&header_buf)?;

    let data_position = mmap.len() - reader.remaining();
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
pub fn hspice_read_impl(filename: &str, debug: i32) -> Result<WaveformResult> {
    if debug > 0 {
        eprintln!("Reading: {}", filename);
    }

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
    let meta = parse_header_metadata(&header_buf)?;

    if debug > 0 {
        eprintln!("Post version: {:?}", meta.post_version);
        eprintln!("Vectors: {}", meta.num_vectors);
        eprintln!("Scale: {}", meta.scale_name);
        if let Some(ref name) = meta.sweep_name {
            eprintln!("Sweep: {} ({} points)", name, meta.sweep_size);
        }
    }

    // Infer analysis type
    let analysis = if meta.var_type == COMPLEX_VAR {
        AnalysisType::AC
    } else {
        let from_scale = AnalysisType::from_scale_name(&meta.scale_name);
        if from_scale != AnalysisType::Unknown {
            from_scale
        } else {
            infer_analysis_type(filename)
        }
    };

    // Build variable list
    let mut variables = Vec::with_capacity(meta.num_vectors);
    variables.push(Variable::new(&meta.scale_name));
    for name in &meta.names {
        variables.push(Variable::new(name));
    }

    // Read data tables
    let mut tables = Vec::with_capacity(meta.sweep_size as usize);

    for sweep_idx in 0..meta.sweep_size {
        if debug > 1 {
            eprintln!("Reading sweep {}/{}", sweep_idx + 1, meta.sweep_size);
        }

        let raw_data = read_data_blocks(&mut reader, meta.post_version, debug > 1)?;
        let (sweep_value, vectors) = process_raw_data(
            &raw_data,
            meta.num_vectors,
            meta.num_variables,
            meta.var_type,
            meta.sweep_name.is_some(),
        );

        tables.push(DataTable {
            sweep_value,
            vectors,
        });
    }

    Ok(WaveformResult {
        title: meta.title,
        date: meta.date,
        analysis,
        variables,
        sweep_param: meta.sweep_name,
        tables,
    })
}
