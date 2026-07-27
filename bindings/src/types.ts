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
  | "ReentrancyDetected";

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
 * `contracts/mux-delegation/src/lib.rs` (error codes 6001–6004).
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
  error: MuxPermissionsError | number
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

  const code =
    typeof error === "number" ? error : nameMap[error] ?? -1;
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
  error: MuxAccountError | number
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

  const code =
    typeof error === "number" ? error : nameMap[error] ?? -1;
  return codeMap[code] ?? "unknown error code";
}

export interface SpendingPolicyLimit {
  asset: Address;
  limit: bigint;
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
  | "SpendLimitExceeded";
