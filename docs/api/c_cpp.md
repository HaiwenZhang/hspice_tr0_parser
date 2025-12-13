# C/C++ API Documentation

This document covers the C FFI API for `hspice-ffi`, enabling HSPICE file parsing from C, C++, and other languages with C FFI support.

## Building

### Build Static Library

```bash
# Clone the repository
git clone https://github.com/HaiwenZhang/hspice_tr0_parser.git
cd hspice_tr0_parser

# Build the static library
cargo build -p hspice-ffi --release

# Output: target/release/libhspicetr0parser.a
```

### Header File

The C header is located at `include/hspice_tr0_parser.h`.

## Linking

### GCC/Clang

```bash
gcc -o myprogram myprogram.c \
    -I./include \
    -L./target/release \
    -lhspicetr0parser
```

### CMake

```cmake
add_executable(myprogram main.c)
target_include_directories(myprogram PRIVATE ${PROJECT_SOURCE_DIR}/include)
target_link_directories(myprogram PRIVATE ${PROJECT_SOURCE_DIR}/target/release)
target_link_libraries(myprogram hspicetr0parser)
```

## API Reference

### Result Functions

#### `hspice_read(filename, debug)`

Read an HSPICE binary file.

```c
CHspiceResult* hspice_read(const char* filename, int debug);
```

**Parameters:**

- `filename`: Path to the HSPICE file (null-terminated)
- `debug`: Debug level (0=quiet, 1=info, 2=verbose)

**Returns:** Pointer to result handle, or NULL on error.

#### `hspice_result_free(result)`

Free a result handle.

```c
void hspice_result_free(CHspiceResult* result);
```

### Metadata Accessors

```c
const char* hspice_result_get_title(const CHspiceResult* result);
const char* hspice_result_get_date(const CHspiceResult* result);
const char* hspice_result_get_scale_name(const CHspiceResult* result);
int hspice_result_get_table_count(const CHspiceResult* result);
```

### Sweep Accessors

```c
int hspice_result_has_sweep(const CHspiceResult* result);
const char* hspice_result_get_sweep_name(const CHspiceResult* result);
int hspice_result_get_sweep_count(const CHspiceResult* result);
int hspice_result_get_sweep_values(const CHspiceResult* result,
                                    double* out_values, int max_count);
```

### Signal Data Accessors

```c
int hspice_result_get_signal_count(const CHspiceResult* result, int table_index);

int hspice_result_get_signal_names(const CHspiceResult* result, int table_index,
                                    const char** out_names, int max_count);

int hspice_result_get_signal_length(const CHspiceResult* result, int table_index,
                                     const char* signal_name);

int hspice_result_signal_is_complex(const CHspiceResult* result, int table_index,
                                     const char* signal_name);

int hspice_result_get_signal_real(const CHspiceResult* result, int table_index,
                                   const char* signal_name,
                                   double* out_values, int max_count);

int hspice_result_get_signal_complex(const CHspiceResult* result, int table_index,
                                      const char* signal_name,
                                      double* out_real, double* out_imag,
                                      int max_count);
```

### Streaming API

```c
CHspiceStream* hspice_stream_open(const char* filename, int chunk_size, int debug);
void hspice_stream_close(CHspiceStream* stream);

const char* hspice_stream_get_scale_name(const CHspiceStream* stream);
int hspice_stream_get_signal_count(const CHspiceStream* stream);
const char* hspice_stream_get_signal_name(const CHspiceStream* stream, int index);

int hspice_stream_next(CHspiceStream* stream);  // Returns: 1=success, 0=EOF, -1=error
int hspice_stream_get_chunk_size(const CHspiceStream* stream);
double hspice_stream_get_time_start(const CHspiceStream* stream);
double hspice_stream_get_time_end(const CHspiceStream* stream);
int hspice_stream_get_signal_data(const CHspiceStream* stream, const char* signal_name,
                                   double* out_buffer, int max_count);
```

## Complete Examples

### Basic Reading

```c
#include "hspice_tr0_parser.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char* argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <filename.tr0>\n", argv[0]);
        return 1;
    }

    // Read HSPICE file
    CHspiceResult* result = hspice_read(argv[1], 0);
    if (!result) {
        fprintf(stderr, "Failed to read file\n");
        return 1;
    }

    // Print metadata
    printf("Title: %s\n", hspice_result_get_title(result));
    printf("Date: %s\n", hspice_result_get_date(result));
    printf("Scale: %s\n", hspice_result_get_scale_name(result));
    printf("Tables: %d\n", hspice_result_get_table_count(result));

    // Get signal count in first table
    int signal_count = hspice_result_get_signal_count(result, 0);
    printf("Signals in table 0: %d\n", signal_count);

    // Get TIME data
    int time_len = hspice_result_get_signal_length(result, 0, "TIME");
    if (time_len > 0) {
        double* time = malloc(time_len * sizeof(double));
        hspice_result_get_signal_real(result, 0, "TIME", time, time_len);

        printf("Time points: %d\n", time_len);
        printf("First: %e, Last: %e\n", time[0], time[time_len - 1]);

        free(time);
    }

    // Cleanup
    hspice_result_free(result);
    return 0;
}
```

### Reading Multiple Signals

```c
#include "hspice_tr0_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void print_signal_info(CHspiceResult* result, int table, const char* name) {
    int len = hspice_result_get_signal_length(result, table, name);
    int is_complex = hspice_result_signal_is_complex(result, table, name);

    printf("  %s: %d points (%s)\n", name, len,
           is_complex ? "complex" : "real");

    if (len > 0 && !is_complex) {
        double* data = malloc(len * sizeof(double));
        hspice_result_get_signal_real(result, table, name, data, len);
        printf("    Range: [%e, %e]\n", data[0], data[len-1]);
        free(data);
    }
}

int main(int argc, char* argv[]) {
    CHspiceResult* result = hspice_read(argv[1], 0);
    if (!result) return 1;

    int table_count = hspice_result_get_table_count(result);

    for (int t = 0; t < table_count; t++) {
        printf("\n=== Table %d ===\n", t);

        int signal_count = hspice_result_get_signal_count(result, t);
        const char** names = malloc(signal_count * sizeof(char*));
        hspice_result_get_signal_names(result, t, names, signal_count);

        for (int s = 0; s < signal_count; s++) {
            print_signal_info(result, t, names[s]);
        }

        free(names);
    }

    hspice_result_free(result);
    return 0;
}
```

### Streaming Large Files

```c
#include "hspice_tr0_parser.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char* argv[]) {
    // Open stream with 10000 points per chunk
    CHspiceStream* stream = hspice_stream_open(argv[1], 10000, 0);
    if (!stream) {
        fprintf(stderr, "Failed to open stream\n");
        return 1;
    }

    printf("Scale: %s\n", hspice_stream_get_scale_name(stream));
    printf("Signals: %d\n", hspice_stream_get_signal_count(stream));

    int chunk_num = 0;
    int total_points = 0;

    // Read chunks
    while (hspice_stream_next(stream) == 1) {
        int chunk_size = hspice_stream_get_chunk_size(stream);
        double t_start = hspice_stream_get_time_start(stream);
        double t_end = hspice_stream_get_time_end(stream);

        printf("Chunk %d: %d points, time %e to %e\n",
               chunk_num, chunk_size, t_start, t_end);

        // Get TIME data from this chunk
        double* time = malloc(chunk_size * sizeof(double));
        hspice_stream_get_signal_data(stream, "TIME", time, chunk_size);

        // Process data...

        free(time);
        total_points += chunk_size;
        chunk_num++;
    }

    printf("\nTotal: %d chunks, %d points\n", chunk_num, total_points);

    hspice_stream_close(stream);
    return 0;
}
```

### C++ Wrapper Example

```cpp
#include "hspice_tr0_parser.h"
#include <iostream>
#include <vector>
#include <string>
#include <memory>

// RAII wrapper for CHspiceResult
class HspiceResult {
public:
    explicit HspiceResult(const std::string& filename, int debug = 0)
        : handle_(hspice_read(filename.c_str(), debug)) {}

    ~HspiceResult() {
        if (handle_) hspice_result_free(handle_);
    }

    // Non-copyable
    HspiceResult(const HspiceResult&) = delete;
    HspiceResult& operator=(const HspiceResult&) = delete;

    // Movable
    HspiceResult(HspiceResult&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }

    bool valid() const { return handle_ != nullptr; }

    std::string title() const { return hspice_result_get_title(handle_); }
    std::string date() const { return hspice_result_get_date(handle_); }
    std::string scale_name() const { return hspice_result_get_scale_name(handle_); }
    int table_count() const { return hspice_result_get_table_count(handle_); }

    std::vector<double> get_signal(int table, const std::string& name) const {
        int len = hspice_result_get_signal_length(handle_, table, name.c_str());
        std::vector<double> data(len);
        hspice_result_get_signal_real(handle_, table, name.c_str(), data.data(), len);
        return data;
    }

private:
    CHspiceResult* handle_;
};

int main(int argc, char* argv[]) {
    HspiceResult result(argv[1]);
    if (!result.valid()) {
        std::cerr << "Failed to read file\n";
        return 1;
    }

    std::cout << "Title: " << result.title() << "\n";
    std::cout << "Scale: " << result.scale_name() << "\n";

    auto time = result.get_signal(0, "TIME");
    std::cout << "Time points: " << time.size() << "\n";
    if (!time.empty()) {
        std::cout << "Range: " << time.front() << " to " << time.back() << "\n";
    }

    return 0;
}
```

## Error Handling

- Functions returning pointers return `NULL` on error
- Functions returning `int` for counts return `0` or negative on error
- Use `debug > 0` to get error messages on stderr

## Memory Management

- Call `hspice_result_free()` to release `CHspiceResult*`
- Call `hspice_stream_close()` to release `CHspiceStream*`
- String pointers returned by getters are valid until the parent object is freed
- Caller must allocate and free buffers passed to `get_signal_*` functions
