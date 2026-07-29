/**
 * Tests for MuxRecoveryClient, RecoveryStatus enum, helper functions,
 * filtering query params, and the exported recovery timelock constants.
 *
 * Covers issue #397 (recovery status enum) and issue #398 (timelock constants).
 */

import {
  MuxRecoveryClient,
  RecoveryQueryFilters,
  RecoveryStatus,
  recoveryStatusFromString,
  isTerminalRecoveryStatus,
  isCancellableRecoveryStatus,
} from "../src/generated/mux-recovery";
import {
  RECOVERY_TIMELOCK_LEDGERS,
  RECOVERY_EXPIRY_LEDGERS,
} from "../src/types";

// ── RecoveryStatus enum (closes #397) ─────────────────────────────────────────

describe("RecoveryStatus enum — variant values", () => {
  it("RecoveryStatus.None equals the string 'None'", () => {
    expect(RecoveryStatus.None).toBe("None");
  });

  it("RecoveryStatus.Pending equals the string 'Pending'", () => {
    expect(RecoveryStatus.Pending).toBe("Pending");
  });

  it("RecoveryStatus.Executed equals the string 'Executed'", () => {
    expect(RecoveryStatus.Executed).toBe("Executed");
  });

  it("RecoveryStatus.Cancelled equals the string 'Cancelled'", () => {
    expect(RecoveryStatus.Cancelled).toBe("Cancelled");
  });

  it("all four variants are distinct", () => {
    const variants = [
      RecoveryStatus.None,
      RecoveryStatus.Pending,
      RecoveryStatus.Executed,
      RecoveryStatus.Cancelled,
    ];
    const unique = new Set(variants);
    expect(unique.size).toBe(4);
  });
});

describe("RecoveryStatus enum — re-export from main package", () => {
  it("RecoveryStatus is exported from mux-recovery binding", () => {
    expect(RecoveryStatus).toBeDefined();
  });

  it("all four variant names exist as enum keys", () => {
    expect("None" in RecoveryStatus).toBe(true);
    expect("Pending" in RecoveryStatus).toBe(true);
    expect("Executed" in RecoveryStatus).toBe(true);
    expect("Cancelled" in RecoveryStatus).toBe(true);
  });
});

// ── recoveryStatusFromString helper (closes #397) ─────────────────────────────

describe("recoveryStatusFromString", () => {
  it("parses 'None' to RecoveryStatus.None", () => {
    expect(recoveryStatusFromString("None")).toBe(RecoveryStatus.None);
  });

  it("parses 'Pending' to RecoveryStatus.Pending", () => {
    expect(recoveryStatusFromString("Pending")).toBe(RecoveryStatus.Pending);
  });

  it("parses 'Executed' to RecoveryStatus.Executed", () => {
    expect(recoveryStatusFromString("Executed")).toBe(RecoveryStatus.Executed);
  });

  it("parses 'Cancelled' to RecoveryStatus.Cancelled", () => {
    expect(recoveryStatusFromString("Cancelled")).toBe(RecoveryStatus.Cancelled);
  });

  it("throws for an unrecognised string", () => {
    expect(() => recoveryStatusFromString("Invalid")).toThrow(
      'Unknown RecoveryStatus value: "Invalid"'
    );
  });

  it("throws for an empty string", () => {
    expect(() => recoveryStatusFromString("")).toThrow(
      'Unknown RecoveryStatus value: ""'
    );
  });

  it("is case-sensitive — 'pending' throws", () => {
    expect(() => recoveryStatusFromString("pending")).toThrow();
  });

  it("round-trips all variants through their string values", () => {
    const variants = [
      RecoveryStatus.None,
      RecoveryStatus.Pending,
      RecoveryStatus.Executed,
      RecoveryStatus.Cancelled,
    ];
    for (const v of variants) {
      expect(recoveryStatusFromString(v)).toBe(v);
    }
  });
});

// ── isTerminalRecoveryStatus helper (closes #397) ─────────────────────────────

describe("isTerminalRecoveryStatus", () => {
  it("returns false for None", () => {
    expect(isTerminalRecoveryStatus(RecoveryStatus.None)).toBe(false);
  });

  it("returns false for Pending", () => {
    expect(isTerminalRecoveryStatus(RecoveryStatus.Pending)).toBe(false);
  });

  it("returns true for Executed", () => {
    expect(isTerminalRecoveryStatus(RecoveryStatus.Executed)).toBe(true);
  });

  it("returns true for Cancelled", () => {
    expect(isTerminalRecoveryStatus(RecoveryStatus.Cancelled)).toBe(true);
  });
});

// ── isCancellableRecoveryStatus helper (closes #397) ──────────────────────────

describe("isCancellableRecoveryStatus", () => {
  it("returns false for None (no active request to cancel)", () => {
    expect(isCancellableRecoveryStatus(RecoveryStatus.None)).toBe(false);
  });

  it("returns true for Pending (owner may cancel)", () => {
    expect(isCancellableRecoveryStatus(RecoveryStatus.Pending)).toBe(true);
  });

  it("returns false for Executed (terminal — cannot cancel)", () => {
    expect(isCancellableRecoveryStatus(RecoveryStatus.Executed)).toBe(false);
  });

  it("returns false for Cancelled (already cancelled)", () => {
    expect(isCancellableRecoveryStatus(RecoveryStatus.Cancelled)).toBe(false);
  });
});

// ── MuxRecoveryClient shape (existing smoke tests) ────────────────────────────

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

// ── Recovery timelock constants (closes #398) ─────────────────────────────────

describe("Recovery timelock constants", () => {
  it("RECOVERY_TIMELOCK_LEDGERS equals the on-chain constant 17_280", () => {
    expect(RECOVERY_TIMELOCK_LEDGERS).toBe(17_280);
  });

  it("RECOVERY_TIMELOCK_LEDGERS represents ~24 hours at 5-second ledger close", () => {
    const seconds = RECOVERY_TIMELOCK_LEDGERS * 5;
    expect(seconds).toBe(86_400); // 24 * 60 * 60
  });

  it("RECOVERY_EXPIRY_LEDGERS equals the on-chain constant 120_960", () => {
    expect(RECOVERY_EXPIRY_LEDGERS).toBe(120_960);
  });

  it("RECOVERY_EXPIRY_LEDGERS represents ~7 days at 5-second ledger close", () => {
    const seconds = RECOVERY_EXPIRY_LEDGERS * 5;
    expect(seconds).toBe(604_800); // 7 * 24 * 60 * 60
  });

  it("RECOVERY_EXPIRY_LEDGERS is greater than RECOVERY_TIMELOCK_LEDGERS", () => {
    expect(RECOVERY_EXPIRY_LEDGERS).toBeGreaterThan(RECOVERY_TIMELOCK_LEDGERS);
  });

  it("can compute executableAt from initiatedAt without an RPC call", () => {
    const initiatedAt = 500_000;
    const executableAt = initiatedAt + RECOVERY_TIMELOCK_LEDGERS;
    const expiresAt = initiatedAt + RECOVERY_EXPIRY_LEDGERS;
    expect(executableAt).toBe(517_280);
    expect(expiresAt).toBe(620_960);
    expect(expiresAt).toBeGreaterThan(executableAt);
  });
});
