//! Zero-copy HSPICE record-block reader.

use crate::reader::MmapReader;
use crate::types::{Endian, PostVersion, Result, WaveformError, END_MARKER_2001, END_MARKER_9601};

/// Encoded values borrowed directly from a data block.
pub(crate) struct RawBlock<'a> {
    pub bytes: &'a [u8],
    pub endian: Endian,
    pub is_end: bool,
}

/// Reads record framing while leaving numeric decoding to the caller.
pub(crate) struct BlockReader<'a> {
    reader: MmapReader<'a>,
    version: PostVersion,
    /// Number of blocks read so far
    block_count: usize,
}

impl<'a> BlockReader<'a> {
    /// Create a new block reader from the given data slice
    pub(crate) fn new(data: &'a [u8], version: PostVersion) -> Self {
        Self {
            reader: MmapReader::new(data),
            version,
            block_count: 0,
        }
    }

    /// Get item size in bytes
    #[inline]
    const fn item_size(&self) -> usize {
        match self.version {
            PostVersion::V9601 => 4,
            PostVersion::V2001 => 8,
        }
    }

    /// Read an encoded block without allocating or converting its values.
    pub(crate) fn next_raw_block(&mut self) -> Result<Option<RawBlock<'a>>> {
        if self.reader.remaining() == 0 {
            return Ok(None);
        }

        let item_size = self.item_size();
        let (num_items, trailer) = self.reader.read_block_header(item_size)?;
        let bytes = self.reader.read_bytes(num_items * item_size)?;
        let endian = self
            .reader
            .endian
            .ok_or_else(|| WaveformError::FormatError("missing endianness".into()))?;

        let is_end = match (self.version, bytes.len()) {
            (PostVersion::V9601, len) if len >= 4 => {
                let start = len - 4;
                endian.read_f32([
                    bytes[start],
                    bytes[start + 1],
                    bytes[start + 2],
                    bytes[start + 3],
                ]) >= END_MARKER_9601
            }
            (PostVersion::V2001, len) if len >= 8 => {
                let start = len - 8;
                endian.read_f64([
                    bytes[start],
                    bytes[start + 1],
                    bytes[start + 2],
                    bytes[start + 3],
                    bytes[start + 4],
                    bytes[start + 5],
                    bytes[start + 6],
                    bytes[start + 7],
                ]) >= END_MARKER_2001
            }
            _ => false,
        };

        self.reader.read_block_trailer(trailer)?;

        self.block_count += 1;
        Ok(Some(RawBlock {
            bytes,
            endian,
            is_end,
        }))
    }

    /// Get the number of blocks read
    #[inline]
    pub(crate) fn block_count(&self) -> usize {
        self.block_count
    }

    /// Get format name (for debug output)
    #[inline]
    pub(crate) fn format_name(&self) -> &'static str {
        match self.version {
            PostVersion::V9601 => "f32",
            PostVersion::V2001 => "f64",
        }
    }

    /// Get the number of bytes consumed
    #[inline]
    pub(crate) fn bytes_consumed(&self) -> usize {
        self.reader.position()
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
