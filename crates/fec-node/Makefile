.PHONY: install build build-release test test-node test-deno test-bun clean

install:                       ## one-time: npm ci
	npm ci

build:                         ## debug addon + JS
	npm run build

build-release:                 ## optimized addon + JS
	npm run build:native:release && npm run build:js

test: test-node test-deno test-bun

clean:
	rm -rf dist native node_modules
