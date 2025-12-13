/**
 * HSPICE Waveform Parser - TypeScript Type Definitions
 * 
 * High-performance WebAssembly library for parsing HSPICE binary files.
 * 
 * @example
 * ```typescript
 * import init, { parseHspice, getSignalNames, getSignalData } from 'hspice-wasm';
 * 
 * // Initialize WASM
 * await init();
 * 
 * // Load file as Uint8Array
 * const fileData = new Uint8Array(await file.arrayBuffer());
 * 
 * // Parse the file
 * const result = parseHspice(fileData);
 * console.log(result.title);
 * console.log(result.analysis);
 * console.log(result.variables);
 * 
 * // Access signal data
 * const time = result.tables[0].signals['TIME'];
 * ```
 */

/** Variable metadata */
export interface Variable {
  /** Variable name (e.g., "TIME", "v(out)") */
  name: string;
  /** Variable type: "time" | "frequency" | "voltage" | "current" | "unknown" */
  type: string;
}

/** Data table (one per sweep point) */
export interface DataTable {
  /** Sweep value (if swept) */
  sweepValue: number | null;
  /** Signal data indexed by name */
  signals: Record<string, Float64Array>;
}

/** Parsed waveform result */
export interface WaveformResult {
  /** Simulation title */
  title: string;
  /** Simulation date */
  date: string;
  /** Scale variable name (e.g., "TIME", "HERTZ") */
  scaleName: string;
  /** Analysis type: "transient" | "ac" | "dc" | "operating" | "noise" | "unknown" */
  analysis: string;
  /** Variable definitions */
  variables: Variable[];
  /** Sweep parameter name (if swept) */
  sweepParam: string | null;
  /** Data tables (one per sweep point) */
  tables: DataTable[];
  /** Number of data points */
  numPoints: number;
  /** Number of variables */
  numVars: number;
  /** Number of sweep points */
  numSweeps: number;
}

/**
 * Parse HSPICE binary data.
 * 
 * @param data - Binary file content as Uint8Array
 * @returns Parsed waveform result
 * @throws Error if parsing fails
 * 
 * @example
 * ```typescript
 * const fileData = new Uint8Array(await file.arrayBuffer());
 * const result = parseHspice(fileData);
 * console.log(`Title: ${result.title}`);
 * console.log(`Analysis: ${result.analysis}`);
 * console.log(`Variables: ${result.numVars}`);
 * ```
 */
export function parseHspice(data: Uint8Array): WaveformResult;

/**
 * Get all signal names from a file.
 * 
 * @param data - Binary file content as Uint8Array
 * @returns Array of signal names
 * 
 * @example
 * ```typescript
 * const names = getSignalNames(fileData);
 * console.log(names); // ["TIME", "v(out)", "i(vin)", ...]
 * ```
 */
export function getSignalNames(data: Uint8Array): string[];

/**
 * Get specific signal data from a file.
 * 
 * @param data - Binary file content as Uint8Array
 * @param signalName - Name of the signal to retrieve
 * @returns Signal data as Float64Array
 * @throws Error if signal not found
 * 
 * @example
 * ```typescript
 * const time = getSignalData(fileData, 'TIME');
 * const vout = getSignalData(fileData, 'v(out)');
 * ```
 */
export function getSignalData(data: Uint8Array, signalName: string): Float64Array;

/**
 * Initialize the WASM module.
 * Must be called before using any other functions.
 * 
 * @param moduleOrPath - Optional: WASM module or path to .wasm file
 * @returns Promise that resolves when initialization is complete
 * 
 * @example
 * ```typescript
 * // Default initialization
 * await init();
 * 
 * // Or with custom path
 * await init('/path/to/hspice_wasm_bg.wasm');
 * ```
 */
export default function init(moduleOrPath?: RequestInfo | URL | WebAssembly.Module): Promise<void>;
