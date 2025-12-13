/**
 * Waveform Parser - C API Header
 *
 * High-performance library for reading HSPICE binary waveform files.
 *
 * Usage:
 *   1. Link with libhspicetr0parser.a (static) or libhspicetr0parser.so
 * (dynamic)
 *   2. Include this header
 *   3. Call waveform_read() to parse a file
 *   4. Use accessor functions to retrieve data
 *   5. Call waveform_free() when done
 *
 * Example:
 *   CWaveformResult* result = waveform_read("simulation.tr0", 0);
 *   if (result) {
 *       printf("Title: %s\n", waveform_get_title(result));
 *       printf("Variables: %d\n", waveform_get_var_count(result));
 *       printf("Points: %d\n", waveform_get_point_count(result));
 *
 *       // Get signal data
 *       int len = waveform_get_data_length(result, 0, 0);
 *       double* data = malloc(len * sizeof(double));
 *       waveform_get_real_data(result, 0, 0, data, len);
 *
 *       waveform_free(result);
 *   }
 */

#ifndef WAVEFORM_PARSER_H
#define WAVEFORM_PARSER_H

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Opaque Types
 * ============================================================================
 */

/** Opaque handle to waveform result */
typedef struct CWaveformResult CWaveformResult;

/** Opaque handle to streaming reader */
typedef struct CWaveformStream CWaveformStream;

/* ============================================================================
 * Analysis and Variable Types
 * ============================================================================
 */

/** Analysis type constants */
#define WAVEFORM_ANALYSIS_TRANSIENT 0
#define WAVEFORM_ANALYSIS_AC 1
#define WAVEFORM_ANALYSIS_DC 2
#define WAVEFORM_ANALYSIS_OPERATING 3
#define WAVEFORM_ANALYSIS_NOISE 4
#define WAVEFORM_ANALYSIS_UNKNOWN -1

/** Variable type constants */
#define WAVEFORM_VAR_TIME 0
#define WAVEFORM_VAR_FREQUENCY 1
#define WAVEFORM_VAR_VOLTAGE 2
#define WAVEFORM_VAR_CURRENT 3
#define WAVEFORM_VAR_UNKNOWN -1

/* ============================================================================
 * Result Creation and Destruction
 * ============================================================================
 */

/**
 * Read a waveform file.
 *
 * @param filename Path to the waveform file (.tr0, .ac0, .sw0)
 * @param debug    Debug level (0=quiet, 1=info, 2=verbose)
 * @return         Pointer to result on success, NULL on error
 *
 * @note The caller must free the result using waveform_free().
 */
CWaveformResult *waveform_read(const char *filename, int debug);

/**
 * Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).
 *
 * @param filename Path to the raw file (.raw)
 * @param debug    Debug level (0=quiet, 1=info, 2=verbose)
 * @return         Pointer to result on success, NULL on error
 *
 * @note The caller must free the result using waveform_free().
 */
CWaveformResult *waveform_read_raw(const char *filename, int debug);

/**
 * Free a waveform result handle.
 *
 * @param result Pointer returned by waveform_read() or waveform_read_raw()
 */
void waveform_free(CWaveformResult *result);

/* ============================================================================
 * Metadata Accessors
 * ============================================================================
 */

/** Get the simulation title. */
const char *waveform_get_title(const CWaveformResult *result);

/** Get the simulation date. */
const char *waveform_get_date(const CWaveformResult *result);

/** Get the scale name (e.g., "TIME", "HERTZ"). */
const char *waveform_get_scale_name(const CWaveformResult *result);

/** Get the analysis type (WAVEFORM_ANALYSIS_*). */
int waveform_get_analysis_type(const CWaveformResult *result);

/** Get the number of data tables (one per sweep point). */
int waveform_get_table_count(const CWaveformResult *result);

/** Get the number of variables/signals. */
int waveform_get_var_count(const CWaveformResult *result);

/** Get the number of data points in the first table. */
int waveform_get_point_count(const CWaveformResult *result);

/* ============================================================================
 * Variable Accessors
 * ============================================================================
 */

/**
 * Get variable name by index.
 *
 * @param result Result handle
 * @param index  Variable index (0-based)
 * @return       Null-terminated string, or NULL on error
 */
const char *waveform_get_var_name(const CWaveformResult *result, int index);

/**
 * Get variable type by index.
 *
 * @param result Result handle
 * @param index  Variable index (0-based)
 * @return       WAVEFORM_VAR_* constant, or -1 on error
 */
int waveform_get_var_type(const CWaveformResult *result, int index);

/* ============================================================================
 * Sweep Accessors
 * ============================================================================
 */

/**
 * Check if the result has sweep data.
 *
 * @return 1 if has sweep, 0 otherwise
 */
int waveform_has_sweep(const CWaveformResult *result);

/**
 * Get the sweep parameter name.
 *
 * @return Null-terminated string, or NULL if no sweep
 */
const char *waveform_get_sweep_param(const CWaveformResult *result);

/**
 * Get the sweep value for a specific table.
 *
 * @param result      Result handle
 * @param table_index Table index (0-based)
 * @return            Sweep value, or 0.0 on error
 */
double waveform_get_sweep_value(const CWaveformResult *result, int table_index);

/* ============================================================================
 * Data Accessors
 * ============================================================================
 */

/**
 * Get the length of data for a variable.
 *
 * @param result      Result handle
 * @param table_index Table index (0-based)
 * @param var_index   Variable index (0-based)
 * @return            Number of data points, or 0 on error
 */
int waveform_get_data_length(const CWaveformResult *result, int table_index,
                             int var_index);

/**
 * Check if data is complex.
 *
 * @return 1 if complex, 0 if real, -1 on error
 */
int waveform_is_complex(const CWaveformResult *result, int table_index,
                        int var_index);

/**
 * Get real data for a variable.
 *
 * @param result      Result handle
 * @param table_index Table index (0-based)
 * @param var_index   Variable index (0-based)
 * @param out_buffer  Output buffer for values
 * @param max_count   Maximum number of values to copy
 * @return            Number of values copied, or -1 on error
 */
int waveform_get_real_data(const CWaveformResult *result, int table_index,
                           int var_index, double *out_buffer, int max_count);

/**
 * Get complex data for a variable (separate real and imaginary arrays).
 *
 * @param result      Result handle
 * @param table_index Table index (0-based)
 * @param var_index   Variable index (0-based)
 * @param out_real    Output buffer for real parts
 * @param out_imag    Output buffer for imaginary parts
 * @param max_count   Maximum number of complex values to copy
 * @return            Number of values copied, or -1 on error
 */
int waveform_get_complex_data(const CWaveformResult *result, int table_index,
                              int var_index, double *out_real, double *out_imag,
                              int max_count);

/* ============================================================================
 * Streaming API
 * ============================================================================
 */

/**
 * Open a file for streaming read.
 *
 * @param filename   Path to the waveform file
 * @param chunk_size Minimum points per chunk
 * @param debug      Debug level
 * @return           Stream handle, or NULL on error
 */
CWaveformStream *waveform_stream_open(const char *filename, int chunk_size,
                                      int debug);

/** Close a streaming reader. */
void waveform_stream_close(CWaveformStream *stream);

/**
 * Read the next chunk.
 *
 * @return 1 if success, 0 if EOF, -1 on error
 */
int waveform_stream_next(CWaveformStream *stream);

/** Get the current chunk's point count. */
int waveform_stream_get_chunk_size(const CWaveformStream *stream);

/**
 * Get the current chunk's time range.
 *
 * @param stream    Stream handle
 * @param out_start Output for start time
 * @param out_end   Output for end time
 * @return          0 on success, -1 on error
 */
int waveform_stream_get_time_range(const CWaveformStream *stream,
                                   double *out_start, double *out_end);

/**
 * Get signal data from the current chunk.
 *
 * @param stream      Stream handle
 * @param signal_name Name of the signal
 * @param out_buffer  Output buffer for values
 * @param max_count   Maximum number of values to copy
 * @return            Number of values copied, or -1 on error
 */
int waveform_stream_get_signal_data(const CWaveformStream *stream,
                                    const char *signal_name, double *out_buffer,
                                    int max_count);

/* ============================================================================
 * Legacy API Aliases (for backward compatibility)
 * ============================================================================
 */

#define hspice_read waveform_read
#define hspice_result_free waveform_free

#ifdef __cplusplus
}
#endif

#endif /* WAVEFORM_PARSER_H */
