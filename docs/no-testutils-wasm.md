# No testutils in release WASM

Release / deployable Soroban WASM **must not** include the `soroban-sdk` `testutils` feature. That feature pulls host-only test APIs unsuitable for on-chain bytecode and inflates audit surface.

## Rules

| Layer | Allowed? | Notes |
|---|---|---|
| `[dependencies]` of `mux-*` crates | **No** | `soroban-sdk` must be feature-free |
| `[dev-dependencies]` | Yes | Used by `#[cfg(test)]` unit tests only |
| Optional crate feature `testutils` | Yes | Opt-in for local testing; never passed to `build-wasm.sh` |
| `soroban-test-helpers` | Yes (rlib only) | Always enables `testutils`; excluded from WASM package list |

`#[cfg(test)]` modules are stripped by the Rust compiler when building `cdylib` targets and do not appear in release WASM.

## How release builds stay clean

1. `scripts/build-wasm.sh` builds **only** `mux-*` contract packages (explicit `-p` list).
2. It never passes `--features` / `--all-features`.
3. After a release build it runs `scripts/check-no-testutils.sh`.

## Verification

```bash
# Cargo.toml + optional WASM string scan
make check-no-testutils

# Full release build + automatic check
make wasm

# Script unit tests (no cargo required)
bash scripts/test-check-no-testutils.sh
```

`check-no-testutils.sh` fails if:

- Any `contracts/mux-*/Cargo.toml` enables `testutils` under `[dependencies]`, or
- `soroban-test-helpers` is marked `cdylib`, or
- A built `.wasm` contains the ASCII string `testutils`.

## Bindings note

TypeScript clients bind the release WASM ABI. They must not assume test-only helpers exist on-chain. Keep `testutils` out of the packages published via `scripts/generate-bindings.sh`.
