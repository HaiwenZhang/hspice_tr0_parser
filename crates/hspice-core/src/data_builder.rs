//! Incremental decoding from HSPICE blocks into column-major vectors.

use num_complex::Complex64;

use crate::types::{DataTable, Endian, PostVersion, VectorData};

enum VectorBuilder {
    Real(Vec<f64>),
    Complex(Vec<Complex64>),
}

trait ValueDecoder {
    const SIZE: usize;

    fn decode(bytes: &[u8]) -> f64;
}

struct F32Little;
struct F32Big;
struct F64Little;
struct F64Big;

impl ValueDecoder for F32Little {
    const SIZE: usize = 4;

    #[inline]
    fn decode(bytes: &[u8]) -> f64 {
        f64::from(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl ValueDecoder for F32Big {
    const SIZE: usize = 4;

    #[inline]
    fn decode(bytes: &[u8]) -> f64 {
        f64::from(f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl ValueDecoder for F64Little {
    const SIZE: usize = 8;

    #[inline]
    fn decode(bytes: &[u8]) -> f64 {
        f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

impl ValueDecoder for F64Big {
    const SIZE: usize = 8;

    #[inline]
    fn decode(bytes: &[u8]) -> f64 {
        f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

/// Builds final vectors directly while preserving partial rows across blocks.
pub(crate) struct DataTableBuilder {
    version: PostVersion,
    num_columns: usize,
    num_complex_signals: usize,
    sweep_value: Option<f64>,
    expects_sweep_value: bool,
    pending: Vec<f64>,
    vectors: Vec<VectorBuilder>,
}

impl DataTableBuilder {
    pub(crate) fn new(
        version: PostVersion,
        num_vectors: usize,
        num_complex_signals: usize,
        has_sweep: bool,
        capacity: usize,
    ) -> Self {
        let num_columns = num_vectors + num_complex_signals;
        let vectors = (0..num_vectors)
            .map(|index| {
                if index > 0 && index - 1 < num_complex_signals {
                    VectorBuilder::Complex(Vec::with_capacity(capacity))
                } else {
                    VectorBuilder::Real(Vec::with_capacity(capacity))
                }
            })
            .collect();

        Self {
            version,
            num_columns,
            num_complex_signals,
            sweep_value: None,
            expects_sweep_value: has_sweep,
            pending: Vec::with_capacity(num_columns),
            vectors,
        }
    }

    /// Appends one encoded block. Rows may span block boundaries.
    pub(crate) fn push_raw_block(&mut self, bytes: &[u8], endian: Endian, is_end: bool) {
        match (self.version, endian) {
            (PostVersion::V9601, Endian::Little) => {
                self.push_encoded_values::<F32Little>(bytes, is_end);
            }
            (PostVersion::V9601, Endian::Big) => {
                self.push_encoded_values::<F32Big>(bytes, is_end);
            }
            (PostVersion::V2001, Endian::Little) => {
                self.push_encoded_values::<F64Little>(bytes, is_end);
            }
            (PostVersion::V2001, Endian::Big) => {
                self.push_encoded_values::<F64Big>(bytes, is_end);
            }
        }
    }

    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.vectors.first().map_or(0, VectorBuilder::len)
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub(crate) fn trailing_value_count(&self) -> usize {
        self.pending.len()
    }

    /// Drains complete vectors while retaining an incomplete row for the next chunk.
    pub(crate) fn take_table(&mut self, next_capacity: usize) -> DataTable {
        DataTable {
            sweep_value: self.sweep_value.take(),
            vectors: self
                .vectors
                .iter_mut()
                .map(|builder| builder.take(next_capacity))
                .collect(),
        }
    }

    pub(crate) fn finish(self) -> DataTable {
        DataTable {
            sweep_value: self.sweep_value,
            vectors: self
                .vectors
                .into_iter()
                .map(VectorBuilder::into_vector_data)
                .collect(),
        }
    }

    fn push_encoded_values<D: ValueDecoder>(&mut self, bytes: &[u8], is_end: bool) {
        if self.num_columns == 0 || bytes.is_empty() {
            return;
        }

        let marker_bytes = if is_end { D::SIZE } else { 0 };
        let data_end = bytes.len().saturating_sub(marker_bytes);
        let mut values = &bytes[..data_end];

        if self.expects_sweep_value {
            let Some(raw_sweep) = values.get(..D::SIZE) else {
                return;
            };
            self.sweep_value = Some(D::decode(raw_sweep));
            self.expects_sweep_value = false;
            values = &values[D::SIZE..];
        }

        if !self.pending.is_empty() {
            let needed = self.num_columns - self.pending.len();
            let available = values.len() / D::SIZE;
            let take = needed.min(available);
            let take_bytes = take * D::SIZE;
            self.pending
                .extend(values[..take_bytes].chunks_exact(D::SIZE).map(D::decode));
            values = &values[take_bytes..];

            if self.pending.len() != self.num_columns {
                return;
            }
            Self::push_row(&mut self.vectors, self.num_complex_signals, &self.pending);
            self.pending.clear();
        }

        let row_bytes = self.num_columns * D::SIZE;
        let mut rows = values.chunks_exact(row_bytes);
        for row in &mut rows {
            Self::push_encoded_row::<D>(&mut self.vectors, self.num_complex_signals, row);
        }
        self.pending
            .extend(rows.remainder().chunks_exact(D::SIZE).map(D::decode));
    }

    #[inline]
    fn push_encoded_row<D: ValueDecoder>(
        vectors: &mut [VectorBuilder],
        num_complex_signals: usize,
        row: &[u8],
    ) {
        let Some((scale, signals)) = vectors.split_first_mut() else {
            return;
        };
        if let VectorBuilder::Real(values) = scale {
            values.push(D::decode(row));
        }

        if num_complex_signals == 0 {
            for (signal, raw_value) in signals.iter_mut().zip(row[D::SIZE..].chunks_exact(D::SIZE))
            {
                if let VectorBuilder::Real(values) = signal {
                    values.push(D::decode(raw_value));
                }
            }
            return;
        }

        let mut byte_offset = D::SIZE;
        for (signal_index, signal) in signals.iter_mut().enumerate() {
            if signal_index < num_complex_signals {
                if let VectorBuilder::Complex(values) = signal {
                    values.push(Complex64::new(
                        D::decode(&row[byte_offset..]),
                        D::decode(&row[byte_offset + D::SIZE..]),
                    ));
                }
                byte_offset += 2 * D::SIZE;
            } else {
                if let VectorBuilder::Real(values) = signal {
                    values.push(D::decode(&row[byte_offset..]));
                }
                byte_offset += D::SIZE;
            }
        }
    }

    #[inline]
    fn push_row(vectors: &mut [VectorBuilder], num_complex_signals: usize, row: &[f64]) {
        let Some((scale, signals)) = vectors.split_first_mut() else {
            return;
        };
        if let VectorBuilder::Real(values) = scale {
            values.push(row[0]);
        }

        if num_complex_signals == 0 {
            for (signal, &value) in signals.iter_mut().zip(&row[1..]) {
                if let VectorBuilder::Real(values) = signal {
                    values.push(value);
                }
            }
            return;
        }

        let mut column = 1;
        for (signal_index, signal) in signals.iter_mut().enumerate() {
            if signal_index < num_complex_signals {
                if let VectorBuilder::Complex(values) = signal {
                    values.push(Complex64::new(row[column], row[column + 1]));
                }
                column += 2;
            } else {
                if let VectorBuilder::Real(values) = signal {
                    values.push(row[column]);
                }
                column += 1;
            }
        }
    }
}

impl VectorBuilder {
    fn len(&self) -> usize {
        match self {
            Self::Real(values) => values.len(),
            Self::Complex(values) => values.len(),
        }
    }

    fn take(&mut self, capacity: usize) -> VectorData {
        match self {
            Self::Real(values) => {
                VectorData::Real(std::mem::replace(values, Vec::with_capacity(capacity)))
            }
            Self::Complex(values) => {
                VectorData::Complex(std::mem::replace(values, Vec::with_capacity(capacity)))
            }
        }
    }

    fn into_vector_data(self) -> VectorData {
        match self {
            Self::Real(values) => VectorData::Real(values),
            Self::Complex(values) => VectorData::Complex(values),
        }
    }
}
