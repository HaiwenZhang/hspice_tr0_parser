# Go API Documentation

This document covers using `hspice_tr0_parser` from Go via CGO, leveraging the C FFI bindings.

## Prerequisites

- Go 1.18 or later
- GCC or Clang for CGO
- Built static library (`libhspicetr0parser.a`)

## Building the Library

```bash
# Clone and build
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser

# Build the static library
cargo build -p hspice-ffi --release

# Copy artifacts to your Go project
cp target/release/libhspicetr0parser.a /path/to/your/go/project/
cp include/hspice_tr0_parser.h /path/to/your/go/project/
```

## Go Wrapper

Create a Go package that wraps the C FFI:

### `hspice/hspice.go`

```go
package hspice

/*
#cgo LDFLAGS: -L${SRCDIR} -lhspicetr0parser -ldl -lm
#include "hspice_tr0_parser.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"unsafe"
)

// Result wraps the HSPICE parsing result
type Result struct {
	handle *C.CHspiceResult
}

// Read parses an HSPICE binary file
func Read(filename string) (*Result, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	handle := C.hspice_read(cFilename, 0)
	if handle == nil {
		return nil, errors.New("failed to read HSPICE file")
	}

	return &Result{handle: handle}, nil
}

// ReadDebug parses with debug output
func ReadDebug(filename string, debug int) (*Result, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	handle := C.hspice_read(cFilename, C.int(debug))
	if handle == nil {
		return nil, errors.New("failed to read HSPICE file")
	}

	return &Result{handle: handle}, nil
}

// Close releases the result resources
func (r *Result) Close() {
	if r.handle != nil {
		C.hspice_result_free(r.handle)
		r.handle = nil
	}
}

// Title returns the simulation title
func (r *Result) Title() string {
	return C.GoString(C.hspice_result_get_title(r.handle))
}

// Date returns the simulation date
func (r *Result) Date() string {
	return C.GoString(C.hspice_result_get_date(r.handle))
}

// ScaleName returns the scale name (e.g., "TIME", "FREQUENCY")
func (r *Result) ScaleName() string {
	return C.GoString(C.hspice_result_get_scale_name(r.handle))
}

// TableCount returns the number of data tables
func (r *Result) TableCount() int {
	return int(C.hspice_result_get_table_count(r.handle))
}

// HasSweep checks if the result has sweep data
func (r *Result) HasSweep() bool {
	return C.hspice_result_has_sweep(r.handle) != 0
}

// SweepName returns the sweep parameter name
func (r *Result) SweepName() string {
	ptr := C.hspice_result_get_sweep_name(r.handle)
	if ptr == nil {
		return ""
	}
	return C.GoString(ptr)
}

// SweepValues returns the sweep values
func (r *Result) SweepValues() []float64 {
	count := int(C.hspice_result_get_sweep_count(r.handle))
	if count == 0 {
		return nil
	}

	values := make([]float64, count)
	C.hspice_result_get_sweep_values(r.handle,
		(*C.double)(unsafe.Pointer(&values[0])), C.int(count))
	return values
}

// SignalCount returns the number of signals in a table
func (r *Result) SignalCount(tableIndex int) int {
	return int(C.hspice_result_get_signal_count(r.handle, C.int(tableIndex)))
}

// SignalLength returns the length of a signal
func (r *Result) SignalLength(tableIndex int, signalName string) int {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))
	return int(C.hspice_result_get_signal_length(r.handle, C.int(tableIndex), cName))
}

// IsComplex checks if a signal is complex
func (r *Result) IsComplex(tableIndex int, signalName string) bool {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))
	return C.hspice_result_signal_is_complex(r.handle, C.int(tableIndex), cName) == 1
}

// GetSignalReal gets real signal data
func (r *Result) GetSignalReal(tableIndex int, signalName string) ([]float64, error) {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))

	length := r.SignalLength(tableIndex, signalName)
	if length <= 0 {
		return nil, errors.New("signal not found or empty")
	}

	data := make([]float64, length)
	result := C.hspice_result_get_signal_real(r.handle, C.int(tableIndex), cName,
		(*C.double)(unsafe.Pointer(&data[0])), C.int(length))

	if result < 0 {
		return nil, errors.New("failed to get signal data")
	}

	return data, nil
}

// GetSignalComplex gets complex signal data
func (r *Result) GetSignalComplex(tableIndex int, signalName string) (real, imag []float64, err error) {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))

	length := r.SignalLength(tableIndex, signalName)
	if length <= 0 {
		return nil, nil, errors.New("signal not found or empty")
	}

	real = make([]float64, length)
	imag = make([]float64, length)

	result := C.hspice_result_get_signal_complex(r.handle, C.int(tableIndex), cName,
		(*C.double)(unsafe.Pointer(&real[0])),
		(*C.double)(unsafe.Pointer(&imag[0])),
		C.int(length))

	if result < 0 {
		return nil, nil, errors.New("failed to get complex signal data")
	}

	return real, imag, nil
}
```

### Streaming API

Add streaming support for large files:

```go
// Stream wraps the streaming reader
type Stream struct {
	handle *C.CHspiceStream
}

// OpenStream opens a file for streaming
func OpenStream(filename string, chunkSize int) (*Stream, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	handle := C.hspice_stream_open(cFilename, C.int(chunkSize), 0)
	if handle == nil {
		return nil, errors.New("failed to open stream")
	}

	return &Stream{handle: handle}, nil
}

// Close releases stream resources
func (s *Stream) Close() {
	if s.handle != nil {
		C.hspice_stream_close(s.handle)
		s.handle = nil
	}
}

// ScaleName returns the scale name
func (s *Stream) ScaleName() string {
	return C.GoString(C.hspice_stream_get_scale_name(s.handle))
}

// SignalCount returns the number of signals
func (s *Stream) SignalCount() int {
	return int(C.hspice_stream_get_signal_count(s.handle))
}

// SignalName returns a signal name by index
func (s *Stream) SignalName(index int) string {
	return C.GoString(C.hspice_stream_get_signal_name(s.handle, C.int(index)))
}

// Next reads the next chunk. Returns false at EOF.
func (s *Stream) Next() (bool, error) {
	result := C.hspice_stream_next(s.handle)
	switch result {
	case 1:
		return true, nil
	case 0:
		return false, nil
	default:
		return false, errors.New("stream read error")
	}
}

// ChunkSize returns the current chunk's point count
func (s *Stream) ChunkSize() int {
	return int(C.hspice_stream_get_chunk_size(s.handle))
}

// TimeRange returns the current chunk's time range
func (s *Stream) TimeRange() (start, end float64) {
	start = float64(C.hspice_stream_get_time_start(s.handle))
	end = float64(C.hspice_stream_get_time_end(s.handle))
	return
}

// GetChunkData gets signal data from the current chunk
func (s *Stream) GetChunkData(signalName string) ([]float64, error) {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))

	size := s.ChunkSize()
	if size <= 0 {
		return nil, errors.New("no data in current chunk")
	}

	data := make([]float64, size)
	result := C.hspice_stream_get_signal_data(s.handle, cName,
		(*C.double)(unsafe.Pointer(&data[0])), C.int(size))

	if result < 0 {
		return nil, errors.New("failed to get signal data")
	}

	return data, nil
}
```

## Usage Examples

### Basic Reading

```go
package main

import (
	"fmt"
	"log"

	"yourproject/hspice"
)

func main() {
	// Read HSPICE file
	result, err := hspice.Read("simulation.tr0")
	if err != nil {
		log.Fatal(err)
	}
	defer result.Close()

	// Print metadata
	fmt.Printf("Title: %s\n", result.Title())
	fmt.Printf("Date: %s\n", result.Date())
	fmt.Printf("Scale: %s\n", result.ScaleName())
	fmt.Printf("Tables: %d\n", result.TableCount())

	// Get TIME data
	time, err := result.GetSignalReal(0, "TIME")
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("Time points: %d\n", len(time))
	fmt.Printf("Range: %e to %e\n", time[0], time[len(time)-1])
}
```

### Processing Signals

```go
package main

import (
	"fmt"
	"log"
	"math"

	"yourproject/hspice"
)

func main() {
	result, err := hspice.Read("simulation.tr0")
	if err != nil {
		log.Fatal(err)
	}
	defer result.Close()

	// Get voltage signal
	vout, err := result.GetSignalReal(0, "v(out)")
	if err != nil {
		log.Fatal(err)
	}

	// Calculate min/max
	min, max := vout[0], vout[0]
	for _, v := range vout {
		if v < min {
			min = v
		}
		if v > max {
			max = v
		}
	}

	fmt.Printf("v(out) range: %.4f V to %.4f V\n", min, max)
	fmt.Printf("v(out) peak-to-peak: %.4f V\n", max-min)
}
```

### Streaming Large Files

```go
package main

import (
	"fmt"
	"log"

	"yourproject/hspice"
)

func main() {
	// Open stream with 50000 points per chunk
	stream, err := hspice.OpenStream("large_simulation.tr0", 50000)
	if err != nil {
		log.Fatal(err)
	}
	defer stream.Close()

	fmt.Printf("Scale: %s\n", stream.ScaleName())
	fmt.Printf("Signals: %d\n", stream.SignalCount())

	chunkNum := 0
	totalPoints := 0

	// Process chunks
	for {
		hasData, err := stream.Next()
		if err != nil {
			log.Fatal(err)
		}
		if !hasData {
			break
		}

		size := stream.ChunkSize()
		start, end := stream.TimeRange()

		fmt.Printf("Chunk %d: %d points, time %e to %e\n",
			chunkNum, size, start, end)

		// Get data from this chunk
		time, _ := stream.GetChunkData("TIME")
		vout, _ := stream.GetChunkData("v(out)")

		// Process data...
		_ = time
		_ = vout

		totalPoints += size
		chunkNum++
	}

	fmt.Printf("\nTotal: %d chunks, %d points\n", chunkNum, totalPoints)
}
```

## Project Structure

```
your-go-project/
├── go.mod
├── main.go
├── hspice/
│   ├── hspice.go              # Go wrapper
│   ├── hspice_tr0_parser.h    # C header (copy from include/)
│   └── libhspicetr0parser.a   # Static library (copy from target/release/)
```

## Building Your Go Project

```bash
# Standard build
go build

# Cross-compile (requires matching Rust target)
CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build
```

## Notes

- CGO adds overhead; for performance-critical applications, consider larger chunk sizes
- The static library must be compiled for the target platform
- Memory management is handled automatically via `Close()` methods
