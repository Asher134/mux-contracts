/**
 * Unit tests for MuxDelegationClient binding shape and error mapping.
 * Covers grant_delegate hardening (closes #405).
 */

import { MuxDelegationClient } from "../src/generated/mux-delegation";
import { ERROR_HTTP_MAP } from "../src/errors";
import { muxDelegationErrorMessage } from "../src/types";

describe("MuxDelegationClient shape", () => {
  it("exposes grantDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.grantDelegate).toBe("function");
  });

  it("exposes revokeDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.revokeDelegate).toBe("function");
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

  // Closes #405 — grant_delegate hardening: TooManyDelegates storage-griefing guard.
  it("maps TooManyDelegates to 409", () => {
    expect(ERROR_HTTP_MAP.TooManyDelegates).toBe(409);
  });
});

describe("muxDelegationErrorMessage helper", () => {
  it("returns message for NotADelegate by name", () => {
    expect(muxDelegationErrorMessage("NotADelegate")).toBe(
      "no delegate grant found for this pair"
    );
  });

  it("returns message for NotADelegate by code 6001", () => {
    expect(muxDelegationErrorMessage(6001)).toBe(
      "no delegate grant found for this pair"
    );
  });

  it("returns message for TooManyPermissions by name", () => {
    expect(muxDelegationErrorMessage("TooManyPermissions")).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("returns message for TooManyPermissions by code 6002", () => {
    expect(muxDelegationErrorMessage(6002)).toBe(
      "permission list exceeds the 64-entry cap"
    );
  });

  it("returns message for EmptyPermissions by name", () => {
    expect(muxDelegationErrorMessage("EmptyPermissions")).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  it("returns message for EmptyPermissions by code 6003", () => {
    expect(muxDelegationErrorMessage(6003)).toBe(
      "permission list is empty; at least one permission is required"
    );
  });

  // Closes #405 — grant_delegate hardening: TooManyDelegates error message.
  it("returns message for TooManyDelegates by name", () => {
    expect(muxDelegationErrorMessage("TooManyDelegates")).toBe(
      "owner already has 128 delegates registered"
    );
  });

  it("returns message for TooManyDelegates by code 6004", () => {
    expect(muxDelegationErrorMessage(6004)).toBe(
      "owner already has 128 delegates registered"
    );
  });

  it("returns unknown for unrecognised code", () => {
    expect(muxDelegationErrorMessage(9999)).toBe("unknown error code");
  });
});
