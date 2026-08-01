# Release Process

## Trigger

Push a version tag such as `v1.5.0`. The `Build and Release` workflow validates
that the tag matches the Cargo workspace, Python package, WASM package, and
Java project versions before building anything.

The workflow then runs Rust formatting, clippy, and workspace tests, plus the
Python suite on this matrix:

| Operating system | Python versions |
| ---------------- | --------------- |
| Linux | 3.12, 3.13, 3.14 |
| macOS | 3.12, 3.13, 3.14 |
| Windows | 3.12, 3.13, 3.14 |

CI on `main` and pull requests uses the same Python matrix. Tag builds are
handled only by the release workflow, avoiding duplicate CI runs.

## Version Sources

These files must carry the same release version:

- `Cargo.toml` (`workspace.package.version`)
- `pyproject.toml` (`project.version`)
- `crates/hspice-wasm/package.json`
- `java/pom.xml`

Cargo workspace members inherit the workspace version. `Cargo.lock`, README
examples, badges, and versioned documentation must be updated with the same
change.

Python support starts at 3.12. The extension uses PyO3's `abi3-py312` feature,
so each native target needs one `cp312-abi3` wheel rather than three duplicate
interpreter-specific wheels. The workflow still builds and tests the extension
with Python 3.12, 3.13, and 3.14 on every supported operating system.

## GitHub Release Assets

| Component | Release assets | Targets |
| --------- | -------------- | ------- |
| Python | `*.whl`, `*.tar.gz` sdist | Linux x86_64/aarch64, macOS Intel/Apple Silicon, Windows x86_64 |
| CLI | `hspice-cli-<version>-<target>.tar.gz` or `.zip` | Linux x86_64/aarch64, macOS Intel/Apple Silicon, Windows x86_64 |
| C ABI | `hspice-ffi-<version>-<target>.tar.gz` or `.zip` | Same native targets as the CLI |
| WASM | `hspice-wasm-<version>.tgz` | Browser/web bundlers |
| Java | `hspice-parser-<version>.jar`, `hspice-java-<version>.zip` | Platform-independent JNA wrapper and Maven POM |
| Integrity | `SHA256SUMS` | All attached files |

The C ABI archives contain the public header plus static and dynamic libraries.
The Java JAR does not embed native code; users also download the matching C ABI
archive and place its dynamic library on the JNA library path. The Java ZIP
adds the Maven POM, binding README, and license for local installation.

There is no separate Go binary or module artifact. The documented Go wrapper
uses CGO and links to the matching C ABI archive, so publishing another copy
would add ambiguity without adding functionality.

GitHub automatically adds source `.zip` and `.tar.gz` archives for the tag.
The Python sdist is still attached because it is Python-package metadata and
can be built by pip/maturin.

## Pre-Tag Checklist

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

With a Python 3.12–3.14 virtual environment:

```bash
python -m pip install --upgrade pip maturin numpy pytest
python -m pip install --no-build-isolation --no-deps .
python -m pytest tests -v
```

Also verify that `git diff --check` succeeds and that the tag exactly matches
the package version, including any pre-release suffix.

## Publishing Scope

The workflow creates a GitHub Release and attaches installable artifacts. It
does not publish to PyPI, npm, or Maven Central; those registries require
separate credentials, namespace ownership, and signing policy.

The manual `CLI Release` workflow remains available for a CLI-only release
under a `cli-v<version>` tag, but normal project releases should use `v<version>`
so every supported artifact is built and published together.
