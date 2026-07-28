import { Address, xdr } from "@stellar/stellar-sdk";

export type NetworkPassphrase = string;

export interface MuxContractIds {
  muxAccount: string;
  muxBatcher: string;
  muxDelegation: string;
  muxPermissions: string;
  muxWalletRegistry: string;
  muxAccountFactory?: string;
  muxRegistry?: string;
  muxPolicy?: string;
}

export interface SpendLimit {
  asset: Address;
  amount: bigint;
  periodLedgers: number;
  spent: bigint;
  resetLedger: number;
}

export interface DelegateInfo {
  address: Address;
  expiryLedger: number;
  canSpend: boolean;
}

export interface Operation {
  target: Address;
  fnName: string;
  args: xdr.ScVal[];
  requireSuccess: boolean;
  /** Classifies the operation intent for indexers and UI. */
  kind: BatchOperationKind;
}

/** Mirrors the on-chain `BatchOperationKind` enum. */
export type BatchOperationKind = "Invoke" | "Transfer" | "Approve";

export interface BatchResult {
  successCount: number;
  failureCount: number;
}

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

export type MuxRecoveryError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "RecoveryAlreadyPending"
  | "NoActiveRecovery"
  | "TimelockNotExpired";

export type MuxBatcherError =
  | "EmptyBatch"
  | "BatchTooLarge"
  | "RequiredOperationFailed"
  | "Unauthorized"
  | "ReentrancyDetected"
  | "MetadataAlreadySet";

/**
 * Contract-level metadata stored once at deployment for registry discovery.
 * Mirrors the on-chain `BatcherMeta` struct.
 */
export interface BatcherMeta {
  /** Short human-readable description of the contract. */
  description: string;
  /** Author or team identifier. */
  author: string;
}

/**
 * Maps a `MuxBatcherError` variant or its raw `u32` contract error code to
 * a human-readable description.
 *
 * Mirrors the on-chain `MuxBatcherError` enum in `contracts/mux-batcher`.
 *
 * @example
 * ```ts
 * import { muxBatcherErrorMessage } from "./types";
 * console.log(muxBatcherErrorMessage("BatchTooLarge")); // "batch exceeds the maximum operation count"
 * console.log(muxBatcherErrorMessage(2));               // "batch exceeds the maximum operation count"
 * ```
 */
export function muxBatcherErrorMessage(error: MuxBatcherError | number): string {
  const codeMap: Record<number, string> = {
    1: "batch contains no operations",
    2: "batch exceeds the maximum operation count",
    3: "a required operation failed; the batch was aborted",
    4: "caller is not authorized",
    5: "reentrant call into the batcher detected",
    6: "metadata has already been set for this batcher instance",
  };

  const nameMap: Record<MuxBatcherError, number> = {
    EmptyBatch: 1,
    BatchTooLarge: 2,
    RequiredOperationFailed: 3,
    Unauthorized: 4,
    ReentrancyDetected: 5,
    MetadataAlreadySet: 6,
  };

  const code = typeof error === "number" ? error : nameMap[error] ?? -1;
  return codeMap[code] ?? "unknown error code";
}

export type MuxDelegationError =
  | "NotADelegate"
  | "TooManyPermissions"
  | "EmptyPermissions"
  | "TooManyDelegates";

/**
 * Maps a `MuxDelegationError` variant or its raw `u32` contract error code to
 * a human-readable description.
 *
 * Mirrors the on-chain `MuxDelegationError` enum in
 * `contracts/mux-delegation/src/lib.rs` (stable ABI codes 6001–6004).
 *
 * Relevant to the delegate permissions map (closes #407):
 *   - `getDelegatePermissions` and `isDelegate` are read-only and do not
 *     return errors; they return an empty list / false for unknown pairs.
 *
 * @example
 * ```ts
 * import { muxDelegationErrorMessage } from "./types";
 * console.log(muxDelegationErrorMessage("NotADelegate")); // "no delegate grant found for this pair"
 * console.log(muxDelegationErrorMessage(6001));           // "no delegate grant found for this pair"
 * ```
 */
export function muxDelegationErrorMessage(
  error: MuxDelegationError | number
): string {
  const codeMap: Record<number, string> = {
    6001: "no delegate grant found for this pair",
    6002: "permission list exceeds the 64-entry cap",
    6003: "permission list is empty; at least one permission is required",
    6004: "owner already has 128 delegates registered",
  };

  const nameMap: Record<MuxDelegationError, number> = {
    NotADelegate: 6001,
    TooManyPermissions: 6002,
    EmptyPermissions: 6003,
    TooManyDelegates: 6004,
  };

  const code =
    typeof error === "number" ? error : nameMap[error] ?? -1;
  return codeMap[code] ?? "unknown error code";
}

export type MuxPermissionsError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "RoleNotFound"
  | "AccountNotInRole"
  | "PermissionNotFound"
  | "TooManyMembers"
  | "TooManyRoles"
  | "AdminNotFound"
  | "AlreadyApproved";

/**
 * Maps a `MuxPermissionsError` variant or its raw `u32` contract error code to
 * a human-readable description.
 *
 * Mirrors the on-chain `error_message` function so that clients can resolve
 * error codes without an extra RPC call.
 *
 * @example
 * ```ts
 * import { muxPermissionsErrorMessage } from "./types";
 * console.log(muxPermissionsErrorMessage("RoleNotFound")); // "role not found"
 * console.log(muxPermissionsErrorMessage(4));              // "role not found"
 * ```
 */
export function muxPermissionsErrorMessage(
  error: MuxPermissionsError | number,
): string {
  const codeMap: Record<number, string> = {
    1: "contract not initialized",
    2: "contract already initialized",
    3: "caller is not authorized",
    4: "role not found",
    5: "account is not a member of the role",
    6: "permission not found",
    7: "role has too many members",
    8: "account holds too many roles",
    9: "pending admin not found",
    10: "approver has already approved this candidate",
  };

  const nameMap: Record<MuxPermissionsError, number> = {
    NotInitialized: 1,
    AlreadyInitialized: 2,
    Unauthorized: 3,
    RoleNotFound: 4,
    AccountNotInRole: 5,
    PermissionNotFound: 6,
    TooManyMembers: 7,
    TooManyRoles: 8,
    AdminNotFound: 9,
    AlreadyApproved: 10,
  };

  const code = typeof error === "number" ? error : (nameMap[error] ?? -1);
  return codeMap[code] ?? "unknown error code";
}

/**
 * Maps a `MuxAccountError` variant or its raw `u32` contract error code to
 * a human-readable description.
 *
 * Mirrors the on-chain `MuxAccountError` enum in `contracts/mux-account`.
 *
 * @example
 * ```ts
 * import { muxAccountErrorMessage } from "./types";
 * console.log(muxAccountErrorMessage("DelegateNotFound")); // "delegate not found"
 * console.log(muxAccountErrorMessage(4));                  // "delegate not found"
 * ```
 */
export function muxAccountErrorMessage(
  error: MuxAccountError | number,
): string {
  const codeMap: Record<number, string> = {
    1: "contract not initialized",
    2: "contract already initialized",
    3: "caller is not authorized",
    4: "delegate not found",
    5: "delegate has expired",
    6: "spend limit exceeded",
    7: "invalid amount",
    8: "invalid period",
    9: "too many delegates",
    10: "reentrancy detected",
    11: "arithmetic overflow",
    12: "too many session keys",
  };

  const nameMap: Record<MuxAccountError, number> = {
    NotInitialized: 1,
    AlreadyInitialized: 2,
    Unauthorized: 3,
    DelegateNotFound: 4,
    DelegateExpired: 5,
    SpendLimitExceeded: 6,
    InvalidAmount: 7,
    InvalidPeriod: 8,
    TooManyDelegates: 9,
    ReentrancyDetected: 10,
    ArithmeticOverflow: 11,
    TooManySessionKeys: 12,
  };

  const code = typeof error === "number" ? error : (nameMap[error] ?? -1);
  return codeMap[code] ?? "unknown error code";
}

export interface SpendingPolicyLimit {
  asset: Address;
  limit: bigint;
}

/**
 * Maps a `SpendingPolicyError` variant or its raw `u32` contract error code to
 * a human-readable description.
 *
 * Mirrors the on-chain `SpendingPolicyError` enum in `contracts/mux-spending-policy`.
 *
 * @example
 * ```ts
 * import { spendingPolicyErrorMessage } from "./types";
 * console.log(spendingPolicyErrorMessage("PolicyNotFound")); // "policy not found"
 * console.log(spendingPolicyErrorMessage(4));                // "policy not found"
 * ```
 */
export function spendingPolicyErrorMessage(
  error: SpendingPolicyError | number,
): string {
  const codeMap: Record<number, string> = {
    1: "contract not initialized",
    2: "contract already initialized",
    3: "caller is not authorized",
    4: "policy not found",
    5: "spend limit exceeded",
    6: "invalid input",
  };

  const nameMap: Record<SpendingPolicyError, number> = {
    NotInitialized: 1,
    AlreadyInitialized: 2,
    Unauthorized: 3,
    PolicyNotFound: 4,
    SpendLimitExceeded: 5,
    InvalidInput: 6,
  };

  const code = typeof error === "number" ? error : (nameMap[error] ?? -1);
  return codeMap[code] ?? "unknown error code";
}

export type MuxPolicyError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "LimitNotFound"
  | "LimitExceeded"
  | "InvalidAmount"
  | "InvalidPeriod";

export type SpendingPolicyError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "PolicyNotFound"
  | "SpendLimitExceeded"
  | "InvalidInput";

export type MuxWalletRegistryError =
  | "NotInitialized"
  | "AlreadyInitialized"
  | "Unauthorized"
  | "WalletNotFound"
  | "TooManyWallets";

/**
 * Error variants for the `mux-account-factory` contract.
 *
 * Mirrors the on-chain `MuxAccountFactoryError` enum
 * (`contracts/mux-account-factory/src/lib.rs`).
 *
 * | Variant          | Code | HTTP | Notes                                       |
 * |------------------|------|------|---------------------------------------------|
 * | Unauthorized     |  1   | 401  | Caller is not the registered owner           |
 * | InvalidAccount   |  2   | 400  | account_address must differ from owner       |
 * | TooManyAccounts  |  3   | 409  | Per-owner 64-account cap reached             |
 * | MetadataNotFound |  4   | 404  | No metadata stored for the account           |
 * | MetadataTooLarge |  5   | 400  | Metadata field exceeds size limit            |
 */
export type MuxAccountFactoryError =
  | "Unauthorized"
  | "InvalidAccount"
  | "TooManyAccounts"
  | "MetadataNotFound"
  | "MetadataTooLarge";

/**
 * Maps a `MuxAccountFactoryError` variant or its raw `u32` contract error code
 * to a human-readable description.
 *
 * Mirrors the on-chain `MuxAccountFactoryError` enum in
 * `contracts/mux-account-factory/src/lib.rs`.
 *
 * @example
 * ```ts
 * import { muxAccountFactoryErrorMessage } from "./types";
 * console.log(muxAccountFactoryErrorMessage("TooManyAccounts")); // "owner has reached the 64-account cap"
 * console.log(muxAccountFactoryErrorMessage(3));                 // "owner has reached the 64-account cap"
 * ```
 */
export function muxAccountFactoryErrorMessage(
  error: MuxAccountFactoryError | number,
): string {
  const codeMap: Record<number, string> = {
    1: "caller is not the registered owner",
    2: "account_address must differ from owner",
    3: "owner has reached the 64-account cap",
    4: "no metadata stored for the specified account",
    5: "metadata field exceeds the allowed size limit",
  };

  const nameMap: Record<MuxAccountFactoryError, number> = {
    Unauthorized: 1,
    InvalidAccount: 2,
    TooManyAccounts: 3,
    MetadataNotFound: 4,
    MetadataTooLarge: 5,
  };

  const code = typeof error === "number" ? error : (nameMap[error] ?? -1);
  return codeMap[code] ?? "unknown error code";
}
