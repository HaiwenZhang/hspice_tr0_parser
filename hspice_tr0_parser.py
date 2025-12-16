"""
**HSPICE Waveform Parser**
Rust implementation with PyO3 by Haiwen Zhang
"""

import hspicetr0parser as _lib

__all__ = ['read', 'read_raw', 'convert_to_raw', 'stream', 'init_logging', 'WaveformResult', 'Variable', 'DataTable']

# Re-export classes
WaveformResult = _lib.WaveformResult
Variable = _lib.Variable
DataTable = _lib.DataTable


def init_logging(level="info"):
    """
    Initialize logging with specified level.
    
    Args:
        level: Log level ("trace", "debug", "info", "warn", "error")
    
    Example:
        >>> from hspice_tr0_parser import init_logging
        >>> init_logging("debug")
    """
    return _lib.init_logging(level)


def read(filename, debug=0):
    """
    Read HSPICE/waveform binary file.
    
    Args:
        filename: Path to the waveform file (.tr0, .ac0, .sw0)
        debug: Debug level (deprecated, use init_logging() instead)
    
    Returns:
        WaveformResult object with the following attributes:
        - title: Simulation title
        - date: Simulation date
        - analysis: Analysis type ('transient', 'ac', 'dc', etc.)
        - scale_name: Scale variable name ('TIME', 'HERTZ', etc.)
        - variables: List of Variable objects with name and var_type
        - tables: List of DataTable objects
        - sweep_param: Sweep parameter name (if swept)
        
        Returns None if an error occurs.
    
    Example:
        >>> from hspice_tr0_parser import read
        >>> result = read('simulation.tr0')
        >>> print(result.title)
        >>> print(result.analysis)  # 'transient'
        >>> 
        >>> # Access signal by name
        >>> time = result.get('TIME')
        >>> vout = result.get('v(out)')
        >>> 
        >>> # List all variables
        >>> for var in result.variables:
        ...     print(f"{var.name}: {var.var_type}")
    """
    # Enable logging if debug > 0 for backward compatibility
    if debug > 0:
        levels = {1: "info", 2: "debug"}
        _lib.init_logging(levels.get(debug, "info"))
    return _lib.read(filename)


def convert_to_raw(input_path, output_path, debug=0):
    """
    Convert HSPICE binary file to SPICE3 raw format.
    
    Args:
        input_path: Path to the input HSPICE file
        output_path: Path for the output .raw file
        debug: Debug level (deprecated, use init_logging() instead)
    
    Returns:
        True if conversion succeeded, False otherwise.
    
    Example:
        >>> from hspice_tr0_parser import convert_to_raw
        >>> convert_to_raw('simulation.tr0', 'simulation.raw')
        True
    """
    # Enable logging if debug > 0 for backward compatibility
    if debug > 0:
        levels = {1: "info", 2: "debug"}
        _lib.init_logging(levels.get(debug, "info"))
    return _lib.convert_to_raw(input_path, output_path)


def stream(filename, chunk_size=10000, signals=None, debug=0):
    """
    Stream HSPICE binary file in chunks for memory-efficient processing.
    
    Args:
        filename: Path to the waveform file
        chunk_size: Minimum points per chunk (default: 10000)
        signals: Optional list of signal names to filter
        debug: Debug level (deprecated, use init_logging() instead)
    
    Yields:
        dict: Chunk with 'chunk_index', 'time_range', 'data'
    
    Example:
        >>> from hspice_tr0_parser import stream
        >>> for chunk in stream('huge_simulation.tr0'):
        ...     print(f"Chunk {chunk['chunk_index']}: {chunk['time_range']}")
        ...     time = chunk['data']['TIME']
    """
    # Enable logging if debug > 0 for backward compatibility
    if debug > 0:
        levels = {1: "info", 2: "debug"}
        _lib.init_logging(levels.get(debug, "info"))
    chunks = _lib.stream(filename, chunk_size, signals)
    for chunk in chunks:
        yield chunk


def read_raw(filename, debug=0):
    """
    Read SPICE3/ngspice raw file (auto-detects binary/ASCII format).
    
    Args:
        filename: Path to the raw file (.raw)
        debug: Debug level (deprecated, use init_logging() instead)
    
    Returns:
        WaveformResult object with the following attributes:
        - title: Simulation title
        - date: Simulation date
        - analysis: Analysis type ('transient', 'ac', 'dc', etc.)
        - scale_name: Scale variable name ('time', 'frequency', etc.)
        - variables: List of Variable objects with name and var_type
        - tables: List of DataTable objects
        
        Returns None if an error occurs.
    
    Example:
        >>> from hspice_tr0_parser import read_raw
        >>> result = read_raw('simulation.raw')
        >>> print(result.title)
        >>> time = result.get('time')
        >>> vout = result.get('v(out)')
    """
    # Enable logging if debug > 0 for backward compatibility
    if debug > 0:
        levels = {1: "info", 2: "debug"}
        _lib.init_logging(levels.get(debug, "info"))
    return _lib.read_raw(filename)