/**
 * Unit tests for MuxDelegationClient binding shape and error mapping.
 * Covers revoke_delegate hardening (closes #406).
 *
 * revoke_delegate error ABI:
 *   - MuxDelegationError::NotADelegate (6001) → 404
 *     Returned when revoke is called but no grant exists for the (owner, delegate) pair.
 *
 * All error codes 6001–6004 are stable ABI — coordinate changes with a
 * registry version bump (see contracts/mux-delegation/src/lib.rs).
 */

import { MuxDelegationClient } from "../src/generated/mux-delegation";
import { ERROR_HTTP_MAP } from "../src/errors";
import { muxDelegationErrorMessage } from "../src/types";

// ── Client shape ──────────────────────────────────────────────────────────────

describe("MuxDelegationClient shape", () => {
  it("exposes grantDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.grantDelegate).toBe("function");
  });

  it("exposes revokeDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.revokeDelegate).toBe("function");
  });

  it("revokeDelegate accepts owner and delegate arguments", () => {
    // Verify arity: (sourceKeypair, owner, delegate) → 3 declared params.
    expect(MuxDelegationClient.prototype.revokeDelegate.length).toBe(3);
  });

  it("exposes getDelegatePermissions as a function", () => {
    expect(typeof MuxDelegationClient.prototype.getDelegatePermissions).toBe("function");
  });

  it("exposes isDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.isDelegate).toBe("function");
  });

  it("exposes getDelegates as a function", () => {
    expect(typeof MuxDelegationClient.prototype.getDelegates).toBe("function");
  });
});

// ── HTTP error mapping ────────────────────────────────────────────────────────

describe("Delegation error HTTP mapping", () => {
  // Revoke-path error: no grant found for the (owner, delegate) pair.
  it("maps NotADelegate to 404", () => {
    expect(ERROR_HTTP_MAP.NotADelegate).toBe(404);
  });

  it("maps TooManyPermissions to 400", () => {
    expect(ERROR_HTTP_MAP.TooManyPermissions).toBe(400);
  });

  it("maps EmptyPermissions to 400", () => {
    expect(ERROR_HTTP_MAP.EmptyPermissions).toBe(400);
  });

  it("maps TooManyDelegates to 409", () => {
    expect(ERROR_HTTP_MAP.TooManyDelegates).toBe(409);
  });
});

// ── Error message helper ──────────────────────────────────────────────────────

describe("muxDelegationErrorMessage — revoke_delegate path (closes #406)", () => {
  // NotADelegate is the only error revoke_delegate can return.
  it("returns message for NotADelegate by name", () => {
    expect(muxDelegationErrorMessage("NotADelegate")).toBe(
      "no delegate grant found for this pair"
    );
  });

  it("returns message for NotADelegate by stable ABI code 6001", () => {
    expect(muxDelegationErrorMessage(6001)).toBe(
      "no delegate grant found for this pair"
    );
  });
});

describe("muxDelegationErrorMessage — full error code table", () => {
  it("resolves TooManyPermissions by name", () => {
    expect(muxDelegationErrorMessage("TooManyPermissions")).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("resolves TooManyPermissions by code 6002", () => {
    expect(muxDelegationErrorMessage(6002)).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("resolves EmptyPermissions by name", () => {
    expect(muxDelegationErrorMessage("EmptyPermissions")).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  it("resolves EmptyPermissions by code 6003", () => {
    expect(muxDelegationErrorMessage(6003)).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  it("resolves TooManyDelegates by name", () => {
    expect(muxDelegationErrorMessage("TooManyDelegates")).toBe(
      "owner already has 128 delegates registered"
    );
  });

  it("resolves TooManyDelegates by code 6004", () => {
    expect(muxDelegationErrorMessage(6004)).toBe(
      "owner already has 128 delegates registered"
    );
  });

  it("returns 'unknown error code' for unrecognised code", () => {
    expect(muxDelegationErrorMessage(9999)).toBe("unknown error code");
  });

  it("returns 'unknown error code' for out-of-range code 0", () => {
    expect(muxDelegationErrorMessage(0)).toBe("unknown error code");
  });
});
