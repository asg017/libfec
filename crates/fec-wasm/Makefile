.PHONY: install install-tools build build-dev build-js test test-node test-deno test-bun test-browser test-workers bench-browser demo clean

install:                       ## one-time: npm ci
	npm ci

install-tools:                 ## the wasm32 target, wasm-bindgen (pinned to Cargo.toml's) and wasm-opt
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli --version 0.2.129 --locked
	@command -v wasm-opt >/dev/null || echo "also install binaryen for wasm-opt (brew install binaryen / apt install binaryen)"

build:                         ## optimized wasm (wasm-release + wasm-opt -O3) + JS
	node scripts/build.mjs

build-dev:                     ## unoptimized wasm, no wasm-opt (fast rebuilds)
	node scripts/build.mjs --dev

build-js:                      ## only the TypeScript layer (after changing crates/fec-node/js)
	node scripts/build.mjs --js

test: test-node test-deno test-bun   ## parity with the napi build + browser-only behavior, under all three runtimes (needs `make build` in ../fec-node)

test-node:
	node scripts/test.mjs node

test-deno:
	node scripts/test.mjs deno

test-bun:
	node scripts/test.mjs bun

test-browser:                  ## headless Chrome (CHROME_PATH=...): the API vs the napi build (default and strict CSP), then the demo
	node tests/browser/run.mjs
	node tests/browser/demo.mjs

test-workers:                  ## the packed package in Cloudflare's workerd (wrangler via npx): bundled wasm, parity with the napi build
	node tests/workers/run.mjs

bench-browser:                 ## the browser build in headless Chrome on BENCH_FILE (default benchmarks/1805248.fec)
	node tests/browser/bench.mjs $(BENCH_FILE)

demo:                          ## the demo app on http://127.0.0.1:8000/fec-wasm/demo/ (with the FEC proxy)
	node scripts/serve.mjs --port 8000

clean:
	rm -rf build dist wasm node_modules
