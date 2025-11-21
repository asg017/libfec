.PHONY: build test test-deno test-node clean help

all: build

build:
	@echo "Building fec-wasm..."
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen --target web --out-dir pkg ../../target/wasm32-unknown-unknown/release/fec_wasm.wasm

test: test-deno test-node

test-deno:
	@echo "Running Deno example..."
	deno run --allow-read example.deno.ts

test-node:
	@echo "Running Node.js example..."
	node example.node.mjs

clean:
	@echo "Cleaning build artifacts..."
	rm -rf pkg/
	cargo clean

# Show help
help:
	@echo "Available targets:"
	@echo "  make build      - Build the WASM module"
	@echo "  make test       - Run all tests (Deno + Node.js)"
	@echo "  make test-deno  - Run Deno example"
	@echo "  make test-node  - Run Node.js example"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make help       - Show this help message"
