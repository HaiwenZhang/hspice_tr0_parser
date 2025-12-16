//! True streaming reader for large HSPICE files
//!
//! This module provides memory-efficient streaming access to large TR0 files,
//! reading data blocks on-demand rather than loading the entire file upfront.
//!
//! Design principles:
//! - Only header is parsed at open() time (~1KB)
//! - Data blocks are read on-demand during iteration
//! - Block boundaries are preserved - never split a data_block in the middle of reading
//! - Incomplete rows at block boundaries are properly accumulated
//! - Peak memory is O(chunk_size * num_signals), not O(file_size)

use crate::parser::{parse_header_only, HeaderMetadata};
use crate::types::{PostVersion, Result, VectorData, COMPLEX_VAR};
use memmap2::Mmap;
use num_complex::Complex64;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use tracing::{info, instrument, trace};

/// Default chunk size (minimum number of time points per chunk)
pub const DEFAULT_CHUNK_SIZE: usize = 10000;

/// A chunk of data from the streaming reader
#[derive(Debug, Clone)]
pub struct DataChunk {
    /// Index of this chunk (0-based)
    pub chunk_index: usize,
    /// Time range [start, end] for this chunk
    pub time_range: (f64, f64),
    /// Signal data for this chunk
    pub data: HashMap<String, VectorData>,
}

/// Metadata about the streaming file
#[derive(Debug, Clone)]
pub struct StreamMetadata {
    /// File title
    pub title: String,
    /// File date
    pub date: String,
    /// Scale name (e.g., "TIME", "HERTZ")
    pub scale_name: String,
    /// All signal names in the file
    pub signal_names: Vec<String>,
    /// Post format version
    pub post_version: PostVersion,
    /// Whether file contains complex data
    pub is_complex: bool,
}

/// True streaming reader for HSPICE files
///
/// Only reads header at open() time. Data blocks are read on-demand.
/// Block boundaries are always preserved - we never split a data_block.
pub struct HspiceStreamReader {
    /// Memory-mapped file data
    mmap: Mmap,
    /// Current read position in the data section
    data_position: usize,
    /// Header metadata
    metadata: HeaderMetadata,
    /// Minimum rows per chunk (may exceed if block is larger)
    min_chunk_size: usize,
    /// Current chunk index
    current_chunk: usize,
    /// Signal filter (None = all signals)
    signal_filter: Option<HashSet<String>>,
    /// Whether we've reached end of data
    finished: bool,
    /// Accumulated rows for current chunk
    row_buffer: Vec<Vec<f64>>,
    /// Pending data from incomplete row at block boundary
    pending_data: Vec<f64>,
    /// Number of columns per row (computed once)
    num_columns: usize,
    /// Whether this is the first data read (for sweep handling)
    first_read: bool,
}

impl HspiceStreamReader {
    /// Open a file for true streaming read
    ///
    /// Only parses the header. Data is read on-demand.
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn open<P: AsRef<Path>>(path: P, min_chunk_size: usize) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Parse header only - returns metadata and data start position
        let (metadata, data_position) = parse_header_only(&mmap)?;

        // Compute number of columns per row
        let num_columns = if metadata.var_type == COMPLEX_VAR {
            metadata.num_vectors + (metadata.num_variables - 1) as usize
        } else {
            metadata.num_vectors
        };

        info!(
            signals = metadata.names.len(),
            scale = %metadata.scale_name,
            chunk_size = min_chunk_size,
            "Stream reader opened"
        );

        Ok(Self {
            mmap,
            data_position,
            metadata,
            min_chunk_size: min_chunk_size.max(1),
            current_chunk: 0,
            signal_filter: None,
            finished: false,
            row_buffer: Vec::new(),
            pending_data: Vec::new(),
            num_columns,
            first_read: true,
        })
    }

    /// Set signal filter to only read specific signals
    pub fn with_signals(mut self, signals: Vec<String>) -> Self {
        self.signal_filter = Some(signals.into_iter().collect());
        self
    }

    /// Get file metadata
    pub fn metadata(&self) -> StreamMetadata {
        StreamMetadata {
            title: self.metadata.title.clone(),
            date: self.metadata.date.clone(),
            scale_name: self.metadata.scale_name.clone(),
            signal_names: self.metadata.names.clone(),
            post_version: self.metadata.post_version,
            is_complex: self.metadata.var_type == COMPLEX_VAR,
        }
    }

    /// Reset reader to beginning of data section
    pub fn reset(&mut self) {
        if let Ok((_, pos)) = parse_header_only(&self.mmap) {
            self.data_position = pos;
            self.current_chunk = 0;
            self.finished = false;
            self.row_buffer.clear();
            self.pending_data.clear();
            self.first_read = true;
        }
    }

    /// Read one complete data block from file
    /// Returns raw f64 values, preserving block boundary
    fn read_one_block(&mut self) -> Result<Option<Vec<f64>>> {
        use crate::block_reader::BlockReader;

        if self.finished || self.data_position >= self.mmap.len() {
            return Ok(None);
        }

        let data_slice = &self.mmap[self.data_position..];
        let mut block_reader = BlockReader::new(data_slice, self.metadata.post_version);

        match block_reader.next_block()? {
            Some(block) => {
                // Update position
                self.data_position += block_reader.bytes_consumed();

                if block.is_end {
                    self.finished = true;
                }

                // Remove end marker if present
                let mut values = block.values;
                if block.is_end && !values.is_empty() {
                    values.pop();
                }

                Ok(Some(values))
            }
            None => {
                self.finished = true;
                Ok(None)
            }
        }
    }

    /// Parse raw block data into rows, handling incomplete rows at boundaries
    fn block_to_rows(&mut self, block_data: Vec<f64>) -> Vec<Vec<f64>> {
        if self.num_columns == 0 {
            return Vec::new();
        }

        // Prepend pending data from previous block
        let mut raw_data = std::mem::take(&mut self.pending_data);
        raw_data.extend(block_data);

        // Handle sweep value at very first read
        if self.first_read && self.metadata.sweep_name.is_some() && !raw_data.is_empty() {
            raw_data.remove(0); // Remove sweep value
        }
        self.first_read = false;

        // Calculate complete rows
        let total_values = raw_data.len();
        let num_complete_rows = total_values / self.num_columns;
        let complete_values = num_complete_rows * self.num_columns;

        // Save incomplete row for next block
        if complete_values < total_values {
            self.pending_data = raw_data[complete_values..].to_vec();
        }

        // Convert to rows
        let mut rows = Vec::with_capacity(num_complete_rows);
        for i in 0..num_complete_rows {
            let start = i * self.num_columns;
            let end = start + self.num_columns;
            rows.push(raw_data[start..end].to_vec());
        }

        rows
    }

    /// Flush any remaining pending data as a final row (if complete)
    fn flush_pending(&mut self) -> Vec<Vec<f64>> {
        if self.pending_data.len() >= self.num_columns && self.num_columns > 0 {
            let num_rows = self.pending_data.len() / self.num_columns;
            let mut rows = Vec::with_capacity(num_rows);
            for i in 0..num_rows {
                let start = i * self.num_columns;
                let end = start + self.num_columns;
                if end <= self.pending_data.len() {
                    rows.push(self.pending_data[start..end].to_vec());
                }
            }
            self.pending_data.clear();
            rows
        } else {
            Vec::new()
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Check if signal should be included based on filter
    #[inline]
    fn should_include_signal(&self, name: &str) -> bool {
        self.signal_filter
            .as_ref()
            .map(|f| f.contains(name))
            .unwrap_or(true)
    }

    /// Check if signal at given index is complex type
    #[inline]
    fn is_complex_signal(&self, signal_index: usize) -> bool {
        self.metadata.var_type == COMPLEX_VAR
            && signal_index < (self.metadata.num_variables - 1) as usize
    }

    // ========================================================================
    // Core Methods
    // ========================================================================

    /// Allocate storage for signal vectors based on filter and type
    fn allocate_signal_storage(
        &self,
        capacity: usize,
    ) -> (HashMap<String, Vec<f64>>, HashMap<String, Vec<Complex64>>) {
        let mut real_vecs = HashMap::new();
        let mut complex_vecs = HashMap::new();
        for (i, name) in self.metadata.names.iter().enumerate() {
            if !self.should_include_signal(name) {
                continue;
            }
            if self.is_complex_signal(i) {
                complex_vecs.insert(name.clone(), Vec::with_capacity(capacity));
            } else {
                real_vecs.insert(name.clone(), Vec::with_capacity(capacity));
            }
        }
        (real_vecs, complex_vecs)
    }

    /// Parse a single row into signal vectors
    fn parse_row_into_signals(
        &self,
        row: &[f64],
        real_vecs: &mut HashMap<String, Vec<f64>>,
        complex_vecs: &mut HashMap<String, Vec<Complex64>>,
    ) {
        let mut col_idx = 1;
        for (i, name) in self.metadata.names.iter().enumerate() {
            if col_idx >= row.len() {
                break;
            }
            let is_complex = self.is_complex_signal(i);
            let col_width = if is_complex { 2 } else { 1 };

            if self.should_include_signal(name) {
                if is_complex && col_idx + 1 < row.len() {
                    if let Some(vec) = complex_vecs.get_mut(name) {
                        vec.push(Complex64::new(row[col_idx], row[col_idx + 1]));
                    }
                } else if let Some(vec) = real_vecs.get_mut(name) {
                    vec.push(row[col_idx]);
                }
            }
            col_idx += col_width;
        }
    }

    /// Build chunk from accumulated rows
    fn build_chunk(&self, rows: &[Vec<f64>]) -> Option<DataChunk> {
        if rows.is_empty() {
            return None;
        }

        // Allocate storage
        let mut scale_vec: Vec<f64> = Vec::with_capacity(rows.len());
        let (mut real_vecs, mut complex_vecs) = self.allocate_signal_storage(rows.len());

        // Parse all rows
        for row in rows {
            if row.is_empty() {
                continue;
            }
            scale_vec.push(row[0]);
            self.parse_row_into_signals(row, &mut real_vecs, &mut complex_vecs);
        }

        // Build result
        let time_range = (
            scale_vec.first().copied().unwrap_or(0.0),
            scale_vec.last().copied().unwrap_or(0.0),
        );

        let mut data = HashMap::new();
        data.insert(
            self.metadata.scale_name.clone(),
            VectorData::Real(scale_vec),
        );
        data.extend(real_vecs.into_iter().map(|(k, v)| (k, VectorData::Real(v))));
        data.extend(
            complex_vecs
                .into_iter()
                .map(|(k, v)| (k, VectorData::Complex(v))),
        );

        Some(DataChunk {
            chunk_index: self.current_chunk,
            time_range,
            data,
        })
    }
}

impl Iterator for HspiceStreamReader {
    type Item = Result<DataChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished && self.row_buffer.is_empty() && self.pending_data.is_empty() {
            return None;
        }

        // Read complete blocks until we have at least min_chunk_size rows
        while self.row_buffer.len() < self.min_chunk_size && !self.finished {
            match self.read_one_block() {
                Ok(Some(block_data)) => {
                    let rows = self.block_to_rows(block_data);
                    self.row_buffer.extend(rows);
                }
                Ok(None) => break,
                Err(e) => return Some(Err(e)),
            }
        }

        // If finished, flush any pending data
        if self.finished && !self.pending_data.is_empty() {
            let final_rows = self.flush_pending();
            self.row_buffer.extend(final_rows);
        }

        if self.row_buffer.is_empty() {
            return None;
        }

        // Take all buffered rows for this chunk
        let chunk_rows = std::mem::take(&mut self.row_buffer);

        match self.build_chunk(&chunk_rows) {
            Some(chunk) => {
                trace!(
                    chunk = self.current_chunk,
                    points = chunk.data.values().next().map(|v| v.len()).unwrap_or(0),
                    time_start = chunk.time_range.0,
                    time_end = chunk.time_range.1,
                    "Chunk built"
                );
                self.current_chunk += 1;
                Some(Ok(chunk))
            }
            None => None,
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Open a file for streaming read with default chunk size
pub fn read_stream<P: AsRef<Path>>(path: P) -> Result<HspiceStreamReader> {
    HspiceStreamReader::open(path, DEFAULT_CHUNK_SIZE)
}

/// Open a file for streaming read with custom minimum chunk size
pub fn read_stream_chunked<P: AsRef<Path>>(
    path: P,
    chunk_size: usize,
) -> Result<HspiceStreamReader> {
    HspiceStreamReader::open(path, chunk_size)
}

/// Open a file for streaming read with signal filter
pub fn read_stream_signals<P: AsRef<Path>>(
    path: P,
    signals: &[&str],
    chunk_size: usize,
) -> Result<HspiceStreamReader> {
    let reader = HspiceStreamReader::open(path, chunk_size)?;
    Ok(reader.with_signals(signals.iter().map(|s| s.to_string()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_reader_basic() {
        let path = "example/PinToPinSim.tr0";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let reader = read_stream(path).expect("Failed to open file");
        let metadata = reader.metadata();

        assert!(!metadata.scale_name.is_empty());
        assert!(!metadata.signal_names.is_empty());

        let mut chunk_count = 0;
        for chunk in reader {
            let chunk = chunk.expect("Failed to read chunk");
            assert!(!chunk.data.is_empty());
            chunk_count += 1;
        }
        assert!(chunk_count > 0);
    }
}
