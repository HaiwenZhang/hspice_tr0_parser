# HSPICE Waveform Parser - Java Bindings

Java API for parsing HSPICE and SPICE3/ngspice waveform files using JNA (Java Native Access).

## Requirements

- Java 11+
- Maven 3.6+
- Native library `libhspicetr0parser` in system library path

## Installation

### Build from Source

```bash
# Build the native library first
cd ..
cargo build -p hspice-ffi --release

# Build Java bindings
cd java
mvn clean package
```

### Add to Your Project (Maven)

```xml
<dependency>
    <groupId>com.hspice</groupId>
    <artifactId>hspice-parser</artifactId>
    <version>1.2.0</version>
</dependency>
```

## Usage

### Basic Example

```java
import com.hspice.HspiceParser;
import com.hspice.WaveformResult;

public class Example {
    public static void main(String[] args) {
        // Read HSPICE file
        try (WaveformResult result = HspiceParser.read("simulation.tr0")) {
            System.out.println("Title: " + result.getTitle());
            System.out.println("Analysis: " + result.getAnalysis());
            System.out.println("Variables: " + result.getNumVars());
            System.out.println("Points: " + result.getNumPoints());

            // List all variables
            for (Variable var : result.getVariables()) {
                System.out.println("  " + var.getName() + ": " + var.getType());
            }

            // Get signal data
            double[] time = result.getRealData("TIME");
            double[] vout = result.getRealData("v(out)");

            System.out.printf("Time: %.3e to %.3e%n", time[0], time[time.length - 1]);
        }
    }
}
```

### Reading SPICE3 Raw Files

```java
// Auto-detects binary or ASCII format
try (WaveformResult result = HspiceParser.readRaw("simulation.raw")) {
    double[] time = result.getRealData("time");
    double[] vout = result.getRealData("v(out)");
}
```

### AC Analysis (Complex Data)

```java
try (WaveformResult result = HspiceParser.read("simulation.ac0")) {
    if (result.isComplex(0, 1)) {
        double[][] data = result.getComplexData("v(out)");
        double[] real = data[0];
        double[] imag = data[1];
    }
}
```

## API Reference

### HspiceParser

| Method                 | Description                         |
| ---------------------- | ----------------------------------- |
| `read(filename)`       | Read HSPICE file (.tr0, .ac0, .sw0) |
| `readRaw(filename)`    | Read SPICE3 raw file                |
| `isLibraryAvailable()` | Check if native library is loaded   |

### WaveformResult

| Method                 | Description                             |
| ---------------------- | --------------------------------------- |
| `getTitle()`           | Simulation title                        |
| `getDate()`            | Simulation date                         |
| `getAnalysis()`        | Analysis type (TRANSIENT, AC, DC, etc.) |
| `getScaleName()`       | Scale variable name (TIME, HERTZ)       |
| `getVariables()`       | List of Variable objects                |
| `getRealData(name)`    | Get signal data by name                 |
| `getComplexData(name)` | Get complex signal data                 |
| `close()`              | Free native resources                   |

## Library Path Setup

The native library must be in your system library path:

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/target/release:$LD_LIBRARY_PATH

# macOS
export DYLD_LIBRARY_PATH=/path/to/target/release:$DYLD_LIBRARY_PATH

# Windows
set PATH=C:\path\to\target\release;%PATH%
```

Or set via Java property:

```bash
java -Djna.library.path=/path/to/lib -jar myapp.jar
```

## License

MIT License
