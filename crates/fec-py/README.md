# fec-py

Python bindings for the FEC parser library.

## Building

```bash
# Development build
maturin develop -m crates/fec-py/Cargo.toml

# Build wheel
maturin build -m crates/fec-py/Cargo.toml --out dist

# Release build
maturin build -m crates/fec-py/Cargo.toml --release --out dist
```

**Note:** Don't use `cargo build` directly - use `maturin` to build PyO3 extension modules.
