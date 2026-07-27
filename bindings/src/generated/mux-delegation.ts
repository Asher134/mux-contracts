/**
 * AUTO-GENERATED STYLE — hand-authored for mux-delegation.
 *
 * Contract: mux-delegation
 *
 * Provides a client for the MuxDelegation contract with optional filtering
 * query parameters on read methods and a convenience `checkDelegate` method.
 */

import {
  Address,
  Contract,
  Keypair,
  nativeToScVal,
  scValToNative,
  SorobanRpc,
  Transaction,
  TransactionBuilder,
  xdr,
} from "@stellar/stellar-sdk";
import type { MuxDelegationError } from "../types";
import { pollTransaction } from "../horizon";

// ── Types ─────────────────────────────────────────────────────────────────────

/**
 * Optional filter parameters for delegation read queries.
 * Filters are applied client-side after the on-chain call returns.
 */
export interface DelegationQueryFilters {
  /** Filter by permission symbol */
  permission?: string;
  /** Only include delegates with at least one permission */
  hasAnyPermission?: boolean;
}

// ── Client options ────────────────────────────────────────────────────────────

export interface MuxDelegationClientOptions {
  contractId: string;
  networkPassphrase: string;
  rpcUrl: string;
}

// ── Client ────────────────────────────────────────────────────────────────────

export class MuxDelegationClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private networkPassphrase: string;

  constructor(opts: MuxDelegationClientOptions) {
    this.contract = new Contract(opts.contractId);
    this.server = new SorobanRpc.Server(opts.rpcUrl, { allowHttp: false });
    this.networkPassphrase = opts.networkPassphrase;
  }

  // ── Write operations ────────────────────────────────────────────────────────

  async grantDelegate(
    sourceKeypair: Keypair,
    owner: Address,
    delegate: Address,
    permissions: string[]
  ): Promise<void> {
    const tx = await this.buildTx(sourceKeypair, "grant_delegate", [
      nativeToScVal(owner.toString(), { type: "address" }),
      nativeToScVal(delegate.toString(), { type: "address" }),
      xdr.ScVal.scvVec(permissions.map((p) => xdr.ScVal.scvSymbol(p))),
    ]);
    await this.submit(tx, sourceKeypair);
  }

  async revokeDelegate(
    sourceKeypair: Keypair,
    owner: Address,
    delegate: Address
  ): Promise<void> {
    const tx = await this.buildTx(sourceKeypair, "revoke_delegate", [
      nativeToScVal(owner.toString(), { type: "address" }),
      nativeToScVal(delegate.toString(), { type: "address" }),
    ]);
    await this.submit(tx, sourceKeypair);
  }

  // ── Read operations ─────────────────────────────────────────────────────────

  /**
   * Return the permissions granted by `owner` to `delegate`.
   *
   * Accepts optional `DelegationQueryFilters` to narrow results client-side:
   * - `permission`: only return that permission if present in the grant.
   * - `hasAnyPermission`: if `true`, returns the full list only when non-empty;
   *   if `false`, returns the full list only when empty.
   */
  async getDelegatePermissions(
    sourceKeypair: Keypair,
    owner: Address,
    delegate: Address,
    filters?: DelegationQueryFilters
  ): Promise<string[]> {
    const tx = await this.buildTx(sourceKeypair, "get_delegate_permissions", [
      nativeToScVal(owner.toString(), { type: "address" }),
      nativeToScVal(delegate.toString(), { type: "address" }),
    ]);
    const result = await this.simulateRead<string[]>(tx);
    return this.applyPermissionFilters(result, filters);
  }

  async isDelegate(
    sourceKeypair: Keypair,
    owner: Address,
    delegate: Address,
    permission: string
  ): Promise<boolean> {
    const tx = await this.buildTx(sourceKeypair, "is_delegate", [
      nativeToScVal(owner.toString(), { type: "address" }),
      nativeToScVal(delegate.toString(), { type: "address" }),
      xdr.ScVal.scvSymbol(permission),
    ]);
    return this.simulateRead<boolean>(tx);
  }

  /**
   * Return all delegates registered under `owner`.
   *
   * Accepts optional `DelegationQueryFilters` to narrow results client-side:
   * - `permission`: only include delegates that have been granted this specific
   *   permission (requires an additional `getDelegatePermissions` call per delegate).
   * - `hasAnyPermission`: if `true`, only include delegates with at least one
   *   permission in the current grant set (no-op here since all listed delegates
   *   have at least one permission; included for API symmetry).
   */
  async getDelegates(
    sourceKeypair: Keypair,
    owner: Address,
    filters?: DelegationQueryFilters
  ): Promise<Address[]> {
    const tx = await this.buildTx(sourceKeypair, "get_delegates", [
      nativeToScVal(owner.toString(), { type: "address" }),
    ]);
    const result = await this.simulateRead<Address[]>(tx);
    return this.applyDelegateFilters(sourceKeypair, owner, result, filters);
  }

  /**
   * Convenience read-only check: returns `true` if `owner` has granted
   * `permission` to `delegate`, `false` otherwise (including when no grant
   * exists at all).
   *
   * Calls the `check_delegate` on-chain entrypoint which returns `Ok(())`
   * for a match or `Err(NotADelegate)` when the permission is absent.
   */
  async checkDelegate(
    sourceKeypair: Keypair,
    owner: Address,
    delegate: Address,
    permission: string
  ): Promise<boolean> {
    try {
      const tx = await this.buildTx(sourceKeypair, "check_delegate", [
        nativeToScVal(owner.toString(), { type: "address" }),
        nativeToScVal(delegate.toString(), { type: "address" }),
        xdr.ScVal.scvSymbol(permission),
      ]);
      await this.simulateRead<void>(tx);
      return true;
    } catch {
      return false;
    }
  }

  // ── Private filter helpers ───────────────────────────────────────────────────

  private applyPermissionFilters(
    permissions: string[],
    filters?: DelegationQueryFilters
  ): string[] {
    if (!filters) return permissions;
    let result = permissions;
    if (filters.permission !== undefined) {
      result = result.filter((p) => p === filters.permission);
    }
    if (filters.hasAnyPermission === true && result.length === 0) {
      return [];
    }
    if (filters.hasAnyPermission === false && result.length > 0) {
      return [];
    }
    return result;
  }

  private async applyDelegateFilters(
    sourceKeypair: Keypair,
    owner: Address,
    delegates: Address[],
    filters?: DelegationQueryFilters
  ): Promise<Address[]> {
    if (!filters) return delegates;
    // If a specific permission filter is given, further narrow the list by
    // checking each delegate's granted permissions client-side.
    if (filters.permission !== undefined) {
      const filtered: Address[] = [];
      for (const d of delegates) {
        const perms = await this.getDelegatePermissions(
          sourceKeypair,
          owner,
          d
        );
        if (perms.includes(filters.permission)) {
          filtered.push(d);
        }
      }
      return filtered;
    }
    // hasAnyPermission is always true for delegates returned by get_delegates
    // (they all have an active grant), so no additional filtering is needed.
    return delegates;
  }

  // ── Private helpers ──────────────────────────────────────────────────────────

  private async buildTx(
    sourceKeypair: Keypair,
    method: string,
    args: xdr.ScVal[]
  ): Promise<Transaction> {
    const account = await this.server.getAccount(sourceKeypair.publicKey());
    return new TransactionBuilder(account, {
      fee: "100",
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();
  }

  private async simulateRead<T>(tx: Transaction): Promise<T> {
    const result = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation failed: ${result.error}`);
    }
    const retval = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse).result?.retval;
    if (!retval) throw new Error("No return value");
    return scValToNative(retval) as T;
  }

  private async submit(tx: Transaction, signer: Keypair): Promise<void> {
    const simResult = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(`Simulation failed: ${simResult.error}`);
    }
    const prepared = SorobanRpc.assembleTransaction(
      tx,
      simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse
    ).build();
    prepared.sign(signer);
    const sendResult = await this.server.sendTransaction(prepared);
    if (sendResult.status === "ERROR") {
      throw new Error(`Transaction failed: ${JSON.stringify(sendResult.errorResult)}`);
    }
    await pollTransaction(this.server, sendResult.hash);
  }
}
