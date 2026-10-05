# Build the NIF into gleam/priv/ and run the Gleam tests (macOS and Linux).
#
#   make                  release build -> gleam/priv/libfec_nif.so
#   make PROFILE=dev      debug build (cargo's target/debug)
#   make test             build, then `gleam test`
#
# erlang:load_nif/2 appends ".so" on every Unix (macOS included), so the
# cdylib is copied to libfec_nif.so whatever cargo called it.

EXT := $(if $(filter Darwin,$(shell uname)),dylib,so)
PROFILE ?= release
# cargo's output dir for the `dev` profile is `debug`; every other profile uses its own name.
PROFILE_DIR := $(if $(filter dev,$(PROFILE)),debug,$(PROFILE))
# Ask cargo rather than assume ../../target, so CARGO_TARGET_DIR is honoured.
TARGET_DIR := $(shell cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
NIF := $(TARGET_DIR)/$(PROFILE_DIR)/liblibfec_nif.$(EXT)

nif:
	cargo build -p fec-gleam --profile $(PROFILE)
	mkdir -p gleam/priv
	cp $(NIF) gleam/priv/libfec_nif.so

test: nif
	cd gleam && gleam test

clean:
	rm -rf gleam/priv gleam/build

.PHONY: nif test clean
