//! HSPICE Data Block Reader
//!
//! Unifies block reading logic from parser.rs and stream.rs.
//! Follows the "Single Source of Truth" principle for all data block reads.

use crate::reader::MmapReader;
use crate::types::{PostVersion, Result, END_MARKER_2001, END_MARKER_9601};

// ============================================================================
// Core Structures
// ============================================================================

/// Result of reading a single data block
#[derive(Debug)]
pub struct BlockData {
    /// Data values in this block
    pub values: Vec<f64>,
    /// Whether this is the last block (end marker detected)
    pub is_end: bool,
}

/// Data block reader
///
/// Provides unified interface for reading HSPICE binary file data blocks.
/// Supports two formats:
/// - V9601: 4-byte float32
/// - V2001: 8-byte float64
pub struct BlockReader<'a> {
    reader: MmapReader<'a>,
    version: PostVersion,
    /// Number of blocks read so far
    block_count: usize,
}

impl<'a> BlockReader<'a> {
    /// Create a new block reader from the given data slice
    pub fn new(data: &'a [u8], version: PostVersion) -> Self {
        Self {
            reader: MmapReader::new(data),
            version,
            block_count: 0,
        }
    }

    /// Get item size in bytes
    #[inline]
    fn item_size(&self) -> usize {
        match self.version {
            PostVersion::V9601 => 4,
            PostVersion::V2001 => 8,
        }
    }

    /// Read the next data block
    ///
    /// Returns `None` if end of file or read failure.
    /// Returns `Some(BlockData)` containing data and end-of-data flag.
    pub fn next_block(&mut self) -> Result<Option<BlockData>> {
        if self.reader.remaining() == 0 {
            return Ok(None);
        }

        let item_size = self.item_size();

        // Read block header
        let (num_items, trailer) = match self.reader.read_block_header(item_size) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        // Read data and detect end marker
        let mut values = Vec::with_capacity(num_items);
        let is_end = match self.version {
            PostVersion::V9601 => {
                self.reader
                    .read_floats_as_f64_into(num_items, &mut values)?;
                values
                    .last()
                    .map(|&v| v as f32 >= END_MARKER_9601)
                    .unwrap_or(false)
            }
            PostVersion::V2001 => {
                self.reader.read_doubles_into(num_items, &mut values)?;
                values
                    .last()
                    .map(|&v| v >= END_MARKER_2001)
                    .unwrap_or(false)
            }
        };

        // Read block trailer
        if self.reader.read_block_trailer(trailer).is_err() {
            return Ok(None);
        }

        self.block_count += 1;

        Ok(Some(BlockData { values, is_end }))
    }

    /// Read all data blocks into a single Vec
    ///
    /// Used for one-shot reading scenarios (e.g., parser.rs).
    pub fn read_all(&mut self) -> Result<Vec<f64>> {
        let estimated = self.reader.remaining() / self.estimate_divisor();
        let mut all_data = Vec::with_capacity(estimated);

        while let Some(block) = self.next_block()? {
            all_data.extend(block.values);
            if block.is_end {
                break;
            }
        }

        Ok(all_data)
    }

    /// Get the number of blocks read
    #[inline]
    pub fn block_count(&self) -> usize {
        self.block_count
    }

    /// Get format name (for debug output)
    #[inline]
    pub fn format_name(&self) -> &'static str {
        match self.version {
            PostVersion::V9601 => "f32",
            PostVersion::V2001 => "f64",
        }
    }

    /// Get divisor for capacity estimation
    ///
    /// These values are empirical estimates accounting for:
    /// - Block header overhead (16 bytes) + trailer (4 bytes) = 20 bytes per block
    /// - Average block size varies by format
    ///
    /// V9601: ~5 bytes per value (4 byte f32 + ~1 byte overhead amortized)
    /// V2001: ~9 bytes per value (8 byte f64 + ~1 byte overhead amortized)
    #[inline]
    fn estimate_divisor(&self) -> usize {
        match self.version {
            PostVersion::V9601 => 5, // 4 bytes (f32) + overhead
            PostVersion::V2001 => 9, // 8 bytes (f64) + overhead
        }
    }

    /// Get the number of bytes consumed
    #[inline]
    pub fn bytes_consumed(&self) -> usize {
        self.reader.position()
    }
}

// ============================================================================
// Iterator Implementation
// ============================================================================

impl<'a> Iterator for BlockReader<'a> {
    type Item = Result<BlockData>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_block() {
            Ok(Some(block)) => Some(Ok(block)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_reader_format_name() {
        // Verify format name is correct
        let empty: &[u8] = &[];
        let reader_9601 = BlockReader::new(empty, PostVersion::V9601);
        assert_eq!(reader_9601.format_name(), "f32");

        let reader_2001 = BlockReader::new(empty, PostVersion::V2001);
        assert_eq!(reader_2001.format_name(), "f64");
    }
}
