# Mux Protocol — Storage Griefing Notes

**Version:** 0.1.0  
**Date:** 2026-05-30  
**Related:** [Threat Model §4.5](threat-model.md#45-storage-griefing)

---

## What is storage griefing?

On Soroban, every contract pays **rent** for the ledger entries it occupies.  All three Mux contracts use **instance storage**, which is billed as a single rent unit shared across all callers.  Two distinct attack surfaces exist:

1. **Unbounded collection growth** — a privileged caller (owner, admin) floods a map or vec, inflating the rent cost for every other user of the contract.
2. **TTL expiry** — if no one extends the instance storage TTL, the entry expires and all contract state is silently lost.

---

## Mitigations in code

### Collection caps

| Contract | Collection | Key | Cap constant | Error on overflow |
|---|---|---|---|---|---|
| `mux-account` | `Delegates` map | `DataKey::Delegates` | `MAX_DELEGATES = 64` | `TooManyDelegates` |
| `mux-account` | `SessionKeyIndex` vec | `DataKey::SessionKeyIndex(owner)` | `MAX_SESSION_KEYS = 32` | `TooManySessionKeys` |
| `mux-account-factory` | `Accounts` vec (per owner) | `DataKey::Accounts(owner)` | `MAX_ACCOUNTS_PER_OWNER = 64` | `TooManyAccounts` |
| `mux-delegation` | `OwnerDelegates` vec | `DataKey::OwnerDelegates(owner)` | `MAX_DELEGATES_PER_OWNER = 128` | `TooManyDelegates` |
| `mux-delegation` | `DelegatePerms` vec | `DataKey::DelegatePerms(owner, delegate)` | `MAX_DELEGATE_PERMS = 64` | `TooManyPermissions` |
| `mux-permissions` | `RoleMembers` vec | `DataKey::RoleMembers(role)` | `MAX_ROLE_MEMBERS = 256` | `TooManyMembers` |
| `mux-permissions` | `AccountRoles` vec | `DataKey::AccountRoles(account)` | `MAX_ROLES_PER_ACCOUNT = 32` | `TooManyRoles` |
| `mux-wallet-registry` | `Names` vec | `DataKey::Names` | `MAX_WALLETS = 128` | `TooManyWallets` |

Caps are enforced on **new insertions only**; updates to existing entries are always allowed.

String size limits are enforced on metadata fields to prevent storage bloat through large strings.

### Vec-backed storage notes

Most Mux registries and indexes are stored as Soroban `Vec<T>` values under a
single instance (or persistent) key. Vec-backed collections are the primary
storage-griefing surface because each `push_back` grows the serialized blob
billed under that key.

**Rules for every vec-backed write path:**

1. **Cap before push** — compare `vec.len()` to the `MAX_*` constant and return
   the matching `TooMany*` error **before** calling `push_back`.
2. **Deduplicate on overwrite** — when a caller re-registers an existing entry
   (same wallet name, same delegate, same contract symbol), update in place
   and do **not** append a second copy to the index vec.
3. **Remove on revoke** — delete paths must `remove` the element from the index
   vec so revoked entries do not permanently inflate rent.
4. **Prefer instance storage for small shared indexes** — per-owner / per-role
   vecs that are enumerated often belong in instance storage with TTL extension
   on every write (see below). Large per-pair permission sets may use
   persistent storage but still require a hard cap.
5. **No unbounded append helpers** — never expose a public entrypoint that
   appends to a vec without both auth and a length check.

**Contract checklist (vec-backed):**

| Contract | Vec key | Cap check location | Overwrite / remove behavior |
|---|---|---|---|
| `mux-account-factory` | `Accounts(owner)` | `deploy_account` / `deploy_account_with_metadata` | Cap only; callers should avoid duplicate addresses |
| `mux-delegation` | `OwnerDelegates(owner)` | `grant_delegate` | Skip push when delegate already present; `revoke_delegate` removes |
| `mux-delegation` | `DelegatePerms(owner, delegate)` | `grant_delegate` | Full replace of permission vec (no append) |
| `mux-permissions` | `RoleMembers` / `AccountRoles` | `grant_role` | Caps on new membership only |
| `mux-registry` | `Names` | `register` / `register_with_metadata` | Name already present → update version/metadata only |
| `mux-wallet-registry` | `Names` | `register_wallet*` | Existing name overwrites wallet; count unchanged |

When adding a new vec-backed `DataKey`, update this table, add a unit test that
fills to the cap and asserts the exact `TooMany*` error, and document the
constant in [`docs/abi_reference.md`](abi_reference.md).

### TTL auto-extension

Every write path in the Mux contracts calls:

```rust
env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
```

| Constant | Value | Approximate duration |
|---|---|---|
| `TTL_THRESHOLD` | 17,280 ledgers | ~1 day (5 s/ledger) |
| `TTL_EXTEND_TO` | 518,400 ledgers | ~30 days |

This means the TTL is bumped to 30 days whenever the remaining TTL drops below 1 day **and** a write occurs.  Contracts that are not written to for more than 30 days will expire unless a keeper extends the TTL externally.

---

## Deployment runbook — TTL keeper

> **Required before mainnet deployment.**

A keeper job must periodically call `extend_ttl` on each contract's instance storage to prevent expiry during idle periods.

Recommended approach using the Stellar CLI:

```bash
stellar contract extend \
  --id <CONTRACT_ID> \
  --ledgers-to-extend 518400 \
  --source <KEEPER_KEYPAIR> \
  --network mainnet
```

Run this job at least once every **25 days** to stay ahead of the 30-day TTL window.

---

## Storage sizing reference

| Collection | Entry size (approx.) | Cap | Max storage |
|---|---|---|---|
| `Delegates` map | ~72 bytes | 64 | ~4.6 KB |
| `Accounts` vec (per owner, factory) | ~32 bytes | 64 | ~2 KB |
| `RoleMembers` vec | ~32 bytes | 256 | ~8 KB |
| `AccountRoles` vec | ~8 bytes | 32 | ~256 bytes |
| `Names` vec (`mux-registry`) | ~16 bytes | 128 | ~2 KB |
| `SessionKeyIndex` vec (per owner) | ~32 bytes | 32 | ~1 KB |
| `SpendLimit` per asset | ~80 bytes | owner-controlled | unbounded (owner only) |
| `Wallet` entries | ~42–50 bytes | 256 | ~12 KB |

`SpendLimit` keys are written only by the contract owner and are not publicly writable, so no cap is enforced.  Owners should avoid registering an excessive number of distinct assets.

---

## Threat cross-reference

| Threat ID | Description | Mitigation |
|---|---|---|
| T-17 | Owner floods delegate map | `MAX_DELEGATES = 64` in `set_delegate` |
| T-18 | Admin floods role member list | `MAX_ROLE_MEMBERS = 256` in `grant_role` |
| T-19 | Admin assigns excessive roles to one account | `MAX_ROLES_PER_ACCOUNT = 32` in `grant_role` |
| T-20 | Spend limits accumulate unbounded per-asset keys | No public write path; owner-only |
| T-21 | Instance storage TTL expiry causes silent data loss | `extend_ttl` on every write + keeper job |
| T-22 | Owner floods wallet registry with distinct names | `MAX_WALLETS = 128` in `register_wallet` |
| T-23 | Owner floods session key index for an account | `MAX_SESSION_KEYS = 32` in `require_session_key_cap` |

---

## Test Coverage for Capacity Guards

Every collection cap has dedicated unit tests that verify the `TooMany*` error path:

| Contract | Test | What it verifies |
|---|---|---|
| `mux-account` | `test_delegate_cap_enforced` | 65th new delegate returns `TooManyDelegates` |
| `mux-account` | `test_delegate_cap_allows_update` | Updating an existing delegate at cap succeeds |
| `mux-delegation` | `test_too_many_delegates_rejected` | 129th delegate returns `TooManyDelegates` |
| `mux-delegation` | `test_grant_too_many_permissions_fails` | 65th permission returns `TooManyPermissions` |
| `mux-permissions` | `test_role_member_cap_enforced` | 257th member returns `TooManyMembers` |
| `mux-permissions` | `test_roles_per_account_cap_enforced` | 33rd role returns `TooManyRoles` |
| `mux-wallet-registry` | `test_register_wallet_caps_names` | 129th wallet returns `TooManyWallets` |
| `mux-registry` | `test_too_many_contracts_via_register` | 129th name via `register()` returns `TooManyContracts` |
| `mux-registry` | `test_too_many_contracts_via_register_with_metadata` | 129th name via `register_with_metadata()` returns `TooManyContracts` |
| `mux-registry` | `test_register_existing_at_capacity_succeeds` | Updating existing name at cap succeeds |
| `mux-registry` | `test_list_contracts_count_at_boundary` | `list_contracts().len() == 128` at capacity |
| `mux-registry` | `test_get_version_after_capacity_filled` | All 128 names are queryable |
| `mux-registry` | `test_register_cross_path_update_no_duplicate` | `register()` + `register_with_metadata()` same name = 1 entry |
| `mux-policy` | `test_set_daily_limit_wallet_cap` | 257th wallet returns `TooManyWallets` |
