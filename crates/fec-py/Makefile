.PHONY: build build-release demo demo-fecfile clean

# Build development wheel
build:
	maturin build -m Cargo.toml --out dist

# Build release (optimized) wheel
build-release:
	maturin build -m Cargo.toml --release --out dist

# Run demo.py with sample FEC files from cache2
demo: build
	uv run --no-cache --no-project --isolated \
		--with 'libfec_parser @ file://dist/libfec_parser-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
		demo.py ../../cache2/*.fec

# Run demo-fecfile.py with sample FEC files from cache2
demo-fecfile: build
	uv run --no-cache --no-project --isolated \
		--with 'libfec_parser @ file://dist/libfec_parser-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
		demo-fecfile.py ../../cache2/*.fec