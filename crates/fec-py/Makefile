.PHONY: develop develop-release build build-release test test-network test-slow notebook stubs clean

# One-time: uv venv && uv sync --group dev

develop:                       ## incremental build into .venv (fast, debug)
	uv run maturin develop --uv

develop-release:               ## same, optimized (for benchmarks / notebook)
	uv run maturin develop --uv --release

build:                         ## wheel into dist/ (what CI and releases do)
	uv run maturin build --release --out dist

build-release: build

test: develop
	uv run pytest tests

test-network: develop
	uv run pytest tests -m network

test-slow: develop-release
	uv run pytest tests -m slow

notebook: develop-release
	uv run --with jupyterlab jupyter lab examples/quickstart.ipynb

stubs: develop                 ## check python/libfec_parser/*.pyi against the built extension
	uv run python -m mypy.stubtest libfec_parser --allowlist stubtest-allowlist.txt
	uv run mypy --check-untyped-defs tests

clean:
	rm -rf dist .venv python/libfec_parser/*.so
