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

    #[inline]
    #[allow(dead_code)]
    pub fn skip(&mut self, count: usize) -> Result<()> {
        if self.pos + count > self.data.len() {
            return Err(HspiceError::IoError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            )));
        }
        self.pos += count;
        Ok(())
    }

    #[inline]
    #[allow(dead_code)]
    pub fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.read_bytes(4)?;
        Ok(match self.endian.unwrap_or(Endian::Little) {
            Endian::Little => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endian::Big => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    #[inline]
    #[allow(dead_code)]
    pub fn read_f32(&mut self) -> Result<f32> {
        let bytes = self.read_bytes(4)?;
        Ok(match self.endian.unwrap_or(Endian::Little) {
            Endian::Little => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endian::Big => f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
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

    /// Bulk read floats from a block - optimized for large data
    #[inline]
    pub fn read_floats_bulk(&mut self, count: usize) -> Result<Vec<f32>> {
        let byte_count = count * 4;
        let bytes = self.read_bytes(byte_count)?;

        let mut result = Vec::with_capacity(count);

        match self.endian.unwrap_or(Endian::Little) {
            Endian::Little => {
                // Process 4 floats at a time for better cache utilization
                let chunks = bytes.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    result.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    result.push(f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]));
                    result.push(f32::from_le_bytes([
                        chunk[8], chunk[9], chunk[10], chunk[11],
                    ]));
                    result.push(f32::from_le_bytes([
                        chunk[12], chunk[13], chunk[14], chunk[15],
                    ]));
                }

                // Handle remaining bytes
                for chunk in remainder.chunks_exact(4) {
                    result.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
            }
            Endian::Big => {
                let chunks = bytes.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    result.push(f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    result.push(f32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]));
                    result.push(f32::from_be_bytes([
                        chunk[8], chunk[9], chunk[10], chunk[11],
                    ]));
                    result.push(f32::from_be_bytes([
                        chunk[12], chunk[13], chunk[14], chunk[15],
                    ]));
                }

                for chunk in remainder.chunks_exact(4) {
                    result.push(f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
            }
        }

        Ok(result)
    }

    /// Bulk read doubles from a block - for 2001 format (8-byte double precision)
    #[inline]
    pub fn read_doubles_bulk(&mut self, count: usize) -> Result<Vec<f64>> {
        let byte_count = count * 8;
        let bytes = self.read_bytes(byte_count)?;

        let mut result = Vec::with_capacity(count);

        match self.endian.unwrap_or(Endian::Little) {
            Endian::Little => {
                // Process 2 doubles at a time for better cache utilization
                let chunks = bytes.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    result.push(f64::from_le_bytes([
                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                        chunk[7],
                    ]));
                    result.push(f64::from_le_bytes([
                        chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14],
                        chunk[15],
                    ]));
                }

                // Handle remaining bytes
                for chunk in remainder.chunks_exact(8) {
                    result.push(f64::from_le_bytes([
                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                        chunk[7],
                    ]));
                }
            }
            Endian::Big => {
                let chunks = bytes.chunks_exact(16);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    result.push(f64::from_be_bytes([
                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                        chunk[7],
                    ]));
                    result.push(f64::from_be_bytes([
                        chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14],
                        chunk[15],
                    ]));
                }

                for chunk in remainder.chunks_exact(8) {
                    result.push(f64::from_be_bytes([
                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                        chunk[7],
                    ]));
                }
            }
        }

        Ok(result)
    }
}
