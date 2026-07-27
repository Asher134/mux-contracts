.PHONY: all build test clean fmt lint clippy wasm check-sizes size-check coverage

all: fmt lint build test

build:
	cargo build --workspace --all-targets

test:
	cargo test --workspace

clean:
	cargo clean

fmt:
	cargo fmt --all -- --check

lint: clippy

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

wasm:
	bash scripts/build-wasm.sh --release

check-sizes: wasm
	bash scripts/check-contract-sizes.sh

# Alias so both spellings work: `make check-size` and `make check-sizes`
size-check: check-sizes

# LLVM source-based coverage. Prints a report stub if llvm-tools-preview is missing.
coverage:
	bash scripts/coverage.sh
