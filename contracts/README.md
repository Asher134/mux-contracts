# Mux Protocol Contracts

This directory contains the smart contracts for the Mux Protocol, a comprehensive account management and permission system for Stellar.

## Contracts

### Core Contracts

- **mux-account**: Smart account implementation with session management
- **mux-account-factory**: Factory contract for creating new Mux accounts
- **mux-permissions**: Role-based access control (RBAC) system
- **mux-registry**: Contract registry for protocol management
- **mux-batcher**: Batch transaction processing

### New Contracts

- **mux-recovery**: Account recovery system for compromised or lost accounts
- **mux-delegation**: Delegation system for permissions and voting power
- **mux-spending-policy**: Contract for storing and checking per-account spend limits

## mux-spending-policy

The spending-policy contract stores per-account/per-asset spend limits and validates spend requests against them.

### Features

- **Policy Management**: Admin can create or update spend policies for account/asset pairs
- **Policy Reads**: Callers can retrieve the current policy for a specific account/asset pair
- **Spend Validation**: `check_spend` rejects requests that exceed the configured limit
- **Input Validation**: Non-positive limits and negative spend amounts are rejected with `InvalidInput`

### Key Functions

- `initialize(admin)`: Initialize the contract with an admin
- `set_policy(account, asset, limit)`: Store or replace a spend policy for an account/asset pair
- `get_policy(account, asset)`: Retrieve the configured policy for an account/asset pair
- `check_spend(account, asset, amount)`: Validate a spend request against the configured policy

### Errors

- `PolicyNotFound`: No spend policy exists for the provided account/asset pair
- `SpendLimitExceeded`: The requested spend is above the configured limit
- `InvalidInput`: The provided limit is not positive or the requested spend is negative

## mux-recovery

The recovery contract provides a secure mechanism for account recovery when accounts are compromised or access is lost.

### Features

- **Recovery Request Struct**: Comprehensive tracking of recovery requests with metadata
- **Admin Approval System**: Only authorized administrators can approve recovery requests
- **Event Emission**: All recovery actions emit events for transparency and auditability
- **Request Management**: Track pending and completed recovery requests

### Key Functions

- `initialize(admin)`: Initialize the contract with an admin
- `request_recovery(old_account, new_account)`: Submit a recovery request
- `approve_recovery(request_id)`: Admin function to approve recovery requests
- `get_recovery_request(request_id)`: Retrieve recovery request details
- `get_pending_requests()`: List all pending recovery requests

### Events

- `init`: Contract initialization
- `req_sub`: Recovery request submitted
- `req_app`: Recovery request approved

## mux-delegation

The delegation contract enables accounts to delegate specific permissions or voting power to other accounts.

### Features

- **Delegation Management**: Grant and revoke delegations with specific permissions
- **Event Emission**: Emits `delegate_granted` events as required
- **Permission Checking**: Verify delegated permissions
- **Delegation Limits**: Prevents storage griefing with reasonable limits
- **Bidirectional Tracking**: Track both delegators and delegates

### Key Functions

- `initialize(admin)`: Initialize the contract with an admin
- `grant_delegation(delegator, delegate, permissions)`: Grant delegation with specific permissions
- `revoke_delegation(delegator, delegate)`: Revoke an existing delegation
- `has_delegation(delegator, delegate)`: Check if delegation exists and is active
- `has_delegated_permission(delegator, delegate, permission)`: Check specific permission
- `get_delegates(delegator)`: Get all delegates for an account
- `get_delegators(delegate)`: Get all delegators for an account

### Events

- `init`: Contract initialization
- `del_grant`: Delegation granted (the required `delegate_granted` event)
- `del_revok`: Delegation revoked

## Security Features

Both contracts implement:

- **Storage TTL Management**: Automatic TTL extension to prevent data loss
- **Access Control**: Proper authorization checks
- **Storage Griefing Protection**: Limits on data structures to prevent abuse
- **Comprehensive Testing**: Full test coverage for all functionality
- **Event Emission**: Transparent logging of all actions

## Testing

Run tests for individual contracts:

```bash
cargo test --package mux-recovery
cargo test --package mux-delegation
```

Or test all contracts:

```bash
cargo test
```

## Integration

These contracts follow the same patterns as existing Mux Protocol contracts:

- Consistent error handling with custom error types
- Soroban SDK best practices
- Storage optimization with TTL management
- Comprehensive event emission for auditability
- Modular design for easy integration

## `no_std` and `alloc` Constraints

All Soroban contract crates in this workspace are `#![no_std]`. The workspace
`Cargo.toml` sets `unsafe_code = "forbid"` at the workspace level, so no
crate may use `unsafe` blocks.

### Why `no_std`?

Soroban smart contracts compile to WASM (`wasm32-unknown-unknown`) and run
inside the Soroban VM, which does not provide a system allocator or OS
services. Using `no_std` ensures:

1. **Correct compilation target** — the WASM target has no `std` library.
2. **No hidden syscalls** — prevents accidental use of file I/O, networking,
   or other OS primitives unavailable on-chain.
3. **Smaller binary size** — `no_std` binaries are typically smaller, reducing
   deployment costs.

### `extern crate alloc`

Only `mux-registry` currently uses `extern crate alloc`. This is allowed
because the Soroban VM provides a heap allocator, and `alloc` types
(`Vec`, `String`, `BTreeMap`, etc.) are safe to use in a `no_std` context
when an allocator is available.

Other contracts avoid `alloc` and rely exclusively on Soroban SDK types
(`soroban_sdk::Vec`, `soroban_sdk::String`, etc.) which are backed by the
Soroban host and do not require the Rust `alloc` crate.

### Constraints for contributors

- **Never add `extern crate std`** to any contract crate.
- **Never add `unsafe` code** — the workspace-level `forbid` enforces this.
- **Prefer `soroban_sdk` collection types** over `alloc` types for
  consistency and gas predictability.
- If `alloc` is needed, document why in the crate-level doc comment and
  ensure the crate still compiles to `wasm32-unknown-unknown`.