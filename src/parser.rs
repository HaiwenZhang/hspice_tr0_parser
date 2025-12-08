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
    let mut buffer = Vec::with_capacity(4096); // Pre-allocate reasonable header size

    loop {
        let (num_items, trailer) = reader.read_block_header(1)?;
        let block_data = reader.read_bytes(num_items)?;
        reader.read_block_trailer(trailer)?;

        buffer.extend_from_slice(block_data);

        // Check for end-of-block marker
        if let Some(pos) = find_subsequence(&buffer, b"$&%#") {
            buffer.truncate(pos);
            break;
        }
    }

    Ok(buffer)
}

/// Read data blocks for 9601 format (float32) - optimized single-pass with bulk reading
fn read_data_blocks_f32(reader: &mut MmapReader, debug: bool) -> Result<Vec<f64>> {
    // Estimate capacity based on remaining file size
    let estimated_floats = reader.remaining() / 5; // Rough estimate accounting for headers
    let mut raw_data = Vec::with_capacity(estimated_floats);
    let mut num_blocks = 0usize;

    loop {
        let (num_items, trailer) = reader.read_block_header(4)?;
        num_blocks += 1;

        // Bulk read all floats in this block
        let block_floats = reader.read_floats_bulk(num_items)?;

        // Check last value for end marker (9601 format: ~1e30)
        let is_end = block_floats
            .last()
            .map(|&v| v >= END_MARKER_9601)
            .unwrap_or(false);

        // Convert f32 to f64 for unified processing
        raw_data.extend(block_floats.into_iter().map(|v| v as f64));
        reader.read_block_trailer(trailer)?;

        if is_end {
            break;
        }
    }

    if debug {
        eprintln!(
            "Read {} data blocks (f32), {} total values (capacity: {})",
            num_blocks,
            raw_data.len(),
            raw_data.capacity()
        );
    }

    Ok(raw_data)
}

/// Read data blocks for 2001 format (float64/double) - optimized single-pass with bulk reading
fn read_data_blocks_f64(reader: &mut MmapReader, debug: bool) -> Result<Vec<f64>> {
    // Estimate capacity based on remaining file size
    let estimated_doubles = reader.remaining() / 9; // Rough estimate accounting for headers
    let mut raw_data = Vec::with_capacity(estimated_doubles);
    let mut num_blocks = 0usize;

    loop {
        let (num_items, trailer) = reader.read_block_header(8)?;
        num_blocks += 1;

        // Bulk read all doubles in this block
        let block_doubles = reader.read_doubles_bulk(num_items)?;

        // Check last value for end marker (2001 format: 1e30)
        let is_end = block_doubles
            .last()
            .map(|&v| v >= END_MARKER_2001)
            .unwrap_or(false);

        raw_data.extend(block_doubles);
        reader.read_block_trailer(trailer)?;

        if is_end {
            break;
        }
    }

    if debug {
        eprintln!(
            "Read {} data blocks (f64), {} total values (capacity: {})",
            num_blocks,
            raw_data.len(),
            raw_data.capacity()
        );
    }

    Ok(raw_data)
}

/// Read data blocks - dispatches to format-specific reader based on post version
fn read_data_blocks(
    reader: &mut MmapReader,
    version: PostVersion,
    debug: bool,
) -> Result<Vec<f64>> {
    match version {
        PostVersion::V9601 => read_data_blocks_f32(reader, debug),
        PostVersion::V2001 => read_data_blocks_f64(reader, debug),
    }
}

/// Extract string from buffer at given range, trimmed
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

/// Extract integer from string in buffer
#[inline]
fn extract_int(buf: &[u8], start: usize, end: usize) -> i32 {
    let s = extract_string(buf, start, end);
    s.trim().parse().unwrap_or(0)
}

/// Parse vector names from header buffer
fn parse_vector_names(buf: &[u8], num_vectors: usize) -> Result<(String, Vec<String>)> {
    if buf.len() < VECTOR_DESCRIPTION_START_POSITION {
        return Err(HspiceError::ParseError(
            "Buffer too short for vector descriptions".into(),
        ));
    }

    let desc_section = &buf[VECTOR_DESCRIPTION_START_POSITION..];
    let desc_str = String::from_utf8_lossy(desc_section);

    let tokens: Vec<&str> = desc_str.split_whitespace().collect();

    if tokens.len() < num_vectors + 1 {
        return Err(HspiceError::ParseError(
            "Not enough vector names in header".into(),
        ));
    }

    let scale_name = tokens.get(num_vectors).unwrap_or(&"time").to_string();

    let mut names = Vec::with_capacity(num_vectors - 1);
    for i in (num_vectors + 1)..(2 * num_vectors) {
        if let Some(name) = tokens.get(i) {
            let mut name = name.to_lowercase();
            if name.starts_with("v(") {
                name = name[2..].trim_end_matches(')').to_string();
            }
            names.push(name);
        }
    }

    Ok((scale_name, names))
}

/// Get sweep info from header
fn get_sweep_info(buf: &[u8], tokens: &[&str], num_vectors: usize) -> Option<(String, i32)> {
    let sweep_name_pos = 2 * num_vectors;
    let sweep_name = tokens.get(sweep_name_pos)?.to_string();

    let post_str = extract_string(buf, POST_START_POSITION2, POST_START_POSITION2 + 4);
    let sweep_size = if post_str == POST_STRING21 {
        extract_int(buf, SWEEP_SIZE_POSITION2, SWEEP_SIZE_POSITION2 + 10)
    } else {
        extract_int(buf, SWEEP_SIZE_POSITION1, SWEEP_SIZE_POSITION1 + 10)
    };

    Some((sweep_name, sweep_size))
}

/// Process raw data into column vectors
fn process_raw_data(
    raw_data: &[f64],
    num_vectors: usize,
    num_variables: i32,
    var_type: i32,
    has_sweep: bool,
    names: &[String],
    scale_name: &str,
) -> (Option<f64>, HashMap<String, VectorData>) {
    let mut num_columns = num_vectors;
    if var_type == COMPLEX_VAR {
        num_columns += (num_variables - 1) as usize;
    }

    let data_offset = if has_sweep { 2 } else { 1 };
    let num_rows = (raw_data.len() - data_offset) / num_columns;

    let sweep_value = if has_sweep { Some(raw_data[0]) } else { None };

    let data_start = if has_sweep { 1 } else { 0 };

    // Pre-allocate all column vectors
    let mut column_data: Vec<Vec<f64>> = (0..num_vectors)
        .map(|_| Vec::with_capacity(num_rows))
        .collect();

    let mut complex_data: Vec<Vec<Complex64>> = (0..(num_variables - 1).max(0) as usize)
        .map(|_| Vec::with_capacity(num_rows))
        .collect();

    // Process rows
    let mut data_pos = data_start;
    for _row in 0..num_rows {
        for col in 0..num_vectors {
            if var_type == COMPLEX_VAR && col > 0 && col < num_variables as usize {
                let real = raw_data[data_pos];
                data_pos += 1;
                let imag = raw_data[data_pos];
                data_pos += 1;
                complex_data[col - 1].push(Complex64::new(real, imag));
            } else {
                column_data[col].push(raw_data[data_pos]);
                data_pos += 1;
            }
        }
    }

    // Build result HashMap
    let mut table = HashMap::with_capacity(num_vectors);

    // Scale is always first column
    table.insert(
        scale_name.to_string(),
        VectorData::Real(std::mem::take(&mut column_data[0])),
    );

    // Other variables
    for (i, name) in names.iter().enumerate() {
        if var_type == COMPLEX_VAR && i < (num_variables - 1) as usize {
            table.insert(
                name.clone(),
                VectorData::Complex(std::mem::take(&mut complex_data[i])),
            );
        } else {
            let col_idx = if var_type == COMPLEX_VAR {
                1 + i - (num_variables - 1) as usize + (num_variables - 1) as usize
            } else {
                i + 1
            };
            if col_idx < column_data.len() {
                table.insert(
                    name.clone(),
                    VectorData::Real(std::mem::take(&mut column_data[col_idx])),
                );
            }
        }
    }

    (sweep_value, table)
}

/// Main HSPICE file reader - optimized version
pub fn hspice_read_impl(filename: &str, debug: i32) -> Result<HspiceResult> {
    if debug > 0 {
        eprintln!("HSpiceRead: reading file {}", filename);
    }

    // Memory-map the file for efficient access
    let file = File::open(filename)?;
    let mmap = unsafe { Mmap::map(&file)? };

    if debug > 0 {
        eprintln!(
            "File size: {} bytes ({:.2} MB)",
            mmap.len(),
            mmap.len() as f64 / 1_048_576.0
        );
    }

    let mut reader = MmapReader::new(&mmap);

    // Check if file has data and is binary format
    if mmap.is_empty() {
        return Err(HspiceError::FormatError("File is empty".into()));
    }

    if mmap[0] >= b' ' {
        return Err(HspiceError::FormatError(
            "File is in ASCII format, only binary supported".into(),
        ));
    }

    // Read header blocks
    let header_buf = read_header_blocks(&mut reader)?;

    if debug > 1 {
        eprintln!("Header buffer size: {} bytes", header_buf.len());
    }

    // Check post format version
    let post1 = extract_string(&header_buf, POST_START_POSITION1, POST_START_POSITION1 + 4);
    let post2 = extract_string(&header_buf, POST_START_POSITION2, POST_START_POSITION2 + 4);

    if post1 != POST_STRING11 && post1 != POST_STRING12 && post2 != POST_STRING21 {
        return Err(HspiceError::FormatError("Unknown post format".into()));
    }

    // Determine post version for data format selection
    let post_version = if post2 == POST_STRING21 {
        PostVersion::V2001 // 8-byte double precision
    } else {
        PostVersion::V9601 // 4-byte float
    };

    if debug > 0 {
        eprintln!("Post version: {:?}", post_version);
    }

    // Extract metadata
    let date = extract_string(&header_buf, DATE_START_POSITION, DATE_END_POSITION);

    let title_end = {
        let mut end = DATE_START_POSITION;
        while end > TITLE_START_POSITION && header_buf.get(end - 1) == Some(&b' ') {
            end -= 1;
        }
        end
    };
    let title = extract_string(&header_buf, TITLE_START_POSITION, title_end);

    // Get number of sweeps
    let num_sweeps = extract_int(
        &header_buf,
        NUM_OF_SWEEPS_POSITION,
        NUM_OF_SWEEPS_END_POSITION,
    );
    if num_sweeps < 0 || num_sweeps > 1 {
        return Err(HspiceError::FormatError(
            "Only one-dimensional sweep supported".into(),
        ));
    }

    // Get number of variables and probes
    let num_probes = extract_int(&header_buf, NUM_OF_PROBES_POSITION, NUM_OF_SWEEPS_POSITION);
    let num_variables = extract_int(
        &header_buf,
        NUM_OF_VARIABLES_POSITION,
        NUM_OF_PROBES_POSITION,
    );
    let num_vectors = (num_probes + num_variables) as usize;

    if debug > 0 {
        eprintln!(
            "Variables: {}, Probes: {}, Total vectors: {}",
            num_variables, num_probes, num_vectors
        );
    }

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
    let (scale_name, names) = parse_vector_names(&header_buf, num_vectors)?;

    if debug > 0 {
        eprintln!("Scale: {}", scale_name);
    }

    // Get sweep info
    let (sweep_name, sweep_size) = if num_sweeps == 1 {
        match get_sweep_info(&header_buf, &tokens, num_vectors) {
            Some((name, size)) => (Some(name), size.max(1)),
            None => (None, 1),
        }
    } else {
        (None, 1)
    };

    let has_sweep = sweep_name.is_some();

    if debug > 0 && has_sweep {
        eprintln!(
            "Sweep: {} with {} points",
            sweep_name.as_ref().unwrap(),
            sweep_size
        );
    }

    // Read and process data tables
    let mut data_tables = Vec::with_capacity(sweep_size as usize);
    let mut sweep_values = if has_sweep {
        Vec::with_capacity(sweep_size as usize)
    } else {
        Vec::new()
    };

    for sweep_idx in 0..sweep_size {
        if debug > 1 {
            eprintln!("Reading sweep point {}/{}", sweep_idx + 1, sweep_size);
        }

        let raw_data = read_data_blocks(&mut reader, post_version, debug > 1)?;

        let (sweep_val, table) = process_raw_data(
            &raw_data,
            num_vectors,
            num_variables,
            var_type,
            has_sweep,
            &names,
            &scale_name,
        );

        if let Some(val) = sweep_val {
            sweep_values.push(val);
        }

        data_tables.push(table);
    }

    Ok(HspiceResult {
        sweep_name,
        sweep_values: if sweep_values.is_empty() {
            None
        } else {
            Some(sweep_values)
        },
        data_tables,
        scale_name,
        title,
        date,
    })
}
