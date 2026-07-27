# Architecture Overview

This document provides a high-level overview of the Mux Protocol architecture. The system is composed of several interoperating smart contracts on the Soroban network that together enable a flexible account abstraction layer.

## Contract Architecture

The core contracts in the Mux Protocol workspace include:

- **mux-account**: The core smart account implementation, enabling abstracted logic and custom validation.
- **mux-account-factory**: Responsible for deterministic deployment and initialization of new Mux accounts.
- **mux-batcher**: A utility contract for batching multiple operations or contract calls into a single transaction.
- **mux-permissions**: A module for defining and enforcing role-based access control and granular permissions within Mux accounts.
- **mux-registry**: A central registry for discovering, verifying, and indexing components, accounts, and valid module implementations.
- **mux-wallet-registry**: A named address book that maps symbolic names to wallet addresses. Only a designated owner may write entries; reads are permissionless.
- **mux-recovery**: Guardian-initiated account recovery with a mandatory 24-hour timelock. Allows pre-registered guardians to transfer ownership to a new address while giving the current owner a cancellation window. Optionally linked to a `mux-registry` deployment for auditability.
- **mux-delegation**: Scoped permission delegation from an owner to one or more delegate addresses. Supports up to 128 delegates per owner and 64 permissions per delegate, with full enumeration and per-permission checks.

## Diagram

```mermaid
graph TD
    User([User / DApp]) --> Batcher[mux-batcher]
    User --> Factory[mux-account-factory]

    Factory -->|Deploys & Initializes| Account[mux-account]

    Batcher -->|Executes batch| Account

    Account -->|Validates Actions| Permissions[mux-permissions]

    Account -->|Looks up Modules| Registry[mux-registry]
    Factory -->|Registers| Registry
    Permissions -.->|Verified via| Registry
    Account -->|Resolves wallets| WalletRegistry[mux-wallet-registry]

    Recovery[mux-recovery] -->|Links to| Registry
    Account -->|Delegates to| Delegation[mux-delegation]
    User -->|Initiates recovery via| Recovery
```

## System Flow

1. **Deployment**: Users interact with the `mux-account-factory` to deploy a new smart account deterministically.
2. **Execution**: Transactions can be sent individually or batched via the `mux-batcher` to optimize gas and latency.
3. **Validation**: The `mux-account` routes calls through `mux-permissions` to ensure the caller has the appropriate rights.
4. **Registry**: The `mux-registry` acts as the source of truth for protocol-wide configurations, valid plugin implementations, and discovery.
5. **Recovery**: `mux-recovery` enables guardian-initiated ownership transfer with a timelock cancellation window. The contract can be linked to a `mux-registry` entry for auditability (see [`docs/recovery-trust-model.md`](recovery-trust-model.md)).
6. **Delegation**: `mux-delegation` allows account owners to grant scoped permissions to delegate addresses, enabling fine-grained access control without transferring ownership.

