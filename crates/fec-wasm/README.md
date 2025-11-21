# fec-wasm

WebAssembly bindings for the FEC parser.

## Building

1. Build the WASM module:
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. Generate JavaScript bindings:
```bash
wasm-bindgen --target web --out-dir pkg ../../target/wasm32-unknown-unknown/release/fec_wasm.wasm
```

Or use the combined command:
```bash
cargo build --target wasm32-unknown-unknown --release && \
  wasm-bindgen --target web --out-dir pkg ../../target/wasm32-unknown-unknown/release/fec_wasm.wasm
```

## Examples

### Deno

```bash
deno run --allow-read example.deno.ts
```

### Node.js

```bash
node example.node.mjs
```

### Browser

Open `index.html` in a browser (requires a local web server):

```bash
# Using Python
python -m http.server 8000

# Using Deno
deno serve --port 8000 .

# Then open http://localhost:8000 in your browser
```

All examples parse the header from `1926611.fec` and display the FEC version.

## Requirements

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` version 0.2.105 (must match version in Cargo.toml)

Install wasm-bindgen-cli:
```bash
cargo install wasm-bindgen-cli --version 0.2.105
```

Add the wasm32 target:
```bash
rustup target add wasm32-unknown-unknown
```
