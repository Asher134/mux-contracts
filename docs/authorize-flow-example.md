# Authorization Flow Examples

This document provides concrete examples of the authorization patterns used across Mux Protocol contracts. These patterns ensure that only authorized parties can mutate on-chain state.

---

## 1. mux-account — Owner Authorization

The `mux-account` contract uses a `require_owner` helper that loads the stored owner address and calls `require_auth()` on it.

### Initialize

```rust
// Only the future owner can initialize the account.
client.initialize(&owner, &guardians);
// Internally: owner.require_auth() is called before any storage write.
```

### Set Delegate

```rust
// Only the owner can add or update delegates.
client.set_delegate(&delegate, &expiry_ledger, &can_spend);
// Internally: require_owner() → owner.require_auth()
```

### Set Spend Limit

```rust
// Only the owner can configure per-asset spend limits.
client.set_spend_limit(&asset, &amount, &period_ledgers);
// Internally: require_owner() → owner.require_auth()
```

### Delegate Spending

```rust
// debit_spend is contract-internal only (called by other Mux contracts).
// The caller must be the current contract address.
client.debit_spend(&asset, &spend);
// Internally: current_contract_address().require_auth()
```

### Authorization hierarchy

```
Owner
  ├── set_delegate(delegate, expiry, can_spend)
  ├── remove_delegate(delegate)
  ├── set_spend_limit(asset, amount, period)
  ├── set_metadata(meta)
  └── unpause()

Contract-internal (self)
  └── debit_spend(asset, spend)

Session key (TODO: not yet enforced)
  └── execute_with_session(session_key, payload)
```

---

## 2. mux-policy — Wallet Authorization

The `mux-policy` contract requires the wallet itself to authorize spend recording.

### Set Daily Limit (admin)

```rust
// Only the admin can configure spending limits.
client.set_daily_limit(&wallet, &limit, &day_ledgers, &None);
// Internally: require_admin() → admin.require_auth()
```

### Record Spend (wallet)

```rust
// Only the wallet itself can record a spend against its own limit.
client.record_spend(&wallet, &amount);
// Internally: wallet.require_auth()
//
// This means:
//   ✓ Wallet A can record_spend(wallet_a, 100)
//   ✗ Wallet B CANNOT record_spend(wallet_a, 100)
//   ✗ Admin CANNOT record_spend(wallet_a, 100)
//   ✗ A relayer CANNOT record_spend(wallet_a, 100)
```

### Reset Counter (admin)

```rust
// Only the admin can perform emergency resets.
client.reset_daily_counter(&wallet);
// Internally: require_admin() → admin.require_auth()
```

---

## 3. mux-registry — Admin Authorization

### Register Contract

```rust
// Only the admin can register or update contract versions.
client.register(&name, &version);
// Internally: require_admin() → admin.require_auth()
```

### Read Queries (no auth)

```rust
// Anyone can query the registry.
client.get_version(&name);       // no auth needed
client.get_metadata(&name);      // no auth needed
client.list_contracts();         // no auth needed
client.check_version(&name);     // no auth needed
```

---

## 4. mux-recovery — Guardian + Owner Authorization

### Initiate Recovery (guardian)

```rust
// Only a registered guardian can initiate recovery.
client.initiate_recovery(&guardian, &new_owner);
// Internally: guardian.require_auth() + require_guardian()
```

### Cancel Recovery (owner)

```rust
// Only the current owner can cancel a pending recovery.
client.cancel_recovery();
// Internally: require_owner() → owner.require_auth()
```

### Execute Recovery (guardian, after timelock)

```rust
// Only a registered guardian can execute after the timelock expires.
client.execute_recovery(&guardian);
// Internally: guardian.require_auth() + require_guardian()
// + checks: status == Pending && current_ledger >= executable_at
```

---

## 5. mux-delegation — Owner Authorization

### Grant Delegate

```rust
// Only the owner can grant permissions to a delegate.
client.grant_delegate(&owner, &delegate, &permissions);
// Internally: owner.require_auth()
```

### Revoke Delegate

```rust
// Only the owner can revoke delegate permissions.
client.revoke_delegate(&owner, &delegate);
// Internally: owner.require_auth()
```

---

## Pattern Summary

| Pattern | Helper | Used by | Auth target |
|---|---|---|---|
| `require_owner` | Load `Owner` from storage, call `require_auth()` | mux-account, mux-recovery, mux-wallet-registry | Contract owner |
| `require_admin` | Load `Admin` from storage, call `require_auth()` | mux-permissions, mux-registry, mux-policy, mux-spending-policy | Contract admin |
| `wallet.require_auth()` | Direct `require_auth()` on the wallet address | mux-policy (`record_spend`) | The wallet itself |
| `guardian.require_auth()` + `require_guardian()` | Auth + membership check in guardian set | mux-recovery | Registered guardian |
| `current_contract_address().require_auth()` | Self-auth for contract-internal calls | mux-account (`debit_spend`) | The contract itself |
