//! Memory-mapped file reader for efficient large file parsing

use std::io::{Error, ErrorKind};

use crate::types::{Endian, Result, WaveformError};

/// Memory-mapped file reader for efficient large file parsing
pub(crate) struct MmapReader<'a> {
    data: &'a [u8],
    pos: usize,
    pub(crate) endian: Option<Endian>,
}

impl<'a> MmapReader<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            endian: None,
        }
    }

    #[inline]
    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Get current read position
    #[inline]
    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    /// Borrow all unread bytes without changing the current position.
    #[inline]
    pub(crate) fn remaining_bytes(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    /// Advance over already-consumed bytes.
    #[inline]
    pub(crate) fn advance(&mut self, count: usize) -> Result<()> {
        self.read_bytes(count).map(|_| ())
    }

    #[inline]
    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| WaveformError::FormatError("read position overflow".into()))?;
        if end > self.data.len() {
            return Err(WaveformError::IoError(Error::new(
                ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            )));
        }
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    /// Read and detect endianness from block header
    pub(crate) fn read_block_header(&mut self, item_size: usize) -> Result<(usize, i32)> {
        if item_size == 0 {
            return Err(WaveformError::FormatError(
                "block item size must be non-zero".into(),
            ));
        }
        let header_bytes = self.read_bytes(16)?;

        // Check endianness by examining first and third int
        let first_le = i32::from_le_bytes([
            header_bytes[0],
            header_bytes[1],
            header_bytes[2],
            header_bytes[3],
        ]);
        let first_be = i32::from_be_bytes([
            header_bytes[0],
            header_bytes[1],
            header_bytes[2],
            header_bytes[3],
        ]);
        let third_le = i32::from_le_bytes([
            header_bytes[8],
            header_bytes[9],
            header_bytes[10],
            header_bytes[11],
        ]);
        let third_be = i32::from_be_bytes([
            header_bytes[8],
            header_bytes[9],
            header_bytes[10],
            header_bytes[11],
        ]);

        let endian = if first_le == 0x00000004 && third_le == 0x00000004 {
            Endian::Little
        } else if first_be == 0x00000004 && third_be == 0x00000004 {
            Endian::Big
        } else {
            return Err(WaveformError::FormatError("corrupted block header".into()));
        };

        self.endian = Some(endian);

        let trailer_value = endian.read_i32([
            header_bytes[12],
            header_bytes[13],
            header_bytes[14],
            header_bytes[15],
        ]);

        let byte_count = usize::try_from(trailer_value).map_err(|_| {
            WaveformError::FormatError(format!("negative block byte count: {trailer_value}"))
        })?;
        if byte_count % item_size != 0 {
            return Err(WaveformError::FormatError(format!(
                "block byte count {byte_count} is not divisible by item size {item_size}"
            )));
        }
        let num_items = byte_count / item_size;
        Ok((num_items, trailer_value))
    }

    /// Read block trailer and verify
    pub(crate) fn read_block_trailer(&mut self, expected: i32) -> Result<()> {
        let trailer_bytes = self.read_bytes(4)?;
        let endian = self
            .endian
            .ok_or_else(|| WaveformError::FormatError("block endianness is unknown".into()))?;
        let trailer = endian.read_i32([
            trailer_bytes[0],
            trailer_bytes[1],
            trailer_bytes[2],
            trailer_bytes[3],
        ]);

        if trailer != expected {
            return Err(WaveformError::FormatError(
                "Block header and trailer mismatch".into(),
            ));
        }
        Ok(())
    }
}
