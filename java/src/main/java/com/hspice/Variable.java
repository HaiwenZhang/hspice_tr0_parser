package com.hspice;

/**
 * Represents a variable/signal in a waveform result.
 */
public class Variable {
    private final String name;
    private final VarType type;

    public Variable(String name, VarType type) {
        this.name = name;
        this.type = type;
    }

    public String getName() {
        return name;
    }

    public VarType getType() {
        return type;
    }

    @Override
    public String toString() {
        return "Variable{name='" + name + "', type=" + type + "}";
    }

    /**
     * Variable type enumeration.
     */
    public enum VarType {
        TIME(0),
        FREQUENCY(1),
        VOLTAGE(2),
        CURRENT(3),
        UNKNOWN(-1);

        private final int code;

        VarType(int code) {
            this.code = code;
        }

        public static VarType fromCode(int code) {
            for (VarType type : values()) {
                if (type.code == code) {
                    return type;
                }
            }
            return UNKNOWN;
        }
    }
}
