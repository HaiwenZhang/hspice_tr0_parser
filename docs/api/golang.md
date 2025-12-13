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

// VarName returns the variable name at index
func (r *WaveformResult) VarName(index int) string {
	return C.GoString(C.waveform_get_var_name(r.ptr, C.int(index)))
}

// VarType returns the variable type at index
func (r *WaveformResult) VarType(index int) int {
	return int(C.waveform_get_var_type(r.ptr, C.int(index)))
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

// Analysis type constants
const (
	AnalysisTransient = 0
	AnalysisAC        = 1
	AnalysisDC        = 2
)

// Variable type constants
const (
	VarTime      = 0
	VarFrequency = 1
	VarVoltage   = 2
	VarCurrent   = 3
)
```

## Usage Example

```go
package main

import (
	"fmt"
	"log"

	"yourproject/hspice"
)

func main() {
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
}
```

## Project Structure

```
myproject/
├── hspice/
│   ├── hspice.go           # Go wrapper
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
