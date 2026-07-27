import type { MuxAccountFactoryError } from "./generated/mux-account-factory";
import type { MuxRegistryError } from "./generated/mux-registry";
import type { MuxWalletRegistryError } from "./generated/mux-wallet-registry";
import type {
  MuxAccountError,
  MuxBatcherError,
  MuxDelegationError,
  MuxPermissionsError,
  MuxPolicyError,
  MuxRecoveryError,
} from "./types";

export interface HttpErrorResponse {
  statusCode: number;
  message: string;
  errorType: string;
}

type ContractError =
  | MuxAccountError
  | MuxBatcherError
  | MuxDelegationError
  | MuxPermissionsError
  | MuxPolicyError
  | MuxAccountFactoryError
  | MuxRegistryError
  | MuxWalletRegistryError
  | MuxRecoveryError;

/**
 * Maps contract error variants to HTTP status codes.
 * - 401: Unauthorized (authentication/permission issues)
 * - 404: Not Found (missing resources)
 * - 400: Bad Request (invalid input, constraint violations)
 * - 409: Conflict (state conflicts)
 * - 500: Internal Server Error (initialization or unknown errors)
 *
 * MuxAccount error codes (contracts/mux-account):
 *   NotInitialized      (1)  → 500
 *   AlreadyInitialized  (2)  → 409
 *   Unauthorized        (3)  → 401
 *   DelegateNotFound    (4)  → 404
 *   DelegateExpired     (5)  → 400
 *   SpendLimitExceeded  (6)  → 400
 *   InvalidAmount       (7)  → 400
 *   InvalidPeriod       (8)  → 400
 *   TooManyDelegates    (9)  → 409
 *   ReentrancyDetected  (10) → 409
 *   ArithmeticOverflow  (11) → 500
 *
 * MuxAccountFactory error codes (contracts/mux-account-factory):
 *   Unauthorized      (1) → 401  caller is not the registered owner
 *   InvalidAccount    (2) → 400  account_address must differ from owner
 *   TooManyAccounts   (3) → 409  per-owner 64-account cap reached
 *   MetadataNotFound  (4) → 404  no metadata stored for the account
 *
 * MuxDelegation error codes (contracts/mux-delegation):
 *   NotADelegate       (6001) → 404  no grant exists for the (owner, delegate) pair
 *   TooManyPermissions (6002) → 400  permission list exceeds the 64-entry cap
 *   EmptyPermissions   (6003) → 400  at least one permission must be specified
 *   TooManyDelegates   (6004) → 409  owner already has 128 delegates registered
 *
 * getDelegatePermissions and isDelegate are read-only and do not return errors;
 * they return an empty list / false for unknown (owner, delegate) pairs.
 */
export const ERROR_HTTP_MAP: Record<string, number> = {
  // Authentication/Authorization errors → 401
  Unauthorized: 401,

  // Not Found errors → 404
  NotADelegate: 404,       // MuxDelegationError (6001): no grant for (owner, delegate)
  DelegateNotFound: 404,
  RoleNotFound: 404,
  AccountNotInRole: 404,
  PermissionNotFound: 404,
  ContractNotFound: 404,
  WalletNotFound: 404,
  MetadataNotFound: 404,   // MuxAccountFactoryError (4)
  AdminNotFound: 404,
  LimitNotFound: 404,

  // Validation/Constraint errors → 400
  InvalidAmount: 400,
  InvalidPeriod: 400,
  SpendLimitExceeded: 400,
  LimitExceeded: 400,
  DelegateExpired: 400,
  EmptyBatch: 400,
  BatchTooLarge: 400,
  InvalidAccount: 400,     // MuxAccountFactoryError (2)
  MetadataTooLarge: 400,

  // Delegation constraint errors → 400
  // MuxDelegationError::TooManyPermissions (6002) and ::EmptyPermissions (6003)
  TooManyPermissions: 400,
  EmptyPermissions: 400,

  // State conflict → 409
  AlreadyInitialized: 409,
  AlreadyApproved: 409,    // MuxPermissionsError (10): approver already approved this candidate

  // Security guard violations → 409 Conflict (concurrent/reentrant call)
  ReentrancyDetected: 409,

  // Capacity limits → 409 Conflict
  // MuxAccountError::TooManyDelegates (9) and MuxDelegationError::TooManyDelegates (6004)
  TooManyDelegates: 409,
  TooManyAccounts: 409,    // MuxAccountFactoryError (3)
  TooManyContracts: 409,
  TooManyMembers: 409,
  TooManyRoles: 409,
  TooManyWallets: 409,
  TooManySessionKeys: 409,

  // Internal/Uninitialized → 500
  NotInitialized: 500,
  RequiredOperationFailed: 500,
  ArithmeticOverflow: 500,
};

/**
 * Converts a contract error to an HTTP error response.
 * Unknown errors default to 500 Internal Server Error.
 */
export function contractErrorToHttp(error: ContractError | string): HttpErrorResponse {
  const errorType = String(error);
  const statusCode = ERROR_HTTP_MAP[errorType] || 500;

  return {
    statusCode,
    message: errorType,
    errorType,
  };
}

/**
 * Checks if an error from a contract call should be treated as an HTTP error.
 * Can be used in middleware/error handlers.
 */
export function isContractError(error: unknown): error is string {
  if (typeof error !== "string") {
    return false;
  }
  return error in ERROR_HTTP_MAP || true; // Conservative: treat any string as potential error
}
