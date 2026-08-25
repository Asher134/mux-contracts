# Developer Onboarding Guide

Welcome to the Mux Protocol! This guide will help you set up your local development environment and start contributing to the codebase.

## Prerequisites
Ensure you have the following installed:
- [Rust & Cargo](https://rustup.rs/) (latest stable version)
- Node.js & npm (for auxiliary scripting and testing)
- Git

## Getting Started

1. **Clone the Repository**
   ```bash
   git clone https://github.com/your-org/mux-contracts.git
   cd mux-contracts
   ```

2. **Build the Contracts**
   We use Cargo to build the smart contracts. A custom `Makefile` is also provided for convenience.
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

3. **Run Tests**
   Ensure all tests are passing before creating a pull request.
   ```bash
   cargo test
   ```

4. **Coverage (optional)**
   ```bash
   # Install cargo-llvm-cov once (bundles its own llvm tools):
   cargo install cargo-llvm-cov --locked
   rustup component add llvm-tools-preview

   make coverage          # summary to stdout
   make coverage-ci       # LCOV output to coverage/lcov.info (same as CI)

   # Or via the script directly:
   bash scripts/coverage.sh --html --open   # HTML report
   bash scripts/coverage.sh --lcov          # LCOV for upload

   # Stub-only (no instrumented test run; for script validation):
   bash scripts/coverage.sh --stub
   bash scripts/test-coverage.sh
   ```
   Without `cargo-llvm-cov` the script prints a coverage report stub listing
   workspace crates and exits 0 so local dev machines without the tooling are
   not blocked.

## Cargo.lock

Commit `Cargo.lock` with dependency changes. See [CONTRIBUTING.md](../CONTRIBUTING.md#cargolock-policy)
for the full policy (reproducible WASM builds, CI cache keys, deny checks).

## Repository Structure
- `contracts/`: Source code for the Mux Protocol smart contracts.
- `docs/`: Architecture and API documentation.
- `tests/`: Integration and end-to-end tests.

## Contribution Guidelines
1. Create a feature branch from `main`.
2. Ensure you follow standard formatting and linting:
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   ```
3. Submit a Pull Request and request a review.

## Architecture
For a high-level overview of the protocol, check out the [Dependency Graph](dependency_graph.md).
