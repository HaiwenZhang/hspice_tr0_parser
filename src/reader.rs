//! Memory-mapped file reader for efficient large file parsing

use crate::types::{Endian, HspiceError, Result};

/// Memory-mapped file reader for efficient large file parsing
pub struct MmapReader<'a> {
    data: &'a [u8],
    pos: usize,
    pub endian: Option<Endian>,
}

impl<'a> MmapReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            endian: None,
        }
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    #[inline]
    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        if self.pos + count > self.data.len() {
            return Err(HspiceError::IoError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            )));
        }
        let bytes = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    /// Read and detect endianness from block header
    pub fn read_block_header(&mut self, item_size: usize) -> Result<(usize, i32)> {
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
            return Err(HspiceError::FormatError("Corrupted block header".into()));
        };

        self.endian = Some(endian);

        let trailer_value = match endian {
            Endian::Little => i32::from_le_bytes([
                header_bytes[12],
                header_bytes[13],
                header_bytes[14],
                header_bytes[15],
            ]),
            Endian::Big => i32::from_be_bytes([
                header_bytes[12],
                header_bytes[13],
                header_bytes[14],
                header_bytes[15],
            ]),
        };

        let num_items = (trailer_value as usize) / item_size;
        Ok((num_items, trailer_value))
    }

    /// Read block trailer and verify
    pub fn read_block_trailer(&mut self, expected: i32) -> Result<()> {
        let trailer_bytes = self.read_bytes(4)?;
        let trailer = match self.endian.unwrap_or(Endian::Little) {
            Endian::Little => i32::from_le_bytes([
                trailer_bytes[0],
                trailer_bytes[1],
                trailer_bytes[2],
                trailer_bytes[3],
            ]),
            Endian::Big => i32::from_be_bytes([
                trailer_bytes[0],
                trailer_bytes[1],
                trailer_bytes[2],
                trailer_bytes[3],
            ]),
        };

        if trailer != expected {
            return Err(HspiceError::FormatError(
                "Block header and trailer mismatch".into(),
            ));
        }
        Ok(())
    }

    /// Read f32 values and convert to f64, appending directly to target Vec
    /// This avoids creating an intermediate Vec<f32>
    #[inline]
    pub fn read_floats_as_f64_into(&mut self, count: usize, target: &mut Vec<f64>) -> Result<()> {
        let byte_count = count * 4; // f32 is 4 bytes
        let bytes = self.read_bytes(byte_count)?;

        target.reserve(count);
        let is_little = matches!(self.endian.unwrap_or(Endian::Little), Endian::Little);

        // Process 2 values at a time for better pipelining
        let chunks = bytes.chunks_exact(8);
        let remainder = chunks.remainder();

        for chunk in chunks {
            if is_little {
                let v1 = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let v2 = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                target.push(v1 as f64);
                target.push(v2 as f64);
            } else {
                let v1 = f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let v2 = f32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                target.push(v1 as f64);
                target.push(v2 as f64);
            }
        }

        // Handle remaining bytes (0 or 4 bytes)
        if remainder.len() >= 4 {
            let v = if is_little {
                f32::from_le_bytes([remainder[0], remainder[1], remainder[2], remainder[3]])
            } else {
                f32::from_be_bytes([remainder[0], remainder[1], remainder[2], remainder[3]])
            };
            target.push(v as f64);
        }

        Ok(())
    }

    /// Read f64 values, appending directly to target Vec
    #[inline]
    pub fn read_doubles_into(&mut self, count: usize, target: &mut Vec<f64>) -> Result<()> {
        let byte_count = count * 8;
        let bytes = self.read_bytes(byte_count)?;

        target.reserve(count);
        let is_little = matches!(self.endian.unwrap_or(Endian::Little), Endian::Little);

        for chunk in bytes.chunks_exact(8) {
            let v = if is_little {
                f64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ])
            } else {
                f64::from_be_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ])
            };
            target.push(v);
        }

        Ok(())
    }
}
