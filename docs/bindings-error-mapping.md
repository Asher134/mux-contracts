# Bindings Error Mapping

This document explains how Soroban contract error enums flow into TypeScript
union types and HTTP status codes in the `@mux-protocol/contracts` package.

## Pipeline

```
Rust #[contracterror] enum
  │
  ▼
Stellar CLI codegen (bindings/src/generated/<contract>.ts)
  │
  ▼
TS string-union type in bindings/src/types.ts
  │
  ▼
ERROR_HTTP_MAP in bindings/src/errors.ts
  │
  ▼
contractErrorToHttp() → HttpErrorResponse
```

### Step 1 — Rust Error Enum

Each contract defines a single `#[contracterror]` enum with `#[repr(u32)]`:

```rust
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MuxAccountError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    // ...
}
```

When a contract function returns `Err(MuxAccountError::Unauthorized)`, the
Soroban runtime encodes the error as an `ScError` with the enum discriminant
(the `u32` code).

### Step 2 — Codegen Output

Running `stellar contract bindings generate` produces a TypeScript client file
per contract in `bindings/src/generated/`. The generated types include a
**string literal union** for each error enum:

```ts
// auto-generated in bindings/src/generated/mux-account.ts
export type MuxAccountError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "DelegateNotFound"
  | ...;
```

The Stellar SDK automatically decodes on-chain `ScError` values back to these
string names when you read the error from a transaction result.

### Step 3 — Manual Union Type (`types.ts`)

For contracts whose generated types don't include a standalone error union (or
when you need a consolidated union across contracts), define it in
`bindings/src/types.ts`:

```ts
export type MuxAccountError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "DelegateNotFound"
  | "DelegateExpired"
  | "SpendLimitExceeded"
  | "InvalidAmount"
  | "InvalidPeriod"
  | "TooManyDelegates"
  | "ReentrancyDetected"
  | "ArithmeticOverflow"
  | "TooManySessionKeys";
```

Each variant name **must** match the Rust enum variant exactly (case-sensitive).

`types.ts` also provides optional `*ErrorMessage()` helpers that map both the
string name and the raw `u32` code to a human-readable description:

```ts
import { muxAccountErrorMessage } from "@mux-protocol/contracts";

muxAccountErrorMessage("DelegateNotFound"); // "delegate not found"
muxAccountErrorMessage(4);                  // "delegate not found"
```

### Step 4 — HTTP Status Map (`errors.ts`)

`bindings/src/errors.ts` exports `ERROR_HTTP_MAP`, a `Record<string, number>`
that maps variant names to HTTP status codes:

```ts
export const ERROR_HTTP_MAP: Record<string, number> = {
  Unauthorized: 401,
  DelegateNotFound: 404,
  InvalidAmount: 400,
  AlreadyInitialized: 409,
  NotInitialized: 500,
  // ...
};
```

The helper `contractErrorToHttp()` wraps this map:

```ts
const response = contractErrorToHttp("Unauthorized");
// { statusCode: 401, message: "Unauthorized", errorType: "Unauthorized" }
```

Unknown errors default to **500 Internal Server Error**.

## HTTP Status Code Conventions

| Status | Category | Examples |
|--------|----------|---------|
| **401** | Authentication / authorization | `Unauthorized` |
| **400** | Invalid input / constraint violation | `InvalidAmount`, `SpendLimitExceeded`, `EmptyBatch`, `BatchTooLarge` |
| **404** | Resource not found | `DelegateNotFound`, `RoleNotFound`, `ContractNotFound`, `WalletNotFound` |
| **409** | State conflict / capacity limit | `AlreadyInitialized`, `TooManyDelegates`, `ReentrancyDetected` |
| **500** | Internal / uninitialized | `NotInitialized`, `ArithmeticOverflow`, `RequiredOperationFailed` |

## Adding a New Error Variant

When you add a variant to a Rust `#[contracterror]` enum, update these files:

| File | Change |
|------|--------|
| `contracts/<crate>/src/lib.rs` | Add variant with the next `u32` code |
| `docs/error_codes.md` | Add row with variant, code, HTTP status, and description |
| `bindings/src/types.ts` | Add variant to the TS union type and update the `*ErrorMessage` maps |
| `bindings/src/errors.ts` | Add entry to `ERROR_HTTP_MAP` with the appropriate HTTP status |
| `contracts/README.md` | No change needed unless the contract summary changes |

After updating, regenerate bindings and run tests:

```bash
bash scripts/generate-bindings.sh
cd bindings && npm test
```

## Cross-Contract Error Overlap

Multiple contracts may use the same variant name (e.g. `Unauthorized` appears
in 9 of 10 contracts). The `ERROR_HTTP_MAP` is **shared** — the same variant
name always maps to the same HTTP status regardless of which contract produced
it. This is intentional: API consumers only need to handle one HTTP status per
error name.

If two contracts need different HTTP semantics for the same error name, rename
one of the variants in the Rust enum to avoid ambiguity.

## Example: End-to-End Flow

```
1. Contract returns Err(MuxAccountError::Unauthorized)   [Rust, u32 code 3]
2. Soroban runtime encodes as ScError(3)                  [on-chain]
3. Stellar SDK decodes to string "Unauthorized"           [TypeScript]
4. contractErrorToHttp("Unauthorized")                    [bindings]
   → { statusCode: 401, message: "Unauthorized", errorType: "Unauthorized" }
5. API returns HTTP 401 to the client
```
