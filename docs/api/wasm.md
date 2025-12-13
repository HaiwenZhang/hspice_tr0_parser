# WebAssembly (WASM) API Documentation

TypeScript API and React.js integration guide for parsing HSPICE binary files in the browser.

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
cd crates/hspice-wasm
wasm-pack build --target web
# Output: pkg/
```

## TypeScript API

### Type Definitions

```typescript
// hspice_wasm.d.ts
export interface HspiceTable {
  signalNames: string[];
  data: { [signalName: string]: Float64Array };
}

export interface HspiceResult {
  title: string;
  date: string;
  scaleName: string;
  sweepName: string | null;
  sweepValues: Float64Array | null;
  tableCount: number;
  tables: HspiceTable[];
}

export function parseHspice(data: Uint8Array): HspiceResult;
export function getSignalData(
  result: HspiceResult,
  tableIndex: number,
  signalName: string
): Float64Array;
export function getSignalNames(
  result: HspiceResult,
  tableIndex: number
): string[];
export default function init(): Promise<void>;
```

### Basic Usage

```typescript
import init, { parseHspice, HspiceResult } from "./pkg/hspice_wasm";

async function loadHspiceFile(file: File): Promise<HspiceResult> {
  await init();

  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);

  return parseHspice(bytes);
}

// Usage
const result = await loadHspiceFile(file);
console.log(result.title);
console.log(result.tables[0].data["TIME"]);
```

---

## React.js Integration

### Installation

```bash
# Add to your React project
npm install @haiwen/hspice-parser

# Or copy built pkg/ folder to your project
```

### Vite Configuration

```typescript
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  plugins: [react(), wasm(), topLevelAwait()],
});
```

### React Hook

```typescript
// hooks/useHspiceParser.ts
import { useState, useCallback } from "react";
import init, { parseHspice, HspiceResult } from "@haiwen/hspice-parser";

let initialized = false;

export function useHspiceParser() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<HspiceResult | null>(null);

  const parse = useCallback(async (file: File) => {
    setLoading(true);
    setError(null);

    try {
      if (!initialized) {
        await init();
        initialized = true;
      }

      const buffer = await file.arrayBuffer();
      const data = parseHspice(new Uint8Array(buffer));
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Parse failed");
    } finally {
      setLoading(false);
    }
  }, []);

  return { parse, result, loading, error };
}
```

### File Upload Component

```tsx
// components/HspiceUploader.tsx
import { useHspiceParser } from "../hooks/useHspiceParser";

export function HspiceUploader() {
  const { parse, result, loading, error } = useHspiceParser();

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) parse(file);
  };

  return (
    <div>
      <input
        type="file"
        accept=".tr0,.ac0,.sw0"
        onChange={handleChange}
        disabled={loading}
      />

      {loading && <p>Parsing...</p>}
      {error && <p style={{ color: "red" }}>{error}</p>}

      {result && (
        <div>
          <h2>{result.title}</h2>
          <p>Date: {result.date}</p>
          <p>Scale: {result.scaleName}</p>
          <p>Tables: {result.tableCount}</p>
          <p>Signals: {result.tables[0].signalNames.length}</p>
        </div>
      )}
    </div>
  );
}
```

### Waveform Viewer Component

```tsx
// components/WaveformViewer.tsx
import { useMemo } from "react";
import { HspiceResult } from "@haiwen/hspice-parser";

interface Props {
  result: HspiceResult;
  signalName: string;
  tableIndex?: number;
}

export function WaveformViewer({ result, signalName, tableIndex = 0 }: Props) {
  const { time, signal } = useMemo(() => {
    const table = result.tables[tableIndex];
    return {
      time: Array.from(table.data["TIME"] || table.data[result.scaleName]),
      signal: Array.from(table.data[signalName] || []),
    };
  }, [result, signalName, tableIndex]);

  if (signal.length === 0) {
    return <p>Signal "{signalName}" not found</p>;
  }

  const min = Math.min(...signal);
  const max = Math.max(...signal);

  return (
    <div>
      <h3>{signalName}</h3>
      <p>Points: {signal.length}</p>
      <p>
        Range: {min.toExponential(3)} to {max.toExponential(3)}
      </p>
      <p>
        Time: {time[0].toExponential(3)} to{" "}
        {time[time.length - 1].toExponential(3)}
      </p>
    </div>
  );
}
```

### Signal Selector Component

```tsx
// components/SignalSelector.tsx
import { HspiceResult } from "@haiwen/hspice-parser";

interface Props {
  result: HspiceResult;
  selected: string[];
  onChange: (signals: string[]) => void;
}

export function SignalSelector({ result, selected, onChange }: Props) {
  const signals = result.tables[0].signalNames.filter((s) => s !== "TIME");

  const toggle = (name: string) => {
    if (selected.includes(name)) {
      onChange(selected.filter((s) => s !== name));
    } else {
      onChange([...selected, name]);
    }
  };

  return (
    <div style={{ maxHeight: 300, overflow: "auto" }}>
      {signals.map((name) => (
        <label key={name} style={{ display: "block" }}>
          <input
            type="checkbox"
            checked={selected.includes(name)}
            onChange={() => toggle(name)}
          />
          {name}
        </label>
      ))}
    </div>
  );
}
```

### Complete App Example

```tsx
// App.tsx
import { useState } from "react";
import { HspiceUploader } from "./components/HspiceUploader";
import { SignalSelector } from "./components/SignalSelector";
import { WaveformViewer } from "./components/WaveformViewer";
import { useHspiceParser } from "./hooks/useHspiceParser";

export default function App() {
  const { parse, result, loading, error } = useHspiceParser();
  const [selectedSignals, setSelectedSignals] = useState<string[]>([]);

  return (
    <div style={{ padding: 20 }}>
      <h1>HSPICE Viewer</h1>

      <input
        type="file"
        accept=".tr0,.ac0,.sw0"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) {
            parse(file);
            setSelectedSignals([]);
          }
        }}
        disabled={loading}
      />

      {loading && <p>Loading...</p>}
      {error && <p style={{ color: "red" }}>{error}</p>}

      {result && (
        <div style={{ display: "flex", gap: 20, marginTop: 20 }}>
          <div style={{ width: 200 }}>
            <h3>Signals</h3>
            <SignalSelector
              result={result}
              selected={selectedSignals}
              onChange={setSelectedSignals}
            />
          </div>

          <div style={{ flex: 1 }}>
            <h3>Info</h3>
            <p>Title: {result.title}</p>
            <p>Date: {result.date}</p>

            {selectedSignals.map((sig) => (
              <WaveformViewer key={sig} result={result} signalName={sig} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

---

## Limitations

- **No File System**: Must provide file content as `Uint8Array`
- **Memory**: Large files loaded entirely into memory
- **No Streaming**: Files processed in single pass
