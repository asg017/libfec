# Dev loop and release build for the libfec R package. Run from crates/fec-r.
#
#   make document install test   # development (fec-parser by `path =`, crates fetched online)
#   make build                   # libfec_<version>.tar.gz that installs offline from anywhere
#
# Why `build` is more than `R CMD build .` (decision R9): `R CMD build` copies only this
# directory, so `path = '../../../fec-parser'` doesn't exist inside the tarball, and
# `cargo vendor` doesn't vendor path dependencies. A cargo `[patch]` (or `paths = [...]`)
# override can't fix that: cargo loads the original path dependency's manifest before it applies
# either, and fails on the missing file (tried, cargo 1.96). So `build` works on a staging copy
# of the package: it copies fec-parser + fec-parser-macros into `src/rust/local/`, rewrites the
# path in the staged `src/rust/Cargo.toml`, vendors every other crate into
# `src/rust/vendor.tar.xz`, and runs `R CMD build` there. The working tree is never modified
# (only the tarball lands here, and it's gitignored), even when a step fails.
#
# Needs: R, cargo, tar with xz support (`-J`), sed, network access for `cargo vendor`.

R ?= R
CARGO ?= cargo
PKG := libfec
VERSION := $(shell sed -n 's/^Version: *//p' DESCRIPTION)
TARBALL := $(PKG)_$(VERSION).tar.gz

# Where `vendor` writes: this package by default, or a staging copy (`build` sets it).
PKG_DIR ?= .
RUST_DIR = $(PKG_DIR)/src/rust

# The workspace crates the R crate needs, relative to this directory.
CRATES_DIR := ..
LOCAL_CRATES := fec-parser fec-parser-macros

# Keep macOS tar from adding AppleDouble `._*` files to the archives.
export COPYFILE_DISABLE := 1

.PHONY: document install test vendor build clean-vendor clean

document:
	Rscript -e 'devtools::document()'

install:
	$(R) CMD INSTALL .

test:
	Rscript -e 'devtools::test()'

# Copy the workspace crates into $(RUST_DIR)/local and vendor every other crate into
# $(RUST_DIR)/vendor.tar.xz, with the cargo source replacement in $(RUST_DIR)/vendor-config.toml
# (src/Makevars.in installs it as the build's cargo config).
#
# The copies keep everything the crates read at compile time (fec-parser-macros reads
# date_columns.txt, float-columns.txt and src/mappings2.json via CARGO_MANIFEST_DIR) and drop
# build output and test data. `--locked` fails if src/rust/Cargo.lock is stale: the tarball
# builds with `--offline`, so the committed lock must already be complete.
vendor:
	rm -rf "$(RUST_DIR)/local" "$(RUST_DIR)/vendor" "$(RUST_DIR)/vendor.tar.xz" "$(RUST_DIR)/vendor-config.toml"
	mkdir -p "$(RUST_DIR)/local"
	tar -C "$(CRATES_DIR)" -cf - \
	  --exclude 'target' --exclude 'tests/fixtures' --exclude 'tests/snapshots' --exclude '.DS_Store' \
	  $(LOCAL_CRATES) | tar -C "$(RUST_DIR)/local" -xf -
	cd "$(RUST_DIR)" && $(CARGO) vendor --locked --manifest-path Cargo.toml vendor > vendor-config.toml
	tar -C "$(RUST_DIR)" -cJf "$(RUST_DIR)/vendor.tar.xz" vendor
	rm -rf "$(RUST_DIR)/vendor"

# Build the source tarball from a staging copy (see the top of this file). The trap removes the
# staging directory however the recipe exits.
build:
	@set -e; \
	stage=$$(mktemp -d "$${TMPDIR:-/tmp}/libfec-build.XXXXXX"); \
	trap 'rm -rf "$$stage"' EXIT; \
	mkdir "$$stage/$(PKG)"; \
	echo "== staging $(PKG) in $$stage/$(PKG)"; \
	tar -cf - \
	  --exclude 'src/rust/target' --exclude 'src/rust/local' --exclude 'src/rust/vendor*' \
	  --exclude 'src/.cargo' --exclude 'src/Makevars' --exclude 'src/*.o' --exclude 'src/*.so' \
	  --exclude '$(PKG)_*.tar.gz' --exclude '.DS_Store' . | tar -C "$$stage/$(PKG)" -xf -; \
	sed -e 's#\.\./\.\./\.\./fec-parser#local/fec-parser#' \
	  src/rust/Cargo.toml > "$$stage/$(PKG)/src/rust/Cargo.toml"; \
	if grep -n '\.\./\.\./\.\.' "$$stage/$(PKG)/src/rust/Cargo.toml"; then \
	  echo "error: the staged src/rust/Cargo.toml still points outside the package"; exit 1; \
	fi; \
	$(MAKE) --no-print-directory vendor PKG_DIR="$$stage/$(PKG)"; \
	(cd "$$stage" && $(R) CMD build $(R_BUILD_ARGS) $(PKG)); \
	mv "$$stage/$(TARBALL)" .; \
	echo "== built $(TARBALL) ($$(du -h $(TARBALL) | cut -f1))"

# Remove what an in-place `make vendor` writes.
clean-vendor:
	rm -rf src/rust/local src/rust/vendor src/rust/vendor.tar.xz src/rust/vendor-config.toml

clean: clean-vendor
	rm -rf src/rust/target $(PKG)_*.tar.gz
