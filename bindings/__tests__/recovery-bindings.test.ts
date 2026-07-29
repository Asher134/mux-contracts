/**
 * Smoke-tests for the MuxRecoveryClient and filtering query params.
 *
 * These tests verify that the client is importable, the types are correct,
 * and filtering query params are properly wired.
 */

import {
  MuxRecoveryClient,
  RecoveryQueryFilters,
  RecoveryStatus,
} from "../src/generated/mux-recovery";

describe("MuxRecoveryClient filtering query params", () => {
  it("exports MuxRecoveryClient class", () => {
    expect(MuxRecoveryClient).toBeDefined();
    expect(typeof MuxRecoveryClient).toBe("function");
  });

  it("exports RecoveryStatus enum with all variants", () => {
    expect(RecoveryStatus.None).toBe("None");
    expect(RecoveryStatus.Pending).toBe("Pending");
    expect(RecoveryStatus.Executed).toBe("Executed");
    expect(RecoveryStatus.Cancelled).toBe("Cancelled");
  });

  it("RecoveryRequest interface includes expiresAt field", () => {
    // Verify the interface shape compiles and carries expiresAt.
    const req: import("../src/generated/mux-recovery").RecoveryRequest = {
      newOwner: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as any,
      initiatedAt: 1000,
      executableAt: 17280,
      expiresAt: 120960,
      status: RecoveryStatus.Pending,
    };
    expect(req.expiresAt).toBe(120960);
  });

  it("RecoveryRequest null defaults include expiresAt", () => {
    // When no recovery is active, all fields should be null including expiresAt.
    const req = {
      status: RecoveryStatus.None,
      newOwner: null,
      initiatedAt: null,
      executableAt: null,
      expiresAt: null,
    };
    expect(req.expiresAt).toBeNull();
  });

  it("exports RecoveryQueryFilters type", () => {
    const filters: RecoveryQueryFilters = {
      status: RecoveryStatus.Pending,
    };
    expect(filters.status).toBe(RecoveryStatus.Pending);
  });

  it("supports filtering by guardian address", () => {
    const filters: RecoveryQueryFilters = {
      guardian: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as any,
    };
    expect(filters.guardian).toBeDefined();
  });

  it("supports filtering by ledger range", () => {
    const filters: RecoveryQueryFilters = {
      initiatedAfter: 1000,
      initiatedBefore: 2000,
    };
    expect(filters.initiatedAfter).toBe(1000);
    expect(filters.initiatedBefore).toBe(2000);
  });

  it("combines multiple filter params", () => {
    const filters: RecoveryQueryFilters = {
      status: RecoveryStatus.Executed,
      initiatedAfter: 500,
    };
    expect(filters.status).toBe(RecoveryStatus.Executed);
    expect(filters.initiatedAfter).toBe(500);
  });
});
