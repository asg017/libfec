.PHONY: install build build-release test test-node test-deno test-bun typecheck clean

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

typecheck:                     ## tsc --noEmit (needs `make build` for native/native.d.ts)
	npx tsc --noEmit -p tsconfig.json

clean:
	rm -rf dist native node_modules
