# HSPICE WASM Parser

High-performance WebAssembly library for parsing HSPICE binary waveform files (.tr0, .ac0, .sw0) in the browser.

## Installation

```bash
npm install hspice-wasm
```

## Usage

### Basic Usage

```typescript
import init, { parseHspice } from "hspice-wasm";

// Initialize WASM (required once)
await init();

// Load file (from input element, fetch, etc.)
const fileData = new Uint8Array(await file.arrayBuffer());

// Parse the file
const result = parseHspice(fileData);

console.log(`Title: ${result.title}`);
console.log(`Analysis: ${result.analysis}`);
console.log(`Variables: ${result.numVars}`);
console.log(`Points: ${result.numPoints}`);

// Access signal data
const time = result.tables[0].signals["TIME"];
const vout = result.tables[0].signals["v(out)"];
```

### Get Signal Names

```typescript
import { getSignalNames } from "hspice-wasm";

const names = getSignalNames(fileData);
// ["TIME", "v(out)", "i(vin)", ...]
```

### Get Single Signal

```typescript
import { getSignalData } from "hspice-wasm";

const time = getSignalData(fileData, "TIME");
// Float64Array
```

## API

### `parseHspice(data: Uint8Array): WaveformResult`

Parse HSPICE binary data and return complete result.

### `getSignalNames(data: Uint8Array): string[]`

Get all signal names from a file.

### `getSignalData(data: Uint8Array, signalName: string): Float64Array`

Get specific signal data.

## Types

```typescript
interface WaveformResult {
  title: string;
  date: string;
  scaleName: string; // "TIME", "HERTZ"
  analysis: string; // "transient", "ac", "dc"
  variables: Variable[];
  sweepParam: string | null;
  tables: DataTable[];
  numPoints: number;
  numVars: number;
  numSweeps: number;
}

interface Variable {
  name: string;
  type: string; // "time", "voltage", "current", "frequency"
}

interface DataTable {
  sweepValue: number | null;
  signals: Record<string, Float64Array>;
}
```

## Building from Source

```bash
# Install wasm-pack
cargo install wasm-pack

# Build WASM package
cd crates/hspice-wasm
wasm-pack build --target web
```

## License

MIT
