# Go API Documentation

This document covers using the HSPICE parser from Go via CGO.

## Building the C Library

```bash
cargo build -p hspice-ffi --release
# Output: target/release/libhspicetr0parser.a
```

## Go Wrapper

Create a Go wrapper that uses CGO:

```go
package hspice

/*
#cgo LDFLAGS: -L${SRCDIR}/lib -lhspicetr0parser -lpthread -ldl -lm
#include "hspice_tr0_parser.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

// InitLogging initializes the logging subsystem with specified level.
// Call once at application startup.
// Levels: "trace", "debug", "info", "warn", "error"
func InitLogging(level string) error {
	cLevel := C.CString(level)
	defer C.free(unsafe.Pointer(cLevel))

	result := C.waveform_init_logging(cLevel)
	if result != 0 {
		return fmt.Errorf("failed to initialize logging with level %s", level)
	}
	return nil
}

// WaveformResult represents parsed waveform data
type WaveformResult struct {
	ptr *C.CWaveformResult
}

// Read parses an HSPICE binary file
func Read(filename string) (*WaveformResult, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	result := C.waveform_read(cFilename, 0)
	if result == nil {
		return nil, fmt.Errorf("failed to read %s", filename)
	}

	return &WaveformResult{ptr: result}, nil
}

// ReadRaw parses a SPICE3/ngspice raw file (auto-detects binary/ASCII)
func ReadRaw(filename string) (*WaveformResult, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	result := C.waveform_read_raw(cFilename, 0)
	if result == nil {
		return nil, fmt.Errorf("failed to read raw file %s", filename)
	}

	return &WaveformResult{ptr: result}, nil
}

// Close frees the result
func (r *WaveformResult) Close() {
	if r.ptr != nil {
		C.waveform_free(r.ptr)
		r.ptr = nil
	}
}

// Title returns the simulation title
func (r *WaveformResult) Title() string {
	return C.GoString(C.waveform_get_title(r.ptr))
}

// Date returns the simulation date
func (r *WaveformResult) Date() string {
	return C.GoString(C.waveform_get_date(r.ptr))
}

// ScaleName returns the scale variable name
func (r *WaveformResult) ScaleName() string {
	return C.GoString(C.waveform_get_scale_name(r.ptr))
}

// AnalysisType returns the analysis type
func (r *WaveformResult) AnalysisType() int {
	return int(C.waveform_get_analysis_type(r.ptr))
}

// VarCount returns the number of variables
func (r *WaveformResult) VarCount() int {
	return int(C.waveform_get_var_count(r.ptr))
}

// PointCount returns the number of data points
func (r *WaveformResult) PointCount() int {
	return int(C.waveform_get_point_count(r.ptr))
}

// TableCount returns the number of sweep tables
func (r *WaveformResult) TableCount() int {
	return int(C.waveform_get_table_count(r.ptr))
}

// VarName returns the variable name at index
func (r *WaveformResult) VarName(index int) string {
	return C.GoString(C.waveform_get_var_name(r.ptr, C.int(index)))
}

// VarType returns the variable type at index
func (r *WaveformResult) VarType(index int) int {
	return int(C.waveform_get_var_type(r.ptr, C.int(index)))
}

// HasSweep returns whether the result has sweep data
func (r *WaveformResult) HasSweep() bool {
	return C.waveform_has_sweep(r.ptr) != 0
}

// SweepParam returns the sweep parameter name
func (r *WaveformResult) SweepParam() string {
	return C.GoString(C.waveform_get_sweep_param(r.ptr))
}

// SweepValue returns the sweep value for a table
func (r *WaveformResult) SweepValue(tableIndex int) float64 {
	return float64(C.waveform_get_sweep_value(r.ptr, C.int(tableIndex)))
}

// IsComplex returns whether a variable contains complex data
func (r *WaveformResult) IsComplex(tableIndex, varIndex int) bool {
	return C.waveform_is_complex(r.ptr, C.int(tableIndex), C.int(varIndex)) == 1
}

// GetRealData returns real data for a variable
func (r *WaveformResult) GetRealData(tableIndex, varIndex int) []float64 {
	length := C.waveform_get_data_length(r.ptr, C.int(tableIndex), C.int(varIndex))
	if length <= 0 {
		return nil
	}

	data := make([]float64, length)
	C.waveform_get_real_data(r.ptr, C.int(tableIndex), C.int(varIndex),
		(*C.double)(unsafe.Pointer(&data[0])), length)

	return data
}

// GetComplexData returns complex data for a variable as (real, imag) slices
func (r *WaveformResult) GetComplexData(tableIndex, varIndex int) ([]float64, []float64) {
	length := C.waveform_get_data_length(r.ptr, C.int(tableIndex), C.int(varIndex))
	if length <= 0 {
		return nil, nil
	}

	real := make([]float64, length)
	imag := make([]float64, length)
	C.waveform_get_complex_data(r.ptr, C.int(tableIndex), C.int(varIndex),
		(*C.double)(unsafe.Pointer(&real[0])),
		(*C.double)(unsafe.Pointer(&imag[0])),
		length)

	return real, imag
}

// Analysis type constants
const (
	AnalysisTransient = 0
	AnalysisAC        = 1
	AnalysisDC        = 2
	AnalysisOperating = 3
	AnalysisNoise     = 4
)

// Variable type constants
const (
	VarTime      = 0
	VarFrequency = 1
	VarVoltage   = 2
	VarCurrent   = 3
)
```

## Logging

The library uses structured logging via `tracing`. To enable log output, call `InitLogging()` before using other functions:

```go
package main

import (
	"log"
	"yourproject/hspice"
)

func main() {
	// Initialize logging with desired level
	// Levels: "trace", "debug", "info", "warn", "error"
	if err := hspice.InitLogging("info"); err != nil {
		log.Printf("Warning: %v", err)
	}

	// Now all operations will output logs
	result, err := hspice.Read("simulation.tr0")
	if err != nil {
		log.Fatal(err)
	}
	defer result.Close()
	// ...
}
```

### Log Levels

| Level   | Description                                                |
| ------- | ---------------------------------------------------------- |
| `trace` | Most verbose, includes per-chunk and per-sweep details     |
| `debug` | Detailed info: file sizes, data block statistics           |
| `info`  | Key operations: file open, parse complete, conversion done |
| `warn`  | Warnings only                                              |
| `error` | Errors only (default if not initialized)                   |

## Usage Example

```go
package main

import (
	"fmt"
	"log"

	"yourproject/hspice"
)

func main() {
	// Enable info-level logging
	hspice.InitLogging("info")

	result, err := hspice.Read("simulation.tr0")
	if err != nil {
		log.Fatal(err)
	}
	defer result.Close()

	fmt.Printf("Title: %s\n", result.Title())
	fmt.Printf("Date: %s\n", result.Date())
	fmt.Printf("Scale: %s\n", result.ScaleName())
	fmt.Printf("Variables: %d\n", result.VarCount())
	fmt.Printf("Points: %d\n", result.PointCount())

	// List variables
	for i := 0; i < result.VarCount(); i++ {
		fmt.Printf("  %s (type=%d)\n", result.VarName(i), result.VarType(i))
	}

	// Get time data
	time := result.GetRealData(0, 0)
	if time != nil {
		fmt.Printf("Time: %e to %e\n", time[0], time[len(time)-1])
	}

	// Check for sweep data
	if result.HasSweep() {
		fmt.Printf("Sweep parameter: %s\n", result.SweepParam())
		for i := 0; i < result.TableCount(); i++ {
			fmt.Printf("  Table %d: sweep=%f\n", i, result.SweepValue(i))
		}
	}
}
```

## Streaming Large Files

```go
package hspice

/*
#cgo LDFLAGS: -L${SRCDIR}/lib -lhspicetr0parser -lpthread -ldl -lm
#include "hspice_tr0_parser.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

// WaveformStream for reading large files in chunks
type WaveformStream struct {
	ptr *C.CWaveformStream
}

// OpenStream opens a file for streaming
func OpenStream(filename string, chunkSize int) (*WaveformStream, error) {
	cFilename := C.CString(filename)
	defer C.free(unsafe.Pointer(cFilename))

	stream := C.waveform_stream_open(cFilename, C.int(chunkSize), 0)
	if stream == nil {
		return nil, fmt.Errorf("failed to open stream for %s", filename)
	}

	return &WaveformStream{ptr: stream}, nil
}

// Close closes the stream
func (s *WaveformStream) Close() {
	if s.ptr != nil {
		C.waveform_stream_close(s.ptr)
		s.ptr = nil
	}
}

// Next reads the next chunk. Returns true if chunk available, false if end of file.
func (s *WaveformStream) Next() (bool, error) {
	result := C.waveform_stream_next(s.ptr)
	if result < 0 {
		return false, fmt.Errorf("stream error")
	}
	return result == 1, nil
}

// ChunkSize returns the size of the current chunk
func (s *WaveformStream) ChunkSize() int {
	return int(C.waveform_stream_get_chunk_size(s.ptr))
}

// TimeRange returns the time range of the current chunk
func (s *WaveformStream) TimeRange() (float64, float64) {
	var start, end C.double
	C.waveform_stream_get_time_range(s.ptr, &start, &end)
	return float64(start), float64(end)
}

// GetSignalData returns signal data from the current chunk
func (s *WaveformStream) GetSignalData(signalName string) []float64 {
	cName := C.CString(signalName)
	defer C.free(unsafe.Pointer(cName))

	size := s.ChunkSize()
	if size <= 0 {
		return nil
	}

	data := make([]float64, size)
	copied := C.waveform_stream_get_signal_data(s.ptr, cName,
		(*C.double)(unsafe.Pointer(&data[0])), C.int(size))

	if copied <= 0 {
		return nil
	}
	return data[:copied]
}
```

### Streaming Usage Example

```go
package main

import (
	"fmt"
	"log"

	"yourproject/hspice"
)

func main() {
	hspice.InitLogging("info")

	stream, err := hspice.OpenStream("large_sim.tr0", 100000)
	if err != nil {
		log.Fatal(err)
	}
	defer stream.Close()

	totalPoints := 0
	for {
		hasChunk, err := stream.Next()
		if err != nil {
			log.Fatal(err)
		}
		if !hasChunk {
			break
		}

		start, end := stream.TimeRange()
		fmt.Printf("Chunk: time=%e to %e, points=%d\n", start, end, stream.ChunkSize())

		time := stream.GetSignalData("TIME")
		totalPoints += len(time)
	}

	fmt.Printf("Total points processed: %d\n", totalPoints)
}
```

## Project Structure

```
myproject/
├── hspice/
│   ├── hspice.go           # Go wrapper
│   ├── stream.go           # Streaming wrapper
│   └── lib/
│       └── libhspicetr0parser.a
├── include/
│   └── hspice_tr0_parser.h
└── main.go
```

## Building

```bash
# Copy library and header
mkdir -p hspice/lib
cp target/release/libhspicetr0parser.a hspice/lib/
cp include/hspice_tr0_parser.h .

# Build Go project
CGO_ENABLED=1 go build
```

## Migration from v1.3.x

Add `InitLogging()` call for log output:

```go
// Old (v1.3.x) - no structured logging
result, _ := hspice.Read("file.tr0")

// New (v1.4.0+) - use InitLogging for control
hspice.InitLogging("info")
result, _ := hspice.Read("file.tr0")
```
