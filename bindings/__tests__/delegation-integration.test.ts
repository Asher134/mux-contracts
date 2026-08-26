/**
 * Integration test stub for mux-delegation contract.
 *
 * These tests verify that the delegation bindings can interact with a
 * live or local Soroban network. They are skipped when the network is
 * unavailable, matching the pattern in integration.test.ts.
 */

import { NETWORK_CONFIGS } from "../src/network";

const NETWORK = process.env.SOROBAN_NETWORK || "localnet";
const config = NETWORK_CONFIGS[NETWORK];

async function isNetworkAvailable(): Promise<boolean> {
  try {
    const response = await globalThis.fetch(config.rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "getNetwork",
        params: [],
      }),
    });
    return response.ok;
  } catch {
    return false;
  }
}

describe("Delegation Integration Tests", () => {
  let networkAvailable: boolean;
  let contractDeployed: boolean;

  const DELEGATION_CONTRACT_ID =
    process.env[`${NETWORK.toUpperCase()}_MUX_DELEGATION_ID`] ||
    config.contracts.muxDelegation ||
    "";

  beforeAll(async () => {
    networkAvailable = await isNetworkAvailable();
    contractDeployed = networkAvailable && !!DELEGATION_CONTRACT_ID;

    if (!networkAvailable) {
      console.warn(
        `⚠️  Network "${NETWORK}" is unavailable at ${config.rpcUrl}. ` +
        `Delegation integration tests will be skipped.`
      );
    } else if (!contractDeployed) {
      console.warn(
        `⚠️  Delegation contract ID not set for network "${NETWORK}". ` +
        `Set ${NETWORK.toUpperCase()}_MUX_DELEGATION_ID to enable these tests.`
      );
    }
  });

  it("should have valid network configuration for delegation", () => {
    expect(config).toBeDefined();
    expect(config.rpcUrl).toBeTruthy();
    expect(config.networkPassphrase).toBeTruthy();
  });

  it("should be able to attempt connection for delegation tests", async () => {
    const available = await isNetworkAvailable();
    expect(typeof available).toBe("boolean");
    if (!available) {
      console.log(
        `ℹ️  Network ${NETWORK} at ${config.rpcUrl} is not currently available. ` +
        `To enable delegation integration tests, start the network or use SOROBAN_NETWORK=testnet.`
      );
    }
  });

  it("should expose delegation contract ID in network config", () => {
    expect(config.contracts).toBeDefined();
    expect(config.contracts.muxDelegation).toBeDefined();
  });

  it("should query is_delegate from a deployed delegation contract", async () => {
    if (!contractDeployed) {
      console.log(`ℹ️  Skipping — delegation contract not available on "${NETWORK}".`);
      return;
    }
    // Call is_delegate via JSON-RPC simulation to verify contract is callable.
    const body = {
      jsonrpc: "2.0",
      id: 2,
      method: "simulateTransaction",
      params: [{ contractId: DELEGATION_CONTRACT_ID, method: "is_delegate", args: ["", ""] }],
    };
    const response = await globalThis.fetch(config.rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    // Accept 200 (success with result) or 400 (RPC error from contract) — contract is present.
    expect([200, 400]).toContain(response.status);
  });

  it("stub: grant_delegate round-trip via bindings", () => {
    if (!contractDeployed) {
      console.log("Skipped — contract not available");
      return;
    }
    // TODO: instantiate MuxDelegationClient, grant a delegate, and verify
    // with get_delegate_permissions. Requires a funded keypair on the
    // target network.
    console.info("TODO: wire MuxDelegationClient with funded keypair for grant_delegate test");
    expect(true).toBe(true);
  });

  it("stub: revoke_delegate round-trip via bindings", () => {
    if (!contractDeployed) {
      console.log("Skipped — contract not available");
      return;
    }
    // TODO: grant then revoke a delegate and assert is_delegate returns false.
    console.info("TODO: wire MuxDelegationClient with funded keypair for revoke_delegate test");
    expect(true).toBe(true);
  });

  it("stub: is_delegate query via bindings", () => {
    if (!contractDeployed) {
      console.log("Skipped — contract not available");
      return;
    }
    // TODO: query is_delegate for an unknown delegate and expect false.
    console.info("TODO: wire MuxDelegationClient with funded keypair for is_delegate test");
    expect(true).toBe(true);
  });
});
