.PHONY: all build test clean fmt lint clippy wasm check-sizes coverage deploy-dry-run deploy-ci

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

# LLVM source-based coverage. Prints a report stub if llvm-tools-preview is missing.
coverage:
	bash scripts/coverage.sh

# Simulate a full deployment without submitting any on-chain transactions.
# No secret keys or live network access required. Useful for local validation. (#449)
deploy-dry-run:
	bash scripts/deploy.sh --dry-run

# Deploy using pre-built WASM artifacts, skipping the cargo build step.
# Intended for CI pipelines that cache build outputs between jobs. (#450)
deploy-ci:
	bash scripts/deploy.sh --skip-build
