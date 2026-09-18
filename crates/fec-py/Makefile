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
		--with "$$(ls dist/libfec_parser-*.whl | head -1)" \
		demo.py ../../cache2/*.fec

# Run demo-fecfile.py with sample FEC files from cache2
demo-fecfile: build
	uv run --no-cache --no-project --isolated \
		--with "$$(ls dist/libfec_parser-*.whl | head -1)" \
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
		--with "$$(ls dist/libfec_parser-*.whl | head -1)" \
		pytest tests/