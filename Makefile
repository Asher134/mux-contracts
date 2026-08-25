.PHONY: all build test clean fmt lint clippy wasm check-sizes size-check \
        coverage coverage-ci \
        deny \
        check-no-testutils \
        verify-wasm-hashes \
        deploy-dry-run deploy-ci

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

# Supply-chain license and advisory check. deny.toml controls policy. (#661)
# Requires: cargo install cargo-deny
deny:
	cargo deny check

# Ensure no mux-* Cargo.toml enables soroban-sdk testutils in [dependencies]
# and that built WASMs (if present) contain no testutils bytes. (#663)
check-no-testutils:
	bash scripts/check-no-testutils.sh

# Compute SHA-256 hashes for every built release WASM and print them. (#664)
# Run after `make wasm`. Uses scripts/verify-wasm-hash.sh --compute-only.
verify-wasm-hashes:
	bash scripts/compute-wasm-hashes.sh

# LLVM source-based coverage using cargo-llvm-cov when available; falls back
# to the legacy stub if the tool is not installed. (#662)
# Requires: cargo install cargo-llvm-cov && rustup component add llvm-tools-preview
coverage:
	bash scripts/coverage.sh

# CI coverage target: always produces LCOV output to coverage/lcov.info. (#662)
# Fails if cargo-llvm-cov is not installed (install it in the CI job first).
coverage-ci:
	bash scripts/coverage.sh --lcov

# Simulate a full deployment without submitting any on-chain transactions.
# No secret keys or live network access required. Useful for local validation. (#449)
deploy-dry-run:
	bash scripts/deploy.sh --dry-run

# Deploy using pre-built WASM artifacts, skipping the cargo build step.
# Intended for CI pipelines that cache build outputs between jobs. (#450)
deploy-ci:
	bash scripts/deploy.sh --skip-build
