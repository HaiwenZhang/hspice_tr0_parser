//! Memory-bounded streaming access to large HSPICE files.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use tracing::{info, instrument, trace};

use crate::block_reader::BlockReader;
use crate::data_builder::DataTableBuilder;
use crate::parser::{parse_header_only, HeaderMetadata};
use crate::types::{DataTable, PostVersion, Result, VectorData, WaveformError, COMPLEX_VAR};

/// Default minimum number of points returned in each chunk.
pub const DEFAULT_CHUNK_SIZE: usize = 10_000;

/// One column-major data chunk from a streaming reader.
#[derive(Debug, Clone)]
pub struct DataChunk {
    /// Zero-based chunk index.
    pub chunk_index: usize,
    /// Scale range covered by this chunk.
    pub time_range: (f64, f64),
    /// Scale and selected signal vectors, keyed by name.
    pub data: HashMap<String, VectorData>,
}

/// Header metadata exposed without decoding waveform samples.
#[derive(Debug, Clone)]
pub struct StreamMetadata {
    /// File title.
    pub title: String,
    /// File date.
    pub date: String,
    /// Scale name, such as `TIME` or `HERTZ`.
    pub scale_name: String,
    /// Signal names, excluding the scale vector.
    pub signal_names: Vec<String>,
    /// HSPICE post format version.
    pub post_version: PostVersion,
    /// Whether signal values are complex.
    pub is_complex: bool,
}

/// Incremental reader that decodes blocks directly into final column vectors.
pub struct HspiceStreamReader {
    mmap: Mmap,
    data_start: usize,
    data_position: usize,
    metadata: HeaderMetadata,
    min_chunk_size: usize,
    current_chunk: usize,
    signal_filter: Option<HashSet<String>>,
    finished: bool,
    num_complex_signals: usize,
    builder: DataTableBuilder,
}

impl HspiceStreamReader {
    /// Opens an HSPICE file and parses only its header.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be opened, mapped, or parsed.
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn open<P: AsRef<Path>>(path: P, min_chunk_size: usize) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        // SAFETY: `file` remains alive until the map is created, and the resulting
        // read-only mapping owns its OS mapping independently of the file handle.
        let mmap = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        if let Err(error) = mmap.advise(memmap2::Advice::Sequential) {
            trace!(%error, "Could not set sequential mapping advice");
        }

        let (metadata, data_start) = parse_header_only(&mmap)?;
        let min_chunk_size = min_chunk_size.max(1);
        let num_complex_signals = Self::complex_signal_count(&metadata)?;
        let builder = Self::new_builder(&metadata, num_complex_signals, min_chunk_size);

        info!(
            signals = metadata.names.len(),
            scale = %metadata.scale_name,
            chunk_size = min_chunk_size,
            "Stream reader opened"
        );

        Ok(Self {
            mmap,
            data_start,
            data_position: data_start,
            metadata,
            min_chunk_size,
            current_chunk: 0,
            signal_filter: None,
            finished: false,
            num_complex_signals,
            builder,
        })
    }

    /// Restricts returned chunks to the requested signals.
    #[must_use]
    pub fn with_signals(mut self, signals: Vec<String>) -> Self {
        self.signal_filter = Some(signals.into_iter().collect());
        self
    }

    /// Returns file metadata.
    #[must_use]
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

    /// Resets iteration to the first data block without reparsing the header.
    pub fn reset(&mut self) {
        self.data_position = self.data_start;
        self.current_chunk = 0;
        self.finished = false;
        self.builder = Self::new_builder(
            &self.metadata,
            self.num_complex_signals,
            self.min_chunk_size,
        );
    }

    fn complex_signal_count(metadata: &HeaderMetadata) -> Result<usize> {
        if metadata.var_type == COMPLEX_VAR {
            usize::try_from(metadata.num_variables - 1).map_err(|_| {
                WaveformError::FormatError(format!(
                    "invalid complex variable count: {}",
                    metadata.num_variables
                ))
            })
        } else {
            Ok(0)
        }
    }

    fn new_builder(
        metadata: &HeaderMetadata,
        num_complex_signals: usize,
        capacity: usize,
    ) -> DataTableBuilder {
        DataTableBuilder::new(
            metadata.post_version,
            metadata.num_vectors,
            num_complex_signals,
            metadata.sweep_name.is_some(),
            capacity,
        )
    }

    /// Reads and decodes one complete record block without a temporary value buffer.
    fn read_one_block(&mut self) -> Result<bool> {
        if self.finished || self.data_position >= self.mmap.len() {
            self.finished = true;
            return Ok(false);
        }

        let (consumed, is_end) = {
            let data = &self.mmap[self.data_position..];
            let mut reader = BlockReader::new(data, self.metadata.post_version);
            let Some(block) = reader.next_raw_block()? else {
                self.finished = true;
                return Ok(false);
            };
            self.builder
                .push_raw_block(block.bytes, block.endian, block.is_end);
            (reader.bytes_consumed(), block.is_end)
        };

        self.data_position = self
            .data_position
            .checked_add(consumed)
            .ok_or_else(|| WaveformError::FormatError("data position overflow".into()))?;
        self.finished = is_end;
        Ok(true)
    }

    fn should_include_signal(&self, name: &str) -> bool {
        self.signal_filter
            .as_ref()
            .is_none_or(|filter| filter.contains(name))
    }

    fn build_chunk(&self, table: DataTable) -> Result<DataChunk> {
        let mut vectors = table.vectors.into_iter();
        let scale = vectors
            .next()
            .ok_or_else(|| WaveformError::FormatError("chunk has no scale vector".into()))?;
        let time_range = match &scale {
            VectorData::Real(values) => {
                let first = values.first().ok_or_else(|| {
                    WaveformError::FormatError("chunk scale vector is empty".into())
                })?;
                let last = values.last().ok_or_else(|| {
                    WaveformError::FormatError("chunk scale vector is empty".into())
                })?;
                (*first, *last)
            }
            VectorData::Complex(_) => {
                return Err(WaveformError::FormatError(
                    "chunk scale vector cannot be complex".into(),
                ));
            }
        };

        let mut data = HashMap::with_capacity(self.metadata.names.len() + 1);
        data.insert(self.metadata.scale_name.clone(), scale);
        for (name, vector) in self.metadata.names.iter().zip(vectors) {
            if self.should_include_signal(name) {
                data.insert(name.clone(), vector);
            }
        }

        Ok(DataChunk {
            chunk_index: self.current_chunk,
            time_range,
            data,
        })
    }
}

impl Iterator for HspiceStreamReader {
    type Item = Result<DataChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished && self.builder.is_empty() {
            return None;
        }

        while self.builder.len() < self.min_chunk_size && !self.finished {
            match self.read_one_block() {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    self.finished = true;
                    return Some(Err(error));
                }
            }
        }

        if self.builder.is_empty() {
            if self.finished && self.builder.trailing_value_count() > 0 {
                trace!(
                    trailing_values = self.builder.trailing_value_count(),
                    "Ignored incomplete final row"
                );
            }
            return None;
        }

        let table = self.builder.take_table(self.min_chunk_size);
        let chunk = match self.build_chunk(table) {
            Ok(chunk) => chunk,
            Err(error) => {
                self.finished = true;
                return Some(Err(error));
            }
        };
        trace!(
            chunk = self.current_chunk,
            points = chunk.data.values().next().map_or(0, VectorData::len),
            time_start = chunk.time_range.0,
            time_end = chunk.time_range.1,
            "Chunk built"
        );
        self.current_chunk += 1;
        Some(Ok(chunk))
    }
}

/// Opens a file with [`DEFAULT_CHUNK_SIZE`].
///
/// # Errors
///
/// Returns an error when the file cannot be opened, mapped, or parsed.
pub fn read_stream<P: AsRef<Path>>(path: P) -> Result<HspiceStreamReader> {
    HspiceStreamReader::open(path, DEFAULT_CHUNK_SIZE)
}

/// Opens a file with a custom minimum chunk size.
///
/// # Errors
///
/// Returns an error when the file cannot be opened, mapped, or parsed.
pub fn read_stream_chunked<P: AsRef<Path>>(
    path: P,
    chunk_size: usize,
) -> Result<HspiceStreamReader> {
    HspiceStreamReader::open(path, chunk_size)
}

/// Opens a file and restricts returned chunks to selected signals.
///
/// # Errors
///
/// Returns an error when the file cannot be opened, mapped, or parsed.
pub fn read_stream_signals<P: AsRef<Path>>(
    path: P,
    signals: &[&str],
    chunk_size: usize,
) -> Result<HspiceStreamReader> {
    HspiceStreamReader::open(path, chunk_size).map(|reader| {
        reader.with_signals(signals.iter().map(|signal| (*signal).to_owned()).collect())
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::read_stream;

    #[test]
    fn stream_reader_yields_non_empty_chunks() {
        let path = "example/PinToPinSim.tr0";
        if !Path::new(path).exists() {
            return;
        }

        let reader = read_stream(path).expect("test fixture should open");
        let chunks = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("test fixture should decode");

        assert!(chunks.iter().all(|chunk| !chunk.data.is_empty()));
    }
}
