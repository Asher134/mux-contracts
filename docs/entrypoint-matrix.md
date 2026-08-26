# Entrypoint Matrix

This document classifies every `#[contractimpl]` entrypoint across the Mux Protocol
contracts as **admin** (requires stored admin/owner authorization), **user** (requires
caller authorization or specific actor auth), or **public** (no authorization required,
read-only queries).

Use this matrix when binding contracts from TypeScript or auditing the attack surface:
admin entrypoints must be called by the stored admin; user entrypoints must be called
by a specific actor; public entrypoints are callable by anyone.

## Legend

| Tag | Meaning |
|-----|---------|
| **A** | Admin / owner — requires the stored admin or owner address to authorize |
| **U** | User / actor — requires a specific caller (e.g. wallet, guardian, session key) to authorize |
| **P** | Public — no authorization required; read-only query |
| **R** | Read-only — no state mutation; may still require auth for actor-scoped reads |

## mux-account

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(owner, guardians)` | A | One-time setup; owner authorizes |
| `unpause()` | A | Owner only |
| `is_paused()` | P | Read-only |
| `set_delegate(delegate, expires_at, can_spend)` | A | Owner only; paused check; `expires_at` is a Unix timestamp |
| `remove_delegate(delegate)` | A | Owner only; paused check |
| `set_spend_limit(asset, amount, period)` | A | Owner only; paused check |
| `debit_spend(asset, spend)` | U | Caller (contract) authorizes; paused check; reentrancy guard |
| `execute(target, function, args, asset, spend)` | A | Owner only; paused check; validates spend limit, then invokes `target` while the reentrancy guard is held, then persists the debit (checks-effects-interactions) |
| `owner()` | P | Read-only |
| `delegates()` | P | Read-only; filters expired |
| `get_delegate(delegate)` | P | Read-only |
| `guardians()` | P | Read-only |
| `register_session_key(session_key, expires_at, scopes)` | A | Owner only; paused check; capped at `MAX_SESSION_KEYS` |
| `revoke_session_key(session_key)` | A | Owner only; paused check |
| `execute_with_session(session_key, payload)` | U | Session key auth; validates registration/revocation/expiry only — does not execute `payload` (see [aa_sequence_diagram.md](aa_sequence_diagram.md)) |
| `set_metadata(meta)` | A | Owner only |
| `get_metadata()` | P | Read-only |

## mux-account-factory

| Entrypoint | Auth | Notes |
|---|---|---|
| `deploy_account(owner, addr)` | U | Owner authorizes; enforces `MAX_ACCOUNTS_PER_OWNER = 64` cap |
| `deploy_account_with_metadata(owner, addr, ...)` | U | Owner authorizes; enforces cap and metadata string size limits |
| `simulate_deploy(owner, addr)` | P | Dry-run; no state mutation; mirrors same cap check as `deploy_account` |
| `simulate_deploy_with_metadata(owner, addr, ...)` | P | Dry-run; no state mutation; mirrors cap and metadata size checks |
| `get_accounts(owner)` | P | Read-only |
| `account_count()` | P | Read-only; global counter across all owners |
| `get_account_metadata(owner, addr)` | P | Read-only |
| `max_accounts_per_owner()` | P | Returns `MAX_ACCOUNTS_PER_OWNER` constant (64); allows clients to preflight cap checks |

## mux-batcher

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(admin)` | A | Optional, one-time; sets the upgrade admin only — batching works without it |
| `upgrade(new_wasm_hash)` | A | Admin only; `NotInitialized` if `initialize` was never called |
| `execute_batch(caller, ops)` | U | Caller authorizes; reentrancy guard |
| `submit_batch(ops)` | U | Delegates to `execute_batch` |
| `estimate_fees(op_count)` | P | Pure computation |
| `max_batch_size()` | P | Returns constant |
| `set_registry_metadata(desc, author)` | P | One-time; no auth required |
| `get_registry_metadata()` | P | Read-only |
| `simulate_batch(caller, ops)` | U | Caller authorizes; no state mutation |

## mux-delegation

| Entrypoint | Auth | Notes |
|---|---|---|
| `grant_delegate(owner, delegate, perms)` | U | Owner authorizes; capped at `MAX_DELEGATE_PERMS` / `MAX_DELEGATES_PER_OWNER` |
| `initialize(admin)` | A | Optional, one-time; sets the upgrade admin only — delegation grants work without it |
| `upgrade(new_wasm_hash)` | A | Admin only; `NotInitialized` if `initialize` was never called |
| `grant_delegate(owner, delegate, perms)` | U | Owner authorizes |
| `revoke_delegate(owner, delegate)` | U | Owner authorizes |
| `get_delegate_permissions(owner, delegate)` | P | Read-only |
| `is_delegate(owner, delegate, perm)` | P | Read-only |
| `get_delegates(owner)` | P | Read-only |
| `check_delegate(owner, delegate, perm)` | P | Read-only; `Ok(())`/`Err(NotADelegate)` variant of `is_delegate` |
| `link_contract_id(admin, contract_id)` | A | Admin authorizes; write-once |
| `link_contract_id(admin, contract_id)` | U | Caller-supplied `admin` param authorizes itself; **not** the same identity as the stored upgrade admin — see [delegation-upgrade.md](delegation-upgrade.md) |
| `get_contract_id()` | P | Read-only |

## mux-permissions

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(admin)` | A | One-time setup |
| `upgrade(new_wasm_hash)` | A | Admin only; `NotInitialized` if `initialize` was never called |
| `create_role(role, perms)` | A | Admin only |
| `grant_role(account, role)` | A | Admin only |
| `revoke_role(account, role)` | A | Admin only |
| `has_permission(account, perm)` | P | Read-only; emits `perm_ok` on grant only, nothing on denial |
| `get_roles(account)` | P | Read-only |
| `get_role_members(role)` | P | Read-only |
| `set_admin_threshold(threshold)` | A | Admin only |
| `propose_admin(new_admin)` | A | Admin only |
| `approve_admin(approver, new_admin)` | A | Admin + approver auth |
| `get_pending_admins()` | P | Read-only |
| `set_metadata(meta)` | A | Admin only |
| `get_metadata()` | P | Read-only |

## mux-policy

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(admin)` | A | One-time setup |
| `upgrade(new_wasm_hash)` | A | Admin only |
| `set_daily_limit(wallet, limit, day_ledgers, registry_id)` | A | Admin only |
| `get_daily_limit(wallet)` | P | Read-only; auto-resets counter |
| `record_spend(wallet, amount)` | U | Wallet authorizes |
| `reset_daily_counter(wallet)` | A | Admin only |

## mux-recovery

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(owner, guardians)` | U | Owner authorizes |
| `initiate_recovery(guardian, new_owner)` | U | Guardian authorizes; rejects if a non-expired recovery is already pending |
| `cancel_recovery()` | U | Owner authorizes |
| `execute_recovery(guardian)` | U | Guardian authorizes; timelock and expiry check |
| `approve_recovery_admin(co_guardian)` | U | Owner **and** a registered guardian co-sign; executes a pending recovery immediately, bypassing the timelock; requires both `owner.require_auth()` and `co_guardian.require_auth()` + guardian-membership check |
| `add_guardian(guardian)` | U | Owner authorizes; capped at `MAX_GUARDIANS` |
| `remove_guardian(guardian)` | U | Owner authorizes; rejects if it would leave zero guardians |
| `owner()` | P | Read-only |
| `guardians()` | P | Read-only |
| `recovery_status()` | P | Read-only |
| `recovery_request()` | P | Read-only; full request record or `None` |
| `set_registry(owner, registry_id)` | U | Owner authorizes |
| `registry_id()` | P | Read-only |

## mux-registry

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(admin)` | A | One-time setup |
| `register(name, version)` | A | Admin only |
| `register_with_metadata(name, version, desc, author, repo)` | A | Admin only |
| `check_version(name)` | P | Dry-run; no state mutation |
| `get_version(name)` | P | Read-only |
| `get_metadata(name)` | P | Read-only |
| `list_contracts()` | P | Read-only |

## mux-spending-policy

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(admin)` | A | One-time setup |
| `set_policy(account, asset, limit, period_ledgers)` | A | Admin only; resets `spent` to 0 |
| `get_policy(account, asset)` | P | Read-only |
| `check_spend(account, asset, amount)` | P | Read-only; no state mutation |

## mux-wallet-registry

| Entrypoint | Auth | Notes |
|---|---|---|
| `initialize(owner)` | U | Owner authorizes |
| `register_wallet(name, wallet)` | U | Owner authorizes |
| `register_wallet_with_metadata(name, wallet, label, desc)` | U | Owner authorizes; capped at `MAX_WALLETS` |
| `get_wallet(name)` | P | Read-only |
| `get_metadata(name)` | P | Read-only |
| `list_wallets()` | P | Read-only |
