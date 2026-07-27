/**
 * Unit tests for MuxDelegationClient binding shape and error mapping.
 * Covers delegate permissions map hardening (closes #407).
 *
 * Rust-side changes in this issue:
 *   - grant_delegate now calls extend_entry_ttl on DelegatePerms and
 *     OwnerDelegates persistent entries so each record stays live
 *     independently of the contract instance TTL.
 *   - revoke_delegate refreshes OwnerDelegates TTL after mutation.
 *
 * getDelegatePermissions and isDelegate are read-only: they return an
 * empty list / false for unknown (owner, delegate) pairs and do not
 * surface errors at the TypeScript boundary.
 */

import { MuxDelegationClient } from "../src/generated/mux-delegation";
import { ERROR_HTTP_MAP } from "../src/errors";
import { muxDelegationErrorMessage } from "../src/types";

// ── Client shape — permissions-map methods ────────────────────────────────────

describe("MuxDelegationClient shape — permissions map (closes #407)", () => {
  it("exposes getDelegatePermissions as a function", () => {
    expect(typeof MuxDelegationClient.prototype.getDelegatePermissions).toBe("function");
  });

  it("getDelegatePermissions accepts sourceKeypair, owner, delegate (arity 3)", () => {
    expect(MuxDelegationClient.prototype.getDelegatePermissions.length).toBe(3);
  });

  it("exposes isDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.isDelegate).toBe("function");
  });

  it("isDelegate accepts sourceKeypair, owner, delegate, permission (arity 4)", () => {
    expect(MuxDelegationClient.prototype.isDelegate.length).toBe(4);
  });

  it("exposes getDelegates as a function", () => {
    expect(typeof MuxDelegationClient.prototype.getDelegates).toBe("function");
  });

  it("exposes grantDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.grantDelegate).toBe("function");
  });

  it("exposes revokeDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.revokeDelegate).toBe("function");
  });
});

// ── HTTP error mapping ────────────────────────────────────────────────────────

describe("Delegation error HTTP mapping", () => {
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

describe("muxDelegationErrorMessage helper", () => {
  it("resolves NotADelegate by name (6001)", () => {
    expect(muxDelegationErrorMessage("NotADelegate")).toBe(
      "no delegate grant found for this pair"
    );
  });

  it("resolves NotADelegate by code 6001", () => {
    expect(muxDelegationErrorMessage(6001)).toBe(
      "no delegate grant found for this pair"
    );
  });

  it("resolves TooManyPermissions by name (6002)", () => {
    expect(muxDelegationErrorMessage("TooManyPermissions")).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("resolves TooManyPermissions by code 6002", () => {
    expect(muxDelegationErrorMessage(6002)).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("resolves EmptyPermissions by name (6003)", () => {
    expect(muxDelegationErrorMessage("EmptyPermissions")).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  it("resolves EmptyPermissions by code 6003", () => {
    expect(muxDelegationErrorMessage(6003)).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  it("resolves TooManyDelegates by name (6004)", () => {
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

  it("returns 'unknown error code' for code 0", () => {
    expect(muxDelegationErrorMessage(0)).toBe("unknown error code");
  });
});
