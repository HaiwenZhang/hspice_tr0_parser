package com.hspice;

import com.sun.jna.Pointer;

/**
 * Main API class for parsing HSPICE and SPICE3 waveform files.
 * 
 * <p>Example usage:</p>
 * <pre>{@code
 * try (WaveformResult result = HspiceParser.read("simulation.tr0")) {
 *     System.out.println("Title: " + result.getTitle());
 *     System.out.println("Analysis: " + result.getAnalysis());
 *     
 *     double[] time = result.getRealData("TIME");
 *     double[] vout = result.getRealData("v(out)");
 * }
 * }</pre>
 */
public final class HspiceParser {
    
    private HspiceParser() {
        // Utility class, no instantiation
    }

    /**
     * Read an HSPICE waveform file.
     * 
     * @param filename Path to the file (.tr0, .ac0, .sw0)
     * @return WaveformResult containing the parsed data
     * @throws HspiceException if parsing fails
     */
    public static WaveformResult read(String filename) {
        return read(filename, 0);
    }

    /**
     * Read an HSPICE waveform file with debug output.
     * 
     * @param filename Path to the file
     * @param debug Debug level (0=quiet, 1=info, 2=verbose)
     * @return WaveformResult containing the parsed data
     * @throws HspiceException if parsing fails
     */
    public static WaveformResult read(String filename, int debug) {
        Pointer ptr = HspiceLibrary.INSTANCE.waveform_read(filename, debug);
        if (ptr == null) {
            throw new HspiceException("Failed to read file: " + filename);
        }
        return new WaveformResult(ptr);
    }

    /**
     * Read a SPICE3/ngspice raw file (auto-detects binary/ASCII format).
     * 
     * @param filename Path to the raw file
     * @return WaveformResult containing the parsed data
     * @throws HspiceException if parsing fails
     */
    public static WaveformResult readRaw(String filename) {
        return readRaw(filename, 0);
    }

    /**
     * Read a SPICE3/ngspice raw file with debug output.
     * 
     * @param filename Path to the raw file
     * @param debug Debug level
     * @return WaveformResult containing the parsed data
     * @throws HspiceException if parsing fails
     */
    public static WaveformResult readRaw(String filename, int debug) {
        Pointer ptr = HspiceLibrary.INSTANCE.waveform_read_raw(filename, debug);
        if (ptr == null) {
            throw new HspiceException("Failed to read raw file: " + filename);
        }
        return new WaveformResult(ptr);
    }

    /**
     * Check if the native library is available.
     * 
     * @return true if the library can be loaded
     */
    public static boolean isLibraryAvailable() {
        try {
            HspiceLibrary.INSTANCE.toString();
            return true;
        } catch (UnsatisfiedLinkError e) {
            return false;
        }
    }
}
