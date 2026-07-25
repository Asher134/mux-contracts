# Mainnet Immutable Flag Guidance

**Version:** 0.1.0  
**Date:** 2026-07-25  
**Status:** Draft  
**Related:** [Mainnet Deploy Checklist](mainnet-deploy-checklist.md), [Contract Upgrade Pattern](contract-upgrade-pattern.md)

---

## Purpose

This document defines when and how Mux Protocol contracts should be treated as immutable on mainnet, and provides guidance for contracts that intentionally expose upgrade paths versus those that do not.

---

## Immutability model

On Soroban, contracts are **immutable by default** — WASM bytecode cannot be changed after deployment unless the contract explicitly exposes an `upgrade()` entry point. This is a security property, not a limitation.

| Immutability level | Meaning | When to use |
|--------------------|---------|-------------|
| **Fully immutable** | No `upgrade()` function; contract code can never change | Core contracts with stable interfaces, audited and battle-tested |
| **Conditionally upgradeable** | `upgrade()` exists, gated by admin auth | Contracts where feature evolution is expected (e.g., policy rules) |
| **Proxied / replaceable** | Contract delegates to an implementation contract that can be swapped | Complex systems needing hot-swap without state migration |

---

## Current contract immutability status

| Contract | Upgradeable | Rationale |
|----------|-------------|-----------|
| `mux-account` | **No** | Core AA logic; immutability is a user trust guarantee |
| `mux-account-factory` | **No** | Factory registration logic is stable; no evolution expected |
| `mux-batcher` | **No** | Atomic batching semantics are fixed by protocol design |
| `mux-delegation` | **No** | Delegate permission model is audited and stable |
| `mux-permissions` | **No** | RBAC registry with stable interface |
| `mux-policy` | **Yes** | Policy rules may evolve; admin-gated `upgrade()` exposed |
| `mux-recovery` | **No** | Recovery timelock is time-critical; upgrade path requires careful design |
| `mux-registry` | **No** | Version registry with stable interface |
| `mux-spending-policy` | **No** | Spend limit logic is stable |
| `mux-wallet-registry` | **No** | Wallet registration is stable |

---

## Making a contract immutable on mainnet

### Option A: Do not expose `upgrade()` (recommended for most contracts)

Simply omit the `upgrade()` function from the contract. Without it, there is no on-chain mechanism to replace the WASM. The contract is immutable from the moment it is deployed.

```rust
// No upgrade() function means no way to change the code.
// This is the strongest immutability guarantee.
```

### Option B: Remove `upgrade()` before mainnet deploy

If a contract currently has `upgrade()` for testing but should be immutable on mainnet:

1. Remove the `upgrade()` function from the `#[contractimpl]` block
2. Optionally remove `DataKey::Admin` if it is only used for upgrade auth
3. Rebuild WASM with `cargo build --target wasm32-unknown-unknown --release`
4. Deploy the new WASM to mainnet (this is the only deploy — no prior upgrade path exists)

### Option C: Admin-revocable upgrade authority

For contracts where upgrade authority should exist but be revocable:

1. Store admin in instance storage during `initialize()`
2. Add a `renounce_upgrade_authority()` function that:
   - Calls `require_admin()`
   - Removes `DataKey::Admin` from storage
   - Once removed, `upgrade()` can never succeed again

```rust
pub fn renounce_upgrade_authority(env: Env) -> Result<(), ContractError> {
    Self::require_admin(&env)?;
    env.storage().instance().remove(&DataKey::Admin);
    // Note: upgrade() will now always fail because require_admin() reads from
    // DataKey::Admin which no longer exists.
    Ok(())
}
```

---

## Why immutability matters on mainnet

| Concern | How immutability helps |
|---------|----------------------|
| **User trust** | Users can verify the contract code matches the audited WASM hash and know it cannot change |
| **Composability** | Other protocols can safely integrate with a known, fixed interface |
| **Audit scope** | Auditors review a fixed codebase; no risk of post-audit behaviour change |
| **Regulatory** | Immutable contracts have clearer legal treatment in some jurisdictions |
| **Attack surface** | No upgrade key to compromise; no admin auth to phish for upgrades |

---

## Trade-offs

| Factor | Immutable | Upgradeable |
|--------|-----------|-------------|
| Bug fixes | Requires redeploy + state migration | In-place WASM swap |
| Feature evolution | Redeploy + migrate users | Add functions in new WASM |
| User confidence | Higher (code is fixed) | Lower (admin could change behaviour) |
| Operational complexity | Higher for protocol changes | Lower for hot-fixes |

---

## Recommended approach by contract type

| Contract type | Recommendation |
|---------------|----------------|
| Core accounting (mux-account) | **Immutable** — user funds depend on fixed logic |
| Factory / registry (mux-account-factory, mux-registry) | **Immutable** — registration logic is stable |
| Policy / spending (mux-policy) | **Conditionally upgradeable** — rules may evolve with governance |
| Recovery (mux-recovery) | **Immutable** — timelock logic is security-critical |
| Permissions (mux-permissions) | **Immutable** — RBAC is audited and stable |
| Batcher (mux-batcher) | **Immutable** — atomic batching semantics are fixed |

---

## Verification

After deploying an immutable contract on mainnet:

1. Record the WASM hash in `CONTRACT_IDS.md`
2. Verify the hash matches the built WASM: `scripts/verify-wasm-hash.sh`
3. Confirm no `upgrade()` function exists in the contract ABI
4. Document the immutability decision in release notes
5. Add the contract to the immutability registry below

---

## Immutability registry

Once a contract is confirmed immutable on mainnet, record it here:

| Contract | Network | WASM hash | Deploy date | Immutable since |
|----------|---------|-----------|-------------|-----------------|
| | | | | |

---

## References

- [Mainnet Deploy Checklist](mainnet-deploy-checklist.md)
- [Contract Upgrade Pattern](contract-upgrade-pattern.md)
- [Soroban Contract Docs](https://developers.stellar.org/docs/build/smart-contracts)
