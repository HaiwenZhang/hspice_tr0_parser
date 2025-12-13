# C/C++ API Documentation

This document covers the C FFI for `hspice-ffi`.

## Building

```bash
cargo build -p hspice-ffi --release
# Output: target/release/libhspicetr0parser.a (static)
# Output: target/release/libhspicetr0parser.so (dynamic)
```

## Header File

Include `include/hspice_tr0_parser.h` in your project.

## API Reference

### Result Management

```c
// Read a waveform file
CWaveformResult* waveform_read(const char* filename, int debug);

// Free result
void waveform_free(CWaveformResult* result);
```

### Metadata

```c
const char* waveform_get_title(const CWaveformResult* result);
const char* waveform_get_date(const CWaveformResult* result);
const char* waveform_get_scale_name(const CWaveformResult* result);
int waveform_get_analysis_type(const CWaveformResult* result);
int waveform_get_table_count(const CWaveformResult* result);
int waveform_get_var_count(const CWaveformResult* result);
int waveform_get_point_count(const CWaveformResult* result);
```

### Variables

```c
const char* waveform_get_var_name(const CWaveformResult* result, int index);
int waveform_get_var_type(const CWaveformResult* result, int index);
```

### Sweep Data

```c
int waveform_has_sweep(const CWaveformResult* result);
const char* waveform_get_sweep_param(const CWaveformResult* result);
double waveform_get_sweep_value(const CWaveformResult* result, int table_index);
```

### Signal Data

```c
int waveform_get_data_length(const CWaveformResult* result,
                              int table_index, int var_index);

int waveform_is_complex(const CWaveformResult* result,
                         int table_index, int var_index);

int waveform_get_real_data(const CWaveformResult* result,
                            int table_index, int var_index,
                            double* out_buffer, int max_count);

int waveform_get_complex_data(const CWaveformResult* result,
                               int table_index, int var_index,
                               double* out_real, double* out_imag,
                               int max_count);
```

## Constants

```c
// Analysis types
#define WAVEFORM_ANALYSIS_TRANSIENT  0
#define WAVEFORM_ANALYSIS_AC         1
#define WAVEFORM_ANALYSIS_DC         2

// Variable types
#define WAVEFORM_VAR_TIME       0
#define WAVEFORM_VAR_FREQUENCY  1
#define WAVEFORM_VAR_VOLTAGE    2
#define WAVEFORM_VAR_CURRENT    3
```

## Complete Example

```c
#include <stdio.h>
#include <stdlib.h>
#include "hspice_tr0_parser.h"

int main() {
    CWaveformResult* result = waveform_read("simulation.tr0", 0);
    if (!result) {
        fprintf(stderr, "Failed to read file\n");
        return 1;
    }

    // Print metadata
    printf("Title: %s\n", waveform_get_title(result));
    printf("Date: %s\n", waveform_get_date(result));
    printf("Scale: %s\n", waveform_get_scale_name(result));
    printf("Analysis: %d\n", waveform_get_analysis_type(result));

    // Print variables
    int num_vars = waveform_get_var_count(result);
    int num_points = waveform_get_point_count(result);
    printf("Variables: %d, Points: %d\n", num_vars, num_points);

    for (int i = 0; i < num_vars; i++) {
        printf("  %s (type=%d)\n",
            waveform_get_var_name(result, i),
            waveform_get_var_type(result, i));
    }

    // Get time data (first variable, index 0)
    double* time = malloc(num_points * sizeof(double));
    int copied = waveform_get_real_data(result, 0, 0, time, num_points);

    printf("Time range: %e to %e\n", time[0], time[copied-1]);

    free(time);
    waveform_free(result);
    return 0;
}
```

## Compilation

```bash
# Static linking
gcc main.c -I./include -L./target/release -lhspicetr0parser -lpthread -ldl -lm -o app

# Dynamic linking
gcc main.c -I./include -L./target/release -lhspicetr0parser -o app
export LD_LIBRARY_PATH=./target/release:$LD_LIBRARY_PATH
```

## C++ Usage

```cpp
#include <iostream>
#include <vector>
#include "hspice_tr0_parser.h"

int main() {
    auto result = waveform_read("simulation.tr0", 0);
    if (!result) return 1;

    std::cout << "Title: " << waveform_get_title(result) << "\n";

    int n = waveform_get_point_count(result);
    std::vector<double> time(n);
    waveform_get_real_data(result, 0, 0, time.data(), n);

    waveform_free(result);
    return 0;
}
```
