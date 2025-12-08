/**
 * HSPICE TR0 Parser - C API Header
 *
 * High-performance library for reading HSPICE binary output files.
 *
 * Usage:
 *   1. Link with libhspicetr0parser.a (static) or libhspicetr0parser.so
 * (dynamic)
 *   2. Include this header
 *   3. Call hspice_read() to parse a file
 *   4. Use accessor functions to retrieve data
 *   5. Call hspice_result_free() when done
 *
 * Example:
 *   CHspiceResult* result = hspice_read("simulation.tr0", 0);
 *   if (result) {
 *       int count = hspice_result_get_signal_count(result, 0);
 *       printf("Found %d signals\n", count);
 *       hspice_result_free(result);
 *   }
 */

#ifndef HSPICE_TR0_PARSER_H
#define HSPICE_TR0_PARSER_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to HSPICE result */
typedef struct CHspiceResult CHspiceResult;

/* ============================================================================
 * Result Creation and Destruction
 * ============================================================================
 */

/**
 * Read an HSPICE binary file.
 *
 * @param filename Path to the HSPICE file (.tr0, .ac0, .sw0)
 * @param debug    Debug level (0=quiet, 1=info, 2=verbose)
 * @return         Pointer to result on success, NULL on error
 *
 * @note The caller must free the result using hspice_result_free().
 */
CHspiceResult *hspice_read(const char *filename, int debug);

/**
 * Free an HSPICE result handle.
 *
 * @param result Pointer returned by hspice_read()
 */
void hspice_result_free(CHspiceResult *result);

/* ============================================================================
 * Metadata Accessors
 * ============================================================================
 */

/**
 * Get the simulation title.
 *
 * @param result Result handle
 * @return       Null-terminated string (valid until result is freed)
 */
const char *hspice_result_get_title(const CHspiceResult *result);

/**
 * Get the simulation date.
 */
const char *hspice_result_get_date(const CHspiceResult *result);

/**
 * Get the scale name (e.g., "TIME" for transient analysis).
 */
const char *hspice_result_get_scale_name(const CHspiceResult *result);

/**
 * Get the number of data tables (one per sweep point).
 */
int hspice_result_get_table_count(const CHspiceResult *result);

/* ============================================================================
 * Sweep Accessors
 * ============================================================================
 */

/**
 * Check if the result has sweep data.
 *
 * @return 1 if has sweep, 0 otherwise
 */
int hspice_result_has_sweep(const CHspiceResult *result);

/**
 * Get the sweep parameter name.
 *
 * @return Null-terminated string, or NULL if no sweep
 */
const char *hspice_result_get_sweep_name(const CHspiceResult *result);

/**
 * Get the number of sweep values.
 */
int hspice_result_get_sweep_count(const CHspiceResult *result);

/**
 * Copy sweep values to an array.
 *
 * @param result     Result handle
 * @param out_values Output buffer for values
 * @param max_count  Maximum number of values to copy
 * @return           Number of values copied
 */
int hspice_result_get_sweep_values(const CHspiceResult *result,
                                   double *out_values, int max_count);

/* ============================================================================
 * Signal Data Accessors
 * ============================================================================
 */

/**
 * Get the number of signals in a data table.
 *
 * @param result      Result handle
 * @param table_index Index of the data table (0 for first/only table)
 */
int hspice_result_get_signal_count(const CHspiceResult *result,
                                   int table_index);

/**
 * Get signal names from a data table.
 *
 * @param result      Result handle
 * @param table_index Data table index
 * @param out_names   Array of char* to receive name pointers
 * @param max_count   Maximum number of names to retrieve
 * @return            Number of names copied
 *
 * @note The returned strings are valid until the result is freed.
 */
int hspice_result_get_signal_names(const CHspiceResult *result, int table_index,
                                   const char **out_names, int max_count);

/**
 * Get the length of a signal's data.
 *
 * @param result       Result handle
 * @param table_index  Data table index
 * @param signal_name  Name of the signal
 * @return             Number of data points, or 0 on error
 */
int hspice_result_get_signal_length(const CHspiceResult *result,
                                    int table_index, const char *signal_name);

/**
 * Check if signal data is complex.
 *
 * @return 1 if complex, 0 if real, -1 on error
 */
int hspice_result_signal_is_complex(const CHspiceResult *result,
                                    int table_index, const char *signal_name);

/**
 * Get real signal data.
 *
 * @param result       Result handle
 * @param table_index  Data table index
 * @param signal_name  Signal name
 * @param out_values   Output buffer for values
 * @param max_count    Maximum number of values to copy
 * @return             Number of values copied, or -1 on error
 */
int hspice_result_get_signal_real(const CHspiceResult *result, int table_index,
                                  const char *signal_name, double *out_values,
                                  int max_count);

/**
 * Get complex signal data (separate real and imaginary arrays).
 *
 * @param result       Result handle
 * @param table_index  Data table index
 * @param signal_name  Signal name
 * @param out_real     Output buffer for real parts
 * @param out_imag     Output buffer for imaginary parts
 * @param max_count    Maximum number of complex values to copy
 * @return             Number of complex values copied, or -1 on error
 */
int hspice_result_get_signal_complex(const CHspiceResult *result,
                                     int table_index, const char *signal_name,
                                     double *out_real, double *out_imag,
                                     int max_count);

#ifdef __cplusplus
}
#endif

#endif /* HSPICE_TR0_PARSER_H */
