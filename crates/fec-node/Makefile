.PHONY: install build build-release test test-node test-deno test-bun test-slow bench typecheck gen-types clean

install:                       ## one-time: npm ci
	npm ci

build:                         ## debug addon + JS
	npm run build

build-release:                 ## optimized addon + JS
	npm run build:native:release && npm run build:js

test: test-node test-deno test-bun   ## the same suite under all three runtimes

test-node:
	node scripts/test.mjs node

test-deno:
	node scripts/test.mjs deno

test-bun:
	node scripts/test.mjs bun

test-slow: build-release        ## perf guards on benchmarks/1805248.fec (all three runtimes)
	node scripts/test.mjs node --slow
	node scripts/test.mjs deno --slow
	node scripts/test.mjs bun --slow

BENCH_FILE ?= ../../benchmarks/1805248.fec

bench: build-release           ## benchmarks/node/bench.mjs on BENCH_FILE under node, deno, bun
	node ../../benchmarks/node/bench.mjs $(BENCH_FILE)

typecheck:                     ## tsc --noEmit (needs `make build` for native/native.d.ts)
	npx tsc --noEmit -p tsconfig.json
	npx tsc -p tests/tsconfig.json

GEN_SCRATCH := ../../target/gen-types

gen-types:                     ## regenerate js/generated/ from fec-parser (commit the result)
	rm -rf $(GEN_SCRATCH)
	cargo run -q -p fec-node --features gen-types --bin gen-types -- $(GEN_SCRATCH)
	node scripts/gen-types.mjs $(GEN_SCRATCH)

clean:
	rm -rf dist native node_modules
