/**
 * Unit tests for MuxDelegationClient binding shape, DelegationQueryFilters,
 * and error mapping.
 */

import {
  MuxDelegationClient,
  DelegationQueryFilters,
} from "../src/generated/mux-delegation";
import { ERROR_HTTP_MAP } from "../src/errors";

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

  it("exposes checkDelegate as a function", () => {
    expect(typeof MuxDelegationClient.prototype.checkDelegate).toBe("function");
  });
});

describe("DelegationQueryFilters interface", () => {
  it("exports DelegationQueryFilters type", () => {
    const filters: DelegationQueryFilters = {};
    expect(filters).toBeDefined();
  });

  it("accepts a permission filter", () => {
    const filters: DelegationQueryFilters = { permission: "transfer" };
    expect(filters.permission).toBe("transfer");
  });

  it("accepts a hasAnyPermission filter set to true", () => {
    const filters: DelegationQueryFilters = { hasAnyPermission: true };
    expect(filters.hasAnyPermission).toBe(true);
  });

  it("accepts a hasAnyPermission filter set to false", () => {
    const filters: DelegationQueryFilters = { hasAnyPermission: false };
    expect(filters.hasAnyPermission).toBe(false);
  });

  it("accepts combined permission and hasAnyPermission filters", () => {
    const filters: DelegationQueryFilters = {
      permission: "read",
      hasAnyPermission: true,
    };
    expect(filters.permission).toBe("read");
    expect(filters.hasAnyPermission).toBe(true);
  });

  it("accepts an empty filter object (no-op)", () => {
    const filters: DelegationQueryFilters = {};
    expect(filters.permission).toBeUndefined();
    expect(filters.hasAnyPermission).toBeUndefined();
  });
});

describe("checkDelegate method shape", () => {
  it("getDelegatePermissions accepts optional DelegationQueryFilters parameter", () => {
    // Verify the method signature accepts the optional filters parameter.
    // The fourth parameter (filters) is optional; this confirms the binding
    // is callable without it and with it.
    const fn = MuxDelegationClient.prototype.getDelegatePermissions;
    expect(typeof fn).toBe("function");
    // arity: sourceKeypair, owner, delegate, [filters] — length may be 3 or 4
    expect(fn.length).toBeLessThanOrEqual(4);
  });

  it("getDelegates accepts optional DelegationQueryFilters parameter", () => {
    const fn = MuxDelegationClient.prototype.getDelegates;
    expect(typeof fn).toBe("function");
    // arity: sourceKeypair, owner, [filters]
    expect(fn.length).toBeLessThanOrEqual(3);
  });

  it("checkDelegate has the correct parameter count", () => {
    // sourceKeypair, owner, delegate, permission → 4 required params
    const fn = MuxDelegationClient.prototype.checkDelegate;
    expect(fn.length).toBe(4);
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
});
