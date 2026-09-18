.PHONY: build build-release demo demo-fecfile notebook test-pytest clean

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

# Open examples/quickstart.ipynb in JupyterLab with the freshly built wheel installed
notebook: build
	uv run --no-cache --no-project --isolated \
		--with "$$(ls $(CURDIR)/dist/libfec_parser-*.whl | head -1)" \
		--with pandas \
		--with jupyterlab \
		jupyter lab examples/quickstart.ipynb

test-pytest:
	uv run --no-cache --no-project --isolated \
		--with pytest \
		--with 'libfec_parser @ file://dist/libfec_parser-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
		pytest tests/