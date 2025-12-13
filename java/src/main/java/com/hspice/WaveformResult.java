package com.hspice;

import com.sun.jna.Pointer;
import java.util.ArrayList;
import java.util.List;

/**
 * Represents the result of parsing a waveform file.
 * Contains metadata and signal data from HSPICE or SPICE3 simulations.
 */
public class WaveformResult implements AutoCloseable {
    private Pointer nativePtr;
    private final HspiceLibrary lib;
    
    // Cached metadata
    private String title;
    private String date;
    private String scaleName;
    private AnalysisType analysis;
    private List<Variable> variables;
    private int numPoints;
    private boolean closed = false;

    WaveformResult(Pointer nativePtr) {
        this.nativePtr = nativePtr;
        this.lib = HspiceLibrary.INSTANCE;
        loadMetadata();
    }

    private void loadMetadata() {
        this.title = lib.waveform_get_title(nativePtr);
        this.date = lib.waveform_get_date(nativePtr);
        this.scaleName = lib.waveform_get_scale_name(nativePtr);
        this.analysis = AnalysisType.fromCode(lib.waveform_get_analysis_type(nativePtr));
        this.numPoints = lib.waveform_get_point_count(nativePtr);
        
        int numVars = lib.waveform_get_var_count(nativePtr);
        this.variables = new ArrayList<>(numVars);
        for (int i = 0; i < numVars; i++) {
            String name = lib.waveform_get_var_name(nativePtr, i);
            Variable.VarType type = Variable.VarType.fromCode(lib.waveform_get_var_type(nativePtr, i));
            variables.add(new Variable(name, type));
        }
    }

    // ========================================================================
    // Metadata Getters
    // ========================================================================

    public String getTitle() { return title; }
    public String getDate() { return date; }
    public String getScaleName() { return scaleName; }
    public AnalysisType getAnalysis() { return analysis; }
    public List<Variable> getVariables() { return variables; }
    public int getNumPoints() { return numPoints; }
    public int getNumVars() { return variables.size(); }

    public boolean hasSweep() {
        return lib.waveform_has_sweep(nativePtr) != 0;
    }

    public String getSweepParam() {
        return lib.waveform_get_sweep_param(nativePtr);
    }

    public int getTableCount() {
        return lib.waveform_get_table_count(nativePtr);
    }

    // ========================================================================
    // Data Access
    // ========================================================================

    /**
     * Get signal data by name.
     * @param signalName Signal name (e.g., "TIME", "v(out)")
     * @return Array of values, or null if not found
     */
    public double[] getRealData(String signalName) {
        return getRealData(signalName, 0);
    }

    /**
     * Get signal data by name from a specific table.
     */
    public double[] getRealData(String signalName, int tableIndex) {
        int varIndex = findVariableIndex(signalName);
        if (varIndex < 0) return null;
        return getRealDataByIndex(tableIndex, varIndex);
    }

    /**
     * Get signal data by index.
     */
    public double[] getRealDataByIndex(int tableIndex, int varIndex) {
        checkNotClosed();
        int length = lib.waveform_get_data_length(nativePtr, tableIndex, varIndex);
        if (length <= 0) return null;
        
        double[] buffer = new double[length];
        int copied = lib.waveform_get_real_data(nativePtr, tableIndex, varIndex, buffer, length);
        if (copied != length) {
            double[] result = new double[copied];
            System.arraycopy(buffer, 0, result, 0, copied);
            return result;
        }
        return buffer;
    }

    /**
     * Check if signal is complex.
     */
    public boolean isComplex(int tableIndex, int varIndex) {
        return lib.waveform_is_complex(nativePtr, tableIndex, varIndex) != 0;
    }

    /**
     * Get complex signal data (returns [real[], imag[]] pair).
     */
    public double[][] getComplexData(String signalName) {
        return getComplexData(signalName, 0);
    }

    public double[][] getComplexData(String signalName, int tableIndex) {
        int varIndex = findVariableIndex(signalName);
        if (varIndex < 0) return null;
        
        checkNotClosed();
        int length = lib.waveform_get_data_length(nativePtr, tableIndex, varIndex);
        if (length <= 0) return null;
        
        double[] real = new double[length];
        double[] imag = new double[length];
        lib.waveform_get_complex_data(nativePtr, tableIndex, varIndex, real, imag, length);
        return new double[][] { real, imag };
    }

    // ========================================================================
    // Utility
    // ========================================================================

    private int findVariableIndex(String name) {
        for (int i = 0; i < variables.size(); i++) {
            if (variables.get(i).getName().equalsIgnoreCase(name)) {
                return i;
            }
        }
        return -1;
    }

    private void checkNotClosed() {
        if (closed) {
            throw new IllegalStateException("WaveformResult has been closed");
        }
    }

    @Override
    public void close() {
        if (!closed && nativePtr != null) {
            lib.waveform_free(nativePtr);
            nativePtr = null;
            closed = true;
        }
    }

    @Override
    public String toString() {
        return String.format("WaveformResult{title='%s', analysis=%s, vars=%d, points=%d}",
            title, analysis, variables.size(), numPoints);
    }

    // ========================================================================
    // Analysis Type
    // ========================================================================

    public enum AnalysisType {
        TRANSIENT(0),
        AC(1),
        DC(2),
        OPERATING(3),
        NOISE(4),
        UNKNOWN(-1);

        private final int code;

        AnalysisType(int code) {
            this.code = code;
        }

        public static AnalysisType fromCode(int code) {
            for (AnalysisType type : values()) {
                if (type.code == code) {
                    return type;
                }
            }
            return UNKNOWN;
        }
    }
}
