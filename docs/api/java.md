# Java API Documentation

This document covers the Java API for `hspice-parser`.

## Installation

### Maven

```xml
<dependency>
    <groupId>com.hspice</groupId>
    <artifactId>hspice-parser</artifactId>
    <version>1.2.0</version>
</dependency>
```

### Build from Source

```bash
cd java
mvn clean package
```

## API Reference

### `HspiceParser.read(filename)`

Read an HSPICE waveform file.

```java
import com.hspice.HspiceParser;
import com.hspice.WaveformResult;

try (WaveformResult result = HspiceParser.read("simulation.tr0")) {
    System.out.println(result.getTitle());
    System.out.println(result.getAnalysis());
}
```

### `HspiceParser.readRaw(filename)`

Read a SPICE3/ngspice raw file (auto-detects binary/ASCII).

```java
try (WaveformResult result = HspiceParser.readRaw("simulation.raw")) {
    double[] time = result.getRealData("time");
}
```

## Classes

### `WaveformResult`

Main result class (implements `AutoCloseable`).

**Methods:**

- `getTitle()` - Simulation title
- `getDate()` - Simulation date
- `getAnalysis()` - Analysis type enum
- `getScaleName()` - Scale variable name
- `getVariables()` - List of Variable objects
- `getNumVars()` - Number of variables
- `getNumPoints()` - Number of data points
- `getRealData(name)` - Get signal by name
- `getComplexData(name)` - Get complex signal
- `hasSweep()` - Check for sweep data
- `close()` - Free native resources

### `Variable`

Signal metadata.

**Methods:**

- `getName()` - Variable name
- `getType()` - Variable type (TIME, VOLTAGE, CURRENT, etc.)

### `WaveformResult.AnalysisType`

```java
enum AnalysisType {
    TRANSIENT, AC, DC, OPERATING, NOISE, UNKNOWN
}
```

### `Variable.VarType`

```java
enum VarType {
    TIME, FREQUENCY, VOLTAGE, CURRENT, UNKNOWN
}
```

## Example

```java
import com.hspice.*;

public class Example {
    public static void main(String[] args) {
        try (WaveformResult result = HspiceParser.read("simulation.tr0")) {
            // Metadata
            System.out.println("Title: " + result.getTitle());
            System.out.println("Analysis: " + result.getAnalysis());

            // List variables
            for (Variable var : result.getVariables()) {
                System.out.println(var.getName() + ": " + var.getType());
            }

            // Get data
            double[] time = result.getRealData("TIME");
            double[] vout = result.getRealData("v(out)");

            System.out.printf("Time: %.3e to %.3e%n",
                time[0], time[time.length - 1]);
        }
    }
}
```

## Native Library Setup

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/lib:$LD_LIBRARY_PATH

# macOS
export DYLD_LIBRARY_PATH=/path/to/lib:$DYLD_LIBRARY_PATH

# Or via JVM property
java -Djna.library.path=/path/to/lib MyApp
```
