package com.hspice;

import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Platform;
import com.sun.jna.Pointer;

/**
 * JNA interface to the native HSPICE waveform parser library.
 */
public interface HspiceLibrary extends Library {
    
    /**
     * Get the library instance.
     * Loads libhspicetr0parser from the system library path.
     */
    HspiceLibrary INSTANCE = Native.load(
        Platform.isWindows() ? "hspicetr0parser" : "hspicetr0parser",
        HspiceLibrary.class
    );

    // ========================================================================
    // Result Creation and Destruction
    // ========================================================================

    /**
     * Read an HSPICE waveform file (.tr0, .ac0, .sw0).
     * @param filename Path to the file
     * @param debug Debug level (0=quiet, 1=info, 2=verbose)
     * @return Pointer to CWaveformResult, or null on error
     */
    Pointer waveform_read(String filename, int debug);

    /**
     * Read a SPICE3/ngspice raw file (auto-detects binary/ASCII).
     * @param filename Path to the raw file
     * @param debug Debug level
     * @return Pointer to CWaveformResult, or null on error
     */
    Pointer waveform_read_raw(String filename, int debug);

    /**
     * Free a waveform result.
     * @param result Pointer returned by waveform_read or waveform_read_raw
     */
    void waveform_free(Pointer result);

    // ========================================================================
    // Metadata Accessors
    // ========================================================================

    String waveform_get_title(Pointer result);
    String waveform_get_date(Pointer result);
    String waveform_get_scale_name(Pointer result);
    int waveform_get_analysis_type(Pointer result);
    int waveform_get_table_count(Pointer result);
    int waveform_get_var_count(Pointer result);
    int waveform_get_point_count(Pointer result);

    // ========================================================================
    // Variable Accessors
    // ========================================================================

    String waveform_get_var_name(Pointer result, int index);
    int waveform_get_var_type(Pointer result, int index);

    // ========================================================================
    // Sweep Data
    // ========================================================================

    int waveform_has_sweep(Pointer result);
    String waveform_get_sweep_param(Pointer result);
    double waveform_get_sweep_value(Pointer result, int tableIndex);

    // ========================================================================
    // Signal Data
    // ========================================================================

    int waveform_get_data_length(Pointer result, int tableIndex, int varIndex);
    int waveform_is_complex(Pointer result, int tableIndex, int varIndex);
    int waveform_get_real_data(Pointer result, int tableIndex, int varIndex,
                                double[] outBuffer, int maxCount);
    int waveform_get_complex_data(Pointer result, int tableIndex, int varIndex,
                                   double[] outReal, double[] outImag, int maxCount);
}
