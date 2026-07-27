# Spending Policy Contract Semantics

This document describes the design, data model, and behavioral guarantees of the `mux-spending-policy` contract.

## Overview

`mux-spending-policy` stores per-account spend limits per asset and validates spend requests against them. It is a lightweight enforcement contract that can be queried by other contracts (such as `mux-account`) before executing a spend operation.

The contract does **not** track cumulative spend or reset counters — it enforces a simple maximum-spendable-amount check. For time-window-based limits (e.g. daily limits), see the `mux-policy` contract.

## Data Model

### `SpendLimit`

```rust
pub struct SpendLimit {
    pub asset: Address,  // Asset identifier (the token)
    pub limit: i128,     // Maximum amount spendable
}
```

Storage key: `DataKey::SpendLimit(account: Address, asset: Address)` — instance storage, one record per (account, asset) pair.

## Functions

### `initialize(admin)`

- One-time setup. Stores the admin address.
- Fails with `AlreadyInitialized` if called more than once.
- Emits `init` event.

### `set_policy(account, asset, limit)` — admin only

- Creates or replaces the spend limit for `account`/`asset`.
- `limit` must be strictly positive (> 0).
- Fails with `InvalidInput` if `limit <= 0`.
- Fails with `NotInitialized` if called before `initialize`.
- Emits `lmt_set` event.

### `get_policy(account, asset)`

- Returns the stored `SpendLimit` record.
- Fails with `PolicyNotFound` if no policy is configured for the pair.
- Read-only — no auth required, no event emitted.

### `check_spend(account, asset, amount)`

- Checks whether `amount` is within the configured spend limit.
- `amount` must be non-negative.
- Fails with `InvalidInput` if `amount < 0`.
- Fails with `PolicyNotFound` if no policy is configured.
- Fails with `SpendLimitExceeded` if `amount > policy.limit`.
- Emits events:
  - `chk_ok` when the spend is within the limit.
  - `chk_ex` when the spend exceeds the limit or policy is not found.

## Error Codes

| Code | Variant | Meaning |
|---|---|---|
| 1 | `NotInitialized` | Contract not yet initialized |
| 2 | `AlreadyInitialized` | `initialize` called more than once |
| 3 | `Unauthorized` | Caller is not the admin |
| 4 | `PolicyNotFound` | No policy configured for account/asset pair |
| 5 | `SpendLimitExceeded` | Spend exceeds the configured limit |
| 6 | `InvalidInput` | Limit ≤ 0 or spend amount negative |

## Events

All state-mutating operations emit a structured event with topics `[mux_spend, action]`:

| Action | Emitted by | Data |
|---|---|---|
| `init` | `initialize` | `admin: Address` |
| `lmt_set` | `set_policy` | `(account: Address, asset: Address, limit: i128)` |
| `chk_ok` | `check_spend` (within limit) | `(account: Address, asset: Address, amount: i128)` |
| `chk_ex` | `check_spend` (exceeds limit / no policy) | `(account: Address, asset: Address, amount: i128, limit_or_reason: i128 \| Symbol)` |

## Authorization Requirements

| Function | Auth check | Who can call |
|---|---|---|
| `initialize` | `admin.require_auth()` | Deployer (once) |
| `set_policy` | `require_admin()` → `admin.require_auth()` | Admin only |
| `get_policy` | None | Anyone (read-only) |
| `check_spend` | None | Anyone (read-only) |

## Storage TTL

Instance storage TTL is extended on every write (`TTL_THRESHOLD = 17 280`, `TTL_EXTEND_TO = 518 400` ledgers ≈ 30 days). Deployers should run a keeper job to extend TTL proactively.

## Key Invariants

- Policy records are keyed by (account, asset) — each account can have separate limits per asset.
- Limits are absolute maximums, not cumulative per time window.
- `check_spend` is a pure read-only check and does **not** modify state.
- There is no storage griefing concern: each set_policy call replaces the single record for that (account, asset) pair.

