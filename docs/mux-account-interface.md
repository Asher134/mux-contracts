# Mux Account Public Interface

`mux-account` is a Soroban smart account that stores one owner, a bounded
delegate map, guardians, and per-asset spend limits. All write entrypoints
extend instance-storage TTL.

## Authorization

| Entrypoint | Required authorization |
|---|---|
| `initialize` | supplied `owner` |
| `unpause` | stored owner |
| `set_delegate` | stored owner |
| `remove_delegate` | stored owner |
| `set_spend_limit` | stored owner |
| `debit_spend` | current contract address |
| `set_metadata` | stored owner |
| `register_session_key` | stored owner |
| `revoke_session_key` | stored owner |
| `execute_with_session` | authorized session key (`session_key.require_auth()`); payload dispatch itself is not yet implemented |
| Read-only entrypoints | none |

Owner-only calls fail with a host authorization error when the signature is
missing. Contract validation failures use `MuxAccountError`.

## Entrypoints

### Initialization and status

- `initialize(owner, guardians) -> Result<(), MuxAccountError>` initializes
  the instance once.
- `owner() -> Result<Address, MuxAccountError>` returns the stored owner.
- `guardians() -> Result<Vec<Address>, MuxAccountError>` returns guardians.
- `is_paused() -> bool` returns the pause flag.
- `unpause() -> Result<(), MuxAccountError>` clears the pause flag.

### Delegates

- `set_delegate(delegate, expires_at, can_spend) -> Result<(), MuxAccountError>`
  inserts or updates a delegate. New entries are capped at 64. `expires_at` is
  a Unix timestamp (`env.ledger().timestamp()`), not a ledger sequence.
- `remove_delegate(delegate) -> Result<(), MuxAccountError>` removes an entry.
- `delegates() -> Result<Map<Address, DelegateInfo>, MuxAccountError>` returns
  only delegates whose `expires_at` timestamp is still in the future.
- `get_delegate(delegate) -> Result<DelegateInfo, MuxAccountError>` returns one
  active delegate, or `DelegateNotFound` / `DelegateExpired`.

`DelegateInfo` contains `address`, `expires_at` (Unix timestamp, `u64`), and
`can_spend`.

### Spend limits

- `set_spend_limit(asset, amount, period_ledgers) -> Result<(), MuxAccountError>`
  sets a positive allowance and reset period for an asset.
- `debit_spend(asset, spend) -> Result<(), MuxAccountError>` atomically rolls
  the period forward when needed and increments `spent`. Missing or exceeded
  limits return `SpendLimitExceeded`.

`SpendLimit` contains `asset`, `amount`, `period_ledgers`, `spent`, and
`reset_ledger`.

### Sessions and metadata

- `register_session_key(session_key, expires_at, scopes) -> Result<(), MuxAccountError>`
  registers or replaces a session key with a Unix-timestamp expiry and a set
  of `Scope` capabilities. New keys are capped at `MAX_SESSION_KEYS` (32) per
  owner.
- `revoke_session_key(session_key) -> Result<(), MuxAccountError>` marks a
  registered session key as revoked.
- `execute_with_session(session_key, payload) -> Result<Bytes, MuxAccountError>`
  validates that `session_key` is authorized, registered, non-revoked, and
  non-expired, then emits an execution audit event and returns empty bytes.
  It does not decode or dispatch `payload` — the `scopes` on the session
  record are stored but not enforced here. See
  [aa_sequence_diagram.md](aa_sequence_diagram.md) for the gap between this
  and the intended account-abstraction execution flow.
- `set_metadata(meta) -> Result<(), MuxAccountError>` stores owner-controlled
  `RegistryMeta`.
- `get_metadata() -> Option<RegistryMeta>` returns metadata when present.

## Errors

| Code | Variant | Meaning |
|---:|---|---|
| 1 | `NotInitialized` | Required account state is absent |
| 2 | `AlreadyInitialized` | Initialization was already completed |
| 3 | `Unauthorized` | Contract state disallows the call |
| 4 | `DelegateNotFound` | Delegate is absent |
| 5 | `DelegateExpired` | Delegate is no longer active |
| 6 | `SpendLimitExceeded` | Limit is absent or would be exceeded |
| 7 | `InvalidAmount` | Amount is not positive |
| 8 | `InvalidPeriod` | Reset period is zero |
| 9 | `TooManyDelegates` | Delegate cap is reached |
| 10 | `ReentrancyDetected` | Spend accounting is already executing |
| 11 | `ArithmeticOverflow` | Spend addition overflowed |
| 12 | `TooManySessionKeys` | Session-key cap is reached |

## Events

Events use topics `(mux_acct, action)`. The actions are `init`, `unpaused`,
`dlg_set`, `dlg_rm`, `lmt_set`, `debited`, `ses_exe`, and `meta_set`.
See [audit events](audit-events.md) for payload shapes.

## Binding notes

The Rust signatures above define the generated client ABI. After changing a
public type or entrypoint, run `bash scripts/generate-bindings.sh` and update
downstream TypeScript calls in the same release.
