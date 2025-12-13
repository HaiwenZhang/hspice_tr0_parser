package com.hspice;

/**
 * Exception thrown when HSPICE parsing operations fail.
 */
public class HspiceException extends RuntimeException {
    
    public HspiceException(String message) {
        super(message);
    }

    public HspiceException(String message, Throwable cause) {
        super(message, cause);
    }
}
