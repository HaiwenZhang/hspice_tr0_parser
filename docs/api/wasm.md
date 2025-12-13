# WebAssembly API Documentation

This document covers the WebAssembly API for `hspice-wasm`.

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
cd crates/hspice-wasm
wasm-pack build --target web

# Output: pkg/
#   hspice_wasm.js
#   hspice_wasm_bg.wasm
#   hspice_wasm.d.ts
```

## Installation

### NPM (after publishing)

```bash
npm install hspice-wasm
```

### Local

Copy `pkg/` contents to your project.

## API Reference

### `init()`

Initialize the WASM module. Must be called before other functions.

```typescript
import init from "hspice-wasm";
await init();
```

### `parseHspice(data: Uint8Array): WaveformResult`

Parse binary file data.

```typescript
import { parseHspice } from "hspice-wasm";

const fileData = new Uint8Array(await file.arrayBuffer());
const result = parseHspice(fileData);
```

### `getSignalNames(data: Uint8Array): string[]`

Get all signal names from a file.

```typescript
import { getSignalNames } from "hspice-wasm";

const names = getSignalNames(fileData);
// ["TIME", "v(out)", "i(vin)", ...]
```

### `getSignalData(data: Uint8Array, signalName: string): Float64Array`

Get specific signal data.

```typescript
import { getSignalData } from "hspice-wasm";

const time = getSignalData(fileData, "TIME");
const vout = getSignalData(fileData, "v(out)");
```

### `parseRaw(data: Uint8Array): WaveformResult`

Parse SPICE3/ngspice raw file (auto-detects binary/ASCII format).

```typescript
import { parseRaw } from "hspice-wasm";

const rawData = new Uint8Array(await file.arrayBuffer());
const result = parseRaw(rawData);
console.log(result.title);
const time = result.tables[0].signals["time"];
```

## Types

### `WaveformResult`

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
```

### `Variable`

```typescript
interface Variable {
  name: string;
  type: string; // "time", "voltage", "current", "frequency"
}
```

### `DataTable`

```typescript
interface DataTable {
  sweepValue: number | null;
  signals: Record<string, Float64Array>;
}
```

## Complete Example

### Basic Usage

```typescript
import init, { parseHspice } from "hspice-wasm";

async function loadWaveform(file: File) {
  // Initialize WASM
  await init();

  // Load file
  const buffer = await file.arrayBuffer();
  const data = new Uint8Array(buffer);

  // Parse
  const result = parseHspice(data);

  console.log(`Title: ${result.title}`);
  console.log(`Analysis: ${result.analysis}`);
  console.log(`Variables: ${result.numVars}`);
  console.log(`Points: ${result.numPoints}`);

  // Access data
  const time = result.tables[0].signals["TIME"];
  const vout = result.tables[0].signals["v(out)"];

  return { time, vout };
}
```

### With React

```tsx
import { useState, useEffect } from "react";
import init, { parseHspice, WaveformResult } from "hspice-wasm";

function WaveformViewer() {
  const [result, setResult] = useState<WaveformResult | null>(null);
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    init().then(() => setInitialized(true));
  }, []);

  const handleFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!initialized || !e.target.files?.[0]) return;

    const file = e.target.files[0];
    const data = new Uint8Array(await file.arrayBuffer());
    setResult(parseHspice(data));
  };

  return (
    <div>
      <input type="file" onChange={handleFile} accept=".tr0,.ac0,.sw0" />
      {result && (
        <div>
          <h2>{result.title}</h2>
          <p>Analysis: {result.analysis}</p>
          <p>Variables: {result.numVars}</p>
          <p>Points: {result.numPoints}</p>
          <ul>
            {result.variables.map((v) => (
              <li key={v.name}>
                {v.name}: {v.type}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
```

### With Chart.js

```typescript
import init, { parseHspice } from "hspice-wasm";
import Chart from "chart.js/auto";

async function plotWaveform(file: File, canvas: HTMLCanvasElement) {
  await init();

  const data = new Uint8Array(await file.arrayBuffer());
  const result = parseHspice(data);

  const time = Array.from(result.tables[0].signals["TIME"]);
  const vout = Array.from(result.tables[0].signals["v(out)"]);

  new Chart(canvas, {
    type: "line",
    data: {
      labels: time.map((t) => t.toExponential(2)),
      datasets: [
        {
          label: "v(out)",
          data: vout,
          borderColor: "blue",
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      plugins: {
        title: { display: true, text: result.title },
      },
    },
  });
}
```

## Bundler Configuration

### Vite

```typescript
// vite.config.ts
import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["hspice-wasm"],
  },
});
```

### Webpack

```javascript
// webpack.config.js
module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },
};
```
