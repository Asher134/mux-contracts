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
| `execute_with_session` | Not yet enforced; registry integration is pending |
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

- `set_delegate(delegate, expiry_ledger, can_spend) -> Result<(), MuxAccountError>`
  inserts or updates a delegate. New entries are capped at 64.
- `remove_delegate(delegate) -> Result<(), MuxAccountError>` removes an entry.
- `delegates() -> Result<Map<Address, DelegateInfo>, MuxAccountError>` returns
  only delegates whose expiry ledger is still in the future.
- `get_delegate(delegate) -> Result<DelegateInfo, MuxAccountError>` returns one
  active delegate, or `DelegateNotFound` / `DelegateExpired`.

`DelegateInfo` contains `address`, `expiry_ledger`, and `can_spend`.

### Spend limits

- `set_spend_limit(asset, amount, period_ledgers) -> Result<(), MuxAccountError>`
  sets a positive allowance and reset period for an asset.
- `debit_spend(asset, spend) -> Result<(), MuxAccountError>` atomically rolls
  the period forward when needed and increments `spent`. Missing or exceeded
  limits return `SpendLimitExceeded`.

`SpendLimit` contains `asset`, `amount`, `period_ledgers`, `spent`, and
`reset_ledger`.

### Sessions and metadata

- `execute_with_session(session_key, payload) -> Result<Bytes, MuxAccountError>`
  currently emits an execution audit event and returns empty bytes. Session
  registry validation and payload dispatch remain pending.
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
