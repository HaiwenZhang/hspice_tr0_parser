# Streaming API Architecture

## Overview

This module provides a **true streaming API** for reading large HSPICE TR0 files, loading data blocks on-demand with peak memory independent of file size.

---

## Design Principles

| Principle                     | Implementation                                                          |
| ----------------------------- | ----------------------------------------------------------------------- |
| **On-demand loading**         | Only parse header (~1KB) at `open()`, read data blocks during iteration |
| **Preserve block boundaries** | Never split a data_block during reading                                 |
| **Handle cross-block rows**   | Incomplete rows accumulate in `pending_data`, merged with next block    |
| **O(chunk) memory**           | Peak memory = chunk_size × num_signals, independent of file size        |

---

## Memory Model

```
1GB file comparison:

┌────────────────────────────────────────────────────────────┐
│  Traditional full read                                      │
│  open(): Load 1GB → Memory 2-3GB                           │
│  iterate(): Access in-memory data                          │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│  True streaming                                             │
│  open(): Parse header (~1KB) → Memory ~0                   │
│  next(): Read 1 block (~80KB) → Memory ~80MB               │
│  Peak: O(chunk_size × num_signals)                         │
└────────────────────────────────────────────────────────────┘
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                       HspiceStreamReader                             │
├─────────────────────────────────────────────────────────────────────┤
│  mmap: Mmap              ← Memory-mapped file (zero memory cost)    │
│  data_position: usize    ← Current read position                    │
│  metadata: HeaderMetadata ← Parsed once at open()                   │
│  pending_data: Vec<f64>  ← Incomplete row across blocks             │
│  row_buffer: Vec<Row>    ← Rows for current chunk                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Iterator::next()                                                    │
│  ├── while row_buffer.len() < min_chunk_size && !finished:          │
│  │   ├── read_one_block() ─────────────┐                            │
│  │   │       ↓                         │                            │
│  │   │   Read complete data_block      │                            │
│  │   │   (preserve block boundary)     │                            │
│  │   │       ↓                         │                            │
│  │   └── block_to_rows()               │                            │
│  │       ├── Merge pending_data        │                            │
│  │       ├── Convert to complete rows  │                            │
│  │       └── Save incomplete tail to pending_data                   │
│  │                                                                   │
│  └── build_chunk(row_buffer) → DataChunk                            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Core Data Structures

### DataChunk

```rust
pub struct DataChunk {
    pub chunk_index: usize,           // Chunk index (0-based)
    pub time_range: (f64, f64),       // Time range [start, end]
    pub data: HashMap<String, VectorData>,  // Signal data
}
```

### Internal State

```rust
pub struct HspiceStreamReader {
    mmap: Mmap,                          // Memory-mapped file (zero overhead)
    data_position: usize,                // Current position
    metadata: HeaderMetadata,            // File header metadata
    pending_data: Vec<f64>,              // Cross-block accumulated data
    row_buffer: Vec<Vec<f64>>,           // Current chunk rows
    min_chunk_size: usize,               // Minimum rows to return
    finished: bool,                      // EOF flag
}
```

---

## API Usage

### Python

```python
from hspice_tr0_parser import hspice_tr0_stream

# Stream a large file
for chunk in hspice_tr0_stream("huge_1gb_file.tr0"):
    print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
    time = chunk['data']['TIME']
    vout = chunk['data']['vout']
    # Process current chunk...
    # Previous chunks are already garbage collected

# Signal filter - reduce memory further
for chunk in hspice_tr0_stream("huge.tr0", signals=['TIME', 'vout']):
    pass  # Only 2 signals in data

# Custom chunk size
for chunk in hspice_tr0_stream("huge.tr0", chunk_size=50000):
    pass  # ~50000 rows per chunk
```

### Rust

```rust
use hspicetr0parser::{read_stream, read_stream_signals};

let reader = read_stream("file.tr0")?;
for chunk in reader {
    let chunk = chunk?;
    println!("Chunk {}: {:?}", chunk.chunk_index, chunk.time_range);
}

// Signal filter
let reader = read_stream_signals("file.tr0", &["vout"], 10000)?;
```

### C

```c
CHspiceStream* stream = hspice_stream_open("file.tr0", 10000, 0);

while (hspice_stream_next(stream) == 1) {
    int size = hspice_stream_get_chunk_size(stream);
    double* buffer = malloc(size * sizeof(double));
    hspice_stream_get_signal_data(stream, "vout", buffer, size);
    // Process...
    free(buffer);
}

hspice_stream_close(stream);
```

---

## Block Boundary Handling

HSPICE files consist of multiple data_blocks, each may not contain complete rows:

```
Block 1: [row1_col1, row1_col2, row1_col3, row2_col1, row2_col2, row2_col3, row3_col1...]
                                                                           ↑ incomplete

Block 2: [row3_col2, row3_col3, row4_col1, row4_col2, row4_col3, row5...]
          ↑ continuation from previous block
```

Solution: `pending_data` buffer

```rust
fn block_to_rows(&mut self, block_data: Vec<f64>) -> Vec<Vec<f64>> {
    // 1. Merge incomplete data from previous block
    let mut raw_data = std::mem::take(&mut self.pending_data);
    raw_data.extend(block_data);

    // 2. Calculate complete rows
    let num_complete_rows = raw_data.len() / self.num_columns;

    // 3. Save incomplete tail to pending_data
    let complete_values = num_complete_rows * self.num_columns;
    if complete_values < raw_data.len() {
        self.pending_data = raw_data[complete_values..].to_vec();
    }

    // 4. Return complete rows
    ...
}
```

---

## Memory Usage Estimates

| Scenario   | File Size | Traditional | Streaming |
| ---------- | --------- | ----------- | --------- |
| Small      | 10MB      | ~30MB       | ~30MB     |
| Medium     | 100MB     | ~300MB      | ~80MB     |
| Large      | 1GB       | ~2-3GB      | ~80MB     |
| Very Large | 10GB      | OOM         | ~80MB     |

> Streaming peak memory ≈ chunk_size × num_signals × 8 bytes

---

## Verification Results

```
85 passed in 4.84s
```

- ✅ All tests pass
- ✅ Data integrity: streamed total points = full read
- ✅ Time continuity: no gaps between chunks
- ✅ Value matching: streamed data = full data

---

## File Structure

```
src/
├── stream.rs          # True streaming implementation
├── parser.rs          # Added parse_header_only() and pub HeaderMetadata
├── lib.rs             # Module exports and Python bindings
└── ffi.rs             # C FFI streaming interface

tests/
└── test_stream.py     # 19 streaming API tests
```
