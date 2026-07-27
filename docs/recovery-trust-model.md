# mux-recovery — Recovery Trust Model

**Version:** 0.1.0
**Status:** Living document — update whenever the recovery contract changes.

---

## 1. Purpose

`mux-recovery` provides social recovery for `mux-account` owners. If an owner loses access to their private key, a quorum of pre-registered guardians can transfer ownership to a new address after a mandatory timelock delay.

---

## 2. Roles and Trust Levels

| Role | Who | Trust Level | Capabilities |
|---|---|---|---|
| **Owner** | Account holder | Highest | Cancel any pending recovery; normal account operations |
| **Guardian** | Trusted contacts set by owner at init | High | Initiate and execute recovery |
| **Stranger** | Any other address | None | No recovery operations |

> **Key invariant:** Guardians are set at `initialize` time and are immutable. The owner cannot change guardians after deployment (prevents a compromised owner from removing guardians before an attack).

---

## 3. Recovery Lifecycle

```
         Guardian calls
         initiate_recovery()
               │
               ▼
         ┌──────────┐
         │ PENDING  │ ◄─── Owner can cancel_recovery() at any time
         └──────────┘
               │
    RECOVERY_TIMELOCK ledgers elapse
    (~24 hours at 5s close time)
               │
               ▼
    Guardian calls execute_recovery()
               │
               ▼
         ┌──────────┐
         │ EXECUTED │  ownership transferred to new_owner
         └──────────┘

    OR

    Owner calls cancel_recovery()
               │
               ▼
         ┌───────────┐
         │ CANCELLED │  no ownership change
         └───────────┘
```

### State transitions

| From | Event | To | Who |
|---|---|---|---|
| *(none)* | `initiate_recovery` | `Pending` | Guardian |
| `Pending` | `execute_recovery` (after timelock) | `Executed` | Guardian |
| `Pending` | `cancel_recovery` | `Cancelled` | Owner |
| `Executed` | — | *(terminal)* | — |
| `Cancelled` | `initiate_recovery` | `Pending` | Guardian (new request) |

---

## 4. Security Properties

### 4.1 Timelock (24-hour cancellation window)

`RECOVERY_TIMELOCK = 17_280` ledgers ≈ 24 hours.

The timelock is the primary defence against a compromised guardian set. Even if all guardians collude to steal the account, the legitimate owner has 24 hours to observe the `rec_init` event on-chain and call `cancel_recovery`.

**Assumption:** The owner monitors their account (or has an automated watcher) at least once every 24 hours.

### 4.2 Single active request

Only one `Pending` request may exist at a time. A second `initiate_recovery` call while a request is `Pending` returns `RecoveryAlreadyPending`. This prevents guardians from flooding the contract with requests to confuse the owner.

### 4.3 Guardian-only initiation and execution

`initiate_recovery` and `execute_recovery` both verify the caller is in the guardian set via `require_auth` + membership check. A non-guardian call returns `Unauthorized`.

### 4.4 Owner-only cancellation

`cancel_recovery` requires `owner.require_auth()`. Only the current owner can cancel, preventing guardians from cancelling their own recovery attempt.

### 4.5 Audit events

Every state mutation emits a structured event:

| Entrypoint | Event topic | Data |
|---|---|---|
| `initialize` | `init` | `owner: Address` |
| `initiate_recovery` | `rec_init` | `(guardian: Address, new_owner: Address)` |
| `execute_recovery` | `rec_exec` | `(guardian: Address, new_owner: Address)` |
| `cancel_recovery` | `rec_cncl` | `()` |
| `set_registry` | `reg_link` | `registry_id: Address` |

> **Note:** For `rec_init`, `executable_at` is derived from the ledger sequence at initiation time plus `RECOVERY_TIMELOCK`. It is not stored directly in the event data but can be computed from the on-chain ledger context.

Off-chain watchers should subscribe to `rec_init` events and alert the owner immediately.

### 4.6 Registry Link

The recovery contract supports an optional `registry_id` field that links the deployment to a specific `mux-registry` instance.

- **Purpose:** When set, off-chain tooling can cross-reference the registry metadata (e.g. contract version, audit status) against the deployed recovery contract to confirm authenticity.
- **Optional field:** `registry_id()` returns `None` if the registry link has not been set. The contract operates normally without it.
- **Owner-gated:** Setting the registry requires `owner.require_auth()`. Guardians and other callers cannot modify the link.
- **Auditability:** Calling `set_registry` emits a `reg_link` event containing the registry address, so the linkage is fully observable on-chain.

> **Upcoming:** The `set_registry()` entrypoint and `registry_id` storage are tracked in issue [#403](https://github.com/mux-labs/mux-contracts/issues/403).

---

## 5. Threat Scenarios

| Threat | Mitigation |
|---|---|
| Attacker compromises one guardian | Single guardian can initiate but owner has 24h to cancel |
| All guardians collude | Owner has 24h cancellation window; monitor `rec_init` events |
| Owner loses key, no guardians | Recovery impossible — owner must set guardians at init |
| Attacker spams recovery requests | Only one Pending request allowed; each requires guardian auth |
| Replay of old recovery request | Each request stores `initiated_at`; executed/cancelled requests cannot be re-executed |
| Owner tries to remove guardians | Guardian set is immutable after `initialize` |

---

## 6. Limitations and Future Work

- **No quorum threshold.** Currently any single guardian can initiate and execute recovery. A future version should require M-of-N guardian signatures.
- **Immutable guardian set.** Guardians cannot be rotated without redeploying the contract. A guardian rotation mechanism with its own timelock is planned.
- **No guardian liveness check.** If all guardians lose their keys, recovery is impossible.
- **No on-chain registry validation.** The `registry_id` field stores an address but does not call the registry at initialization time. Off-chain tooling must verify the link is correct and that the registry entry matches the deployed contract.
- **Single-signer guardian model.** Each guardian acts independently — there is no on-chain M-of-N threshold enforced per-signature. Any single guardian can initiate and execute recovery on their own. A multi-signature threshold requirement is a planned future improvement.

---

## 7. Related Documents

- [`docs/threat-model.md`](threat-model.md) — overall Mux Protocol threat model
- [`docs/audit-events.md`](audit-events.md) — full event schema reference
- [`contracts/mux-recovery/src/lib.rs`](../contracts/mux-recovery/src/lib.rs) — contract source
- `#403` — Recovery registry link implementation; tracks the `set_registry()` entrypoint and `registry_id` storage
