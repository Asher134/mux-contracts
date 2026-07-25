# Mux Protocol Contracts

This directory contains the Soroban smart contracts for the Mux Protocol. Each
crate compiles to a WASM module deployable on Stellar.

## Contract Index

| Crate | Description |
|---|---|
| [`mux-account`](mux-account/) | Smart wallet with owner, delegates, session keys, and spend limits |
| [`mux-account-factory`](mux-account-factory/) | Factory for deploying and registering account instances with metadata |
| [`mux-batcher`](mux-batcher/) | Atomic multi-operation batching with optional per-op failure handling |
| [`mux-delegation`](mux-delegation/) | Grant / revoke specific permissions or voting power to delegates |
| [`mux-permissions`](mux-permissions/) | Role-based access control (RBAC) — roles, permissions, grant/revoke |
| [`mux-policy`](mux-policy/) | Per-wallet daily spend-limit policy with auto-reset |
| [`mux-recovery`](mux-recovery/) | Account recovery with timelock and admin approval |
| [`mux-registry`](mux-registry/) | Contract version and metadata registry |
| [`mux-spending-policy`](mux-spending-policy/) | Per-account/per-asset spend-limit policy and validation |
| [`mux-wallet-registry`](mux-wallet-registry/) | Named wallet registry for address lookup |

### Shared Crate

| Crate | Description |
|---|---|
| [`soroban-test-helpers`](soroban-test-helpers/) | Shared mock environment and test utilities (not compiled to WASM) |

## Quick Reference

### Build

```bash
cargo build --target wasm32-unknown-unknown --release --workspace
```

### Test

```bash
cargo test --workspace --all-features
```

### Per-contract test

```bash
cargo test --package <crate>
```

## Common Patterns

All contracts follow these conventions:

- **`#![no_std]`** — no Rust standard library; WASM-target compatible.
- **Single error enum** — each contract defines one `#[contracterror]` enum with
  `#[repr(u32)]` codes starting at 1. Common variants (`NotInitialized = 1`,
  `AlreadyInitialized = 2`, `Unauthorized = 3`) appear in most contracts.
- **Storage griefing guards** — every collection-backed storage (Vec, Map) has an
  explicit `MAX_*` cap with a dedicated error variant.
- **TTL management** — persistent entries call `extend_ttl` on every write;
  instance storage is extended after state-mutating functions.
- **Event emission** — all state changes emit audit events under a contract-
  specific topic prefix.

## Error Codes

See [docs/error_codes.md](../docs/error_codes.md) for the full error code
reference across all contracts.

## TypeScript Bindings

Auto-generated clients for every contract live in [`../bindings/src/generated/`](../bindings/src/generated/).
After changing any contract interface, regenerate with:

```bash
bash scripts/generate-bindings.sh
```

## Documentation

- [Architecture Overview](../docs/architecture-overview.md)
- [Threat Model](../docs/threat-model.md)
- [Access Control Checklist](../docs/access-control-checklist.md)
- [Storage Griefing Notes](../docs/storage-griefing.md)
- [Error Codes Reference](../docs/error_codes.md)
- [Audit Prep](../docs/audit-prep.md)
