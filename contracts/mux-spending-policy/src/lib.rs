/*!
 * mux-spending-policy: Spending-policy enforcement contract for Mux Protocol.
 *
 * Stores per-account spend limits and validates spend requests against them.
 *
 * # `no_std` Constraints
 *
 * This crate is `#![no_std]` and does not use `extern crate alloc`.
 * All data structures use Soroban SDK types backed by the Soroban host.
 *
 * ## Audit Events
 *
 * This contract emits the following events:
 * - `initialize`: Emitted when the contract is initialized with an admin address.
 * - `lmt_set`: Emitted when a spending limit policy is created or updated.
 *
 * Events can be queried via the Soroban RPC `getEvents` endpoint with contract ID filter.
 */

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env};

// ── Audit events ──────────────────────────────────────────────────────────────
fn emit(
    env: &Env,
    action: soroban_sdk::Symbol,
    data: impl soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
) {
    env.events()
        .publish((symbol_short!("mux_spend"), action), data);
}

// ── Storage keys ──────────────────────────────────────────────────────────────

/// Key space used by the spending-policy contract.
#[contracttype]
pub enum DataKey {
    /// Instance storage key for the admin address.
    Admin,
    /// Persistent policy record keyed by (account, asset).
    SpendLimit(Address, Address),
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// A spend policy describing the maximum spendable amount for a given asset
/// within a rolling time window measured in ledgers.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendLimit {
    /// Asset identifier associated with the policy.
    pub asset: Address,
    /// Maximum amount that may be spent per period window.
    pub limit: i128,
    /// Amount spent in the current period window.
    pub spent: i128,
    /// Ledger sequence at which the current window expires and `spent` resets.
    pub reset_ledger: u32,
    /// Length of one period window in ledgers (≈ 17 280 at 5-second close ≈ 1 day).
    /// Must be > 0.
    pub period_ledgers: u32,
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors returned by the spending-policy contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SpendingPolicyError {
    /// The contract has not been initialized.
    NotInitialized = 1,
    /// The contract was already initialized.
    AlreadyInitialized = 2,
    /// The caller is not authorized to perform the requested action.
    Unauthorized = 3,
    /// No spend policy exists for the requested account/asset pair.
    PolicyNotFound = 4,
    /// The requested spend exceeds the configured policy limit.
    SpendLimitExceeded = 5,
    /// The provided input is invalid (for example a non-positive limit).
    InvalidInput = 6,
    /// The provided period is invalid (zero is not allowed).
    InvalidPeriod = 7,
}

// ── Storage TTL ───────────────────────────────────────────────────────────────
// STORAGE-GRIEFING (T-21): extend instance TTL on every write so the policy
// registry stays live as long as it is actively used.  See docs/storage-griefing.md.
const TTL_THRESHOLD: u32 = 17_280; // ~1 day
const TTL_EXTEND_TO: u32 = 518_400; // ~30 days

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MuxSpendingPolicy;

#[contractimpl]
impl MuxSpendingPolicy {
    /// Initialize the contract with the admin address that may manage policies.
    pub fn initialize(env: Env, admin: Address) -> Result<(), SpendingPolicyError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(SpendingPolicyError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        emit(&env, symbol_short!("init"), admin);
        Ok(())
    }

    /// Set or replace the spend limit for an account/asset pair.
    ///
    /// Only the initialized admin can change policies. The configured limit
    /// must be strictly positive. `period_ledgers` sets the rolling window
    /// length in ledgers and must be > 0 (≈ 17 280 ledgers ≈ 1 day at 5-second
    /// ledger close). The spent counter is reset to 0 on every call.
    pub fn set_policy(
        env: Env,
        account: Address,
        asset: Address,
        limit: i128,
        period_ledgers: u32,
    ) -> Result<(), SpendingPolicyError> {
        Self::require_admin(&env)?;
        if limit <= 0 {
            return Err(SpendingPolicyError::InvalidInput);
        }
        if period_ledgers == 0 {
            return Err(SpendingPolicyError::InvalidPeriod);
        }
        let policy = SpendLimit {
            asset: asset.clone(),
            limit,
            spent: 0,
            reset_ledger: env.ledger().sequence().saturating_add(period_ledgers),
            period_ledgers,
        };
        env.storage()
            .instance()
            .set(&DataKey::SpendLimit(account.clone(), asset.clone()), &policy);
        Self::extend_ttl(&env);
        emit(&env, symbol_short!("lmt_set"), (account, asset, limit, period_ledgers));
        Ok(())
    }

    /// Get the spend limit for an account/asset pair.
    pub fn get_policy(
        env: Env,
        account: Address,
        asset: Address,
    ) -> Result<SpendLimit, SpendingPolicyError> {
        env.storage()
            .instance()
            .get(&DataKey::SpendLimit(account, asset))
            .ok_or(SpendingPolicyError::PolicyNotFound)
    }

    /// Check whether `amount` is within the policy limit for `account`/`asset`.
    ///
    /// The check is evaluated against a rolling period window. If the window
    /// has expired (current ledger >= `reset_ledger`), the spent counter is
    /// reset and the window advanced before checking. The updated record is
    /// persisted so the reset is durable.
    ///
    /// Returns `Ok(())` when the spend is allowed, `Err(SpendLimitExceeded)`
    /// when the cumulative spend would exceed the configured limit,
    /// `Err(PolicyNotFound)` when no policy is configured, and
    /// `Err(InvalidInput)` for negative amounts.
    pub fn check_spend(
        env: Env,
        account: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), SpendingPolicyError> {
        if amount < 0 {
            return Err(SpendingPolicyError::InvalidInput);
        }
        let key = DataKey::SpendLimit(account.clone(), asset.clone());
        let mut policy: SpendLimit = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(SpendingPolicyError::PolicyNotFound)?;

        // Auto-reset window if elapsed.
        if env.ledger().sequence() >= policy.reset_ledger {
            policy.spent = 0;
            policy.reset_ledger = env
                .ledger()
                .sequence()
                .saturating_add(policy.period_ledgers);
        }

        let new_spent = policy
            .spent
            .checked_add(amount)
            .ok_or(SpendingPolicyError::SpendLimitExceeded)?;

        if new_spent > policy.limit {
            return Err(SpendingPolicyError::SpendLimitExceeded);
        }

        // Persist the updated spent counter (and any window reset).
        policy.spent = new_spent;
        env.storage().instance().set(&key, &policy);
        Self::extend_ttl(&env);
        Ok(())
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn require_admin(env: &Env) -> Result<(), SpendingPolicyError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(SpendingPolicyError::NotInitialized)?;
        admin.require_auth();
        Ok(())
    }

    /// Extend instance-storage TTL on every write to prevent silent data loss (T-21).
    fn extend_ttl(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events},
        Env, FromVal,
    };

    fn topic_action(
        env: &Env,
        events: &soroban_sdk::Vec<(
            soroban_sdk::Address,
            soroban_sdk::Vec<soroban_sdk::Val>,
            soroban_sdk::Val,
        )>,
        idx: u32,
    ) -> soroban_sdk::Symbol {
        let (_, topics, _) = events.get(idx).unwrap();
        soroban_sdk::Symbol::from_val(env, &topics.get(1).unwrap())
    }

    fn setup() -> (Env, MuxSpendingPolicyClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    // ── initialize ────────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_emits_init_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(topic_action(&env, &events, 0), symbol_short!("init"));
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        assert!(client.try_initialize(&admin).is_ok());
        assert!(client.try_initialize(&admin).is_err());
    }

    #[test]
    fn test_double_initialize_fails() {
        let (env, client, admin) = setup();
        assert_eq!(
            client.try_initialize(&admin),
            Err(Ok(SpendingPolicyError::AlreadyInitialized))
        );
    }

    // ── set_policy ────────────────────────────────────────────────────────────

    #[test]
    fn test_set_policy_emits_lmt_set_event() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &1000, &17280);
        let events = env.events().all();
        // init + lmt_set
        assert_eq!(events.len(), 2);
        assert_eq!(topic_action(&env, &events, 1), symbol_short!("lmt_set"));
    }

    #[test]
    fn test_set_and_get_policy() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &1000, &17280);
        let policy = client.get_policy(&account, &asset);
        assert_eq!(policy.limit, 1000);
        assert_eq!(policy.asset, asset);
        assert_eq!(policy.period_ledgers, 17280);
        assert_eq!(policy.spent, 0);
    }

    #[test]
    fn test_set_policy_overwrites_existing() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &500, &17280);
        client.set_policy(&account, &asset, &2000, &17280);
        let policy = client.get_policy(&account, &asset);
        assert_eq!(policy.limit, 2000);
    }

    #[test]
    fn test_set_policy_emits_event_on_update() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &100, &17280);
        client.set_policy(&account, &asset, &200, &17280);
        let events = env.events().all();
        // init + lmt_set + lmt_set
        assert_eq!(events.len(), 3);
        assert_eq!(topic_action(&env, &events, 2), symbol_short!("lmt_set"));
    }

    // ── get_policy ────────────────────────────────────────────────────────────

    #[test]
    fn test_get_policy_not_found() {
        let (env, client, _) = setup();
        let result = client.try_get_policy(&Address::generate(&env), &Address::generate(&env));
        assert_eq!(result, Err(Ok(SpendingPolicyError::PolicyNotFound)));
    }

    // ── check_spend ───────────────────────────────────────────────────────────

    #[test]
    fn test_check_spend_within_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &1000, &17280);
        assert!(client.try_check_spend(&account, &asset, &500).is_ok());
    }

    #[test]
    fn test_check_spend_exceeds_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &1000, &17280);
        let result = client.try_check_spend(&account, &asset, &1001);
        assert_eq!(result, Err(Ok(SpendingPolicyError::SpendLimitExceeded)));
    }

    #[test]
    fn test_check_spend_no_policy() {
        let (env, client, _) = setup();
        let result =
            client.try_check_spend(&Address::generate(&env), &Address::generate(&env), &1);
        assert_eq!(result, Err(Ok(SpendingPolicyError::PolicyNotFound)));
    }

    #[test]
    fn test_set_policy_rejects_non_positive_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        let result = client.try_set_policy(&account, &asset, &0, &17280);
        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();
        assert_eq!(err, SpendingPolicyError::InvalidInput);
    }

    #[test]
    fn test_check_spend_rejects_negative_amount() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        client.set_policy(&account, &asset, &1000, &17280);
        let result = client.try_check_spend(&account, &asset, &-1);
        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();
        assert_eq!(err, SpendingPolicyError::InvalidInput);
    }

    #[test]
    fn test_invalid_input_error_code() {
        assert_eq!(SpendingPolicyError::InvalidInput as u32, 6);
    }

    // ── Unit Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_accounts_same_asset() {
        let (env, client, _) = setup();
        let asset = Address::generate(&env);
        let account1 = Address::generate(&env);
        let account2 = Address::generate(&env);
        
        client.set_policy(&account1, &asset, &1000, &17280);
        client.set_policy(&account2, &asset, &2000, &17280);
        
        let policy1 = client.get_policy(&account1, &asset);
        let policy2 = client.get_policy(&account2, &asset);
        
        assert_eq!(policy1.limit, 1000);
        assert_eq!(policy2.limit, 2000);
        assert_eq!(policy1.asset, asset);
        assert_eq!(policy2.asset, asset);
    }

    #[test]
    fn test_multiple_assets_same_account() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        
        client.set_policy(&account, &asset1, &1000, &17280);
        client.set_policy(&account, &asset2, &5000, &17280);
        
        let policy1 = client.get_policy(&account, &asset1);
        let policy2 = client.get_policy(&account, &asset2);
        
        assert_eq!(policy1.limit, 1000);
        assert_eq!(policy1.asset, asset1);
        assert_eq!(policy2.limit, 5000);
        assert_eq!(policy2.asset, asset2);
    }

    #[test]
    fn test_policy_boundary_values() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        let max_limit = i128::MAX;
        client.set_policy(&account, &asset, &max_limit, &17280);
        let policy = client.get_policy(&account, &asset);
        assert_eq!(policy.limit, max_limit);
        
        assert!(client.try_check_spend(&account, &asset, &max_limit).is_ok());
    }

    #[test]
    fn test_check_spend_at_exact_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        let limit = 1000_i128;
        
        client.set_policy(&account, &asset, &limit, &17280);
        
        assert!(client.try_check_spend(&account, &asset, &limit).is_ok());
        // spent is now equal to limit; one more should fail
        assert_eq!(
            client.try_check_spend(&account, &asset, &1),
            Err(Ok(SpendingPolicyError::SpendLimitExceeded))
        );
    }

    #[test]
    fn test_check_spend_zero_amount() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000, &17280);
        assert!(client.try_check_spend(&account, &asset, &0).is_ok());
    }

    #[test]
    fn test_error_codes_mapping() {
        // Verify all error codes have expected discriminant values
        assert_eq!(SpendingPolicyError::NotInitialized as u32, 1);
        assert_eq!(SpendingPolicyError::AlreadyInitialized as u32, 2);
        assert_eq!(SpendingPolicyError::Unauthorized as u32, 3);
        assert_eq!(SpendingPolicyError::PolicyNotFound as u32, 4);
        assert_eq!(SpendingPolicyError::SpendLimitExceeded as u32, 5);
    }

    // ── Negative Path Tests ────────────────────────────────────────────────────

    #[test]
    fn test_check_spend_with_negative_amount() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000, &17280);
        
        let result = client.try_check_spend(&account, &asset, &-500);
        assert_eq!(result, Err(Ok(SpendingPolicyError::InvalidInput)));
    }

    #[test]
    fn test_get_policy_not_found_specific_error() {
        let (env, client, _) = setup();
        let nonexistent_account = Address::generate(&env);
        let nonexistent_asset = Address::generate(&env);
        
        let result = client.try_get_policy(&nonexistent_account, &nonexistent_asset);
        assert_eq!(result, Err(Ok(SpendingPolicyError::PolicyNotFound)));
    }

    #[test]
    fn test_check_spend_policy_not_found_error() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        let result = client.try_check_spend(&account, &asset, &500);
        assert_eq!(result, Err(Ok(SpendingPolicyError::PolicyNotFound)));
    }

    #[test]
    fn test_check_spend_exceeds_limit_error() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000, &17280);
        
        let result = client.try_check_spend(&account, &asset, &1001);
        assert_eq!(result, Err(Ok(SpendingPolicyError::SpendLimitExceeded)));
    }

    #[test]
    fn test_check_spend_far_exceeds_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000, &17280);
        
        let result = client.try_check_spend(&account, &asset, &1_000_000);
        assert_eq!(result, Err(Ok(SpendingPolicyError::SpendLimitExceeded)));
    }

    #[test]
    fn test_set_policy_overwrites_completely() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        
        client.set_policy(&account, &asset1, &10000, &17280);
        let policy1 = client.get_policy(&account, &asset1);
        assert_eq!(policy1.asset, asset1);
        assert_eq!(policy1.limit, 10000);
        
        client.set_policy(&account, &asset2, &5000, &17280);
        
        let policy1_check = client.get_policy(&account, &asset1);
        let policy2_check = client.get_policy(&account, &asset2);
        
        assert_eq!(policy1_check.limit, 10000);
        assert_eq!(policy2_check.limit, 5000);
    }

    #[test]
    fn test_get_policy_returns_correct_asset() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &2500, &17280);
        let policy = client.get_policy(&account, &asset);
        
        assert_eq!(policy.asset, asset);
        assert_eq!(policy.limit, 2500);
    }

    #[test]
    fn test_sequential_check_spend_calls() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000, &17280);
        
        // Cumulative spend: 100 + 200 + 500 = 800; still within limit
        assert!(client.try_check_spend(&account, &asset, &100).is_ok());
        assert!(client.try_check_spend(&account, &asset, &200).is_ok());
        assert!(client.try_check_spend(&account, &asset, &500).is_ok());
        
        // 800 spent; 201 more would exceed 1000
        assert_eq!(
            client.try_check_spend(&account, &asset, &201),
            Err(Ok(SpendingPolicyError::SpendLimitExceeded))
        );
    }

    #[test]
    fn test_policy_with_min_positive_limit() {
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1, &17280);
        let policy = client.get_policy(&account, &asset);
        
        assert_eq!(policy.limit, 1);
        assert!(client.try_check_spend(&account, &asset, &1).is_ok());
        assert_eq!(
            client.try_check_spend(&account, &asset, &1),
            Err(Ok(SpendingPolicyError::SpendLimitExceeded))
        );
    }

    // ── Audit Event Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_initialize_emits_event() {
        use soroban_sdk::testutils::Events;
        
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        
        client.initialize(&admin);
        
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        
        let (_, topics, data) = events.get(0).unwrap();
        assert_eq!(topics.len(), 2);
        
        // Verify topics
        let contract_tag = soroban_sdk::Symbol::from_val(&env, &topics.get(0).unwrap());
        let action = soroban_sdk::Symbol::from_val(&env, &topics.get(1).unwrap());
        assert_eq!(contract_tag, symbol_short!("mux_spend"));
        assert_eq!(action, symbol_short!("init"));
    }

    #[test]
    fn test_set_policy_emits_event() {
        use soroban_sdk::testutils::Events;
        
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        // Clear events from setup (initialize event)
        env.events().all();
        
        client.set_policy(&account, &asset, &1000);
        
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        
        let (_, topics, _) = events.get(0).unwrap();
        
        // Verify topics
        let contract_tag = soroban_sdk::Symbol::from_val(&env, &topics.get(0).unwrap());
        let action = soroban_sdk::Symbol::from_val(&env, &topics.get(1).unwrap());
        assert_eq!(contract_tag, symbol_short!("mux_spend"));
        assert_eq!(action, symbol_short!("lmt_set"));
    }

    #[test]
    fn test_multiple_events_emitted() {
        use soroban_sdk::testutils::Events;
        
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let account1 = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let account2 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        
        // Initialize
        client.initialize(&admin);
        
        // Set first policy
        client.set_policy(&account1, &asset1, &1000);
        
        // Set second policy
        client.set_policy(&account2, &asset2, &2000);
        
        let events = env.events().all();
        
        // Should have 3 events: initialize + 2 set_policy
        assert_eq!(events.len(), 3);
        
        // Verify first event is initialize
        let (_, topics1, _) = events.get(0).unwrap();
        let action1 = soroban_sdk::Symbol::from_val(&env, &topics1.get(1).unwrap());
        assert_eq!(action1, symbol_short!("init"));
        
        // Verify second event is set_policy
        let (_, topics2, _) = events.get(1).unwrap();
        let action2 = soroban_sdk::Symbol::from_val(&env, &topics2.get(1).unwrap());
        assert_eq!(action2, symbol_short!("lmt_set"));
        
        // Verify third event is set_policy
        let (_, topics3, _) = events.get(2).unwrap();
        let action3 = soroban_sdk::Symbol::from_val(&env, &topics3.get(1).unwrap());
        assert_eq!(action3, symbol_short!("lmt_set"));
    }

    #[test]
    fn test_check_spend_does_not_emit_event() {
        use soroban_sdk::testutils::Events;
        
        let (env, client, _) = setup();
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        
        client.set_policy(&account, &asset, &1000);
        
        // Clear events from setup and set_policy
        env.events().all();
        
        // check_spend should not emit events (read-only operation)
        client.check_spend(&account, &asset, &500);
        
        let events = env.events().all();
        assert_eq!(events.len(), 0);
    }

    // ── NotInitialized tests (#505) ──────────────────────────────────────────

    /// `set_policy` must return `NotInitialized` when called before `initialize`.
    #[test]
    fn test_set_policy_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        let result = client.try_set_policy(&account, &asset, &1000);
        assert_eq!(result, Err(Ok(SpendingPolicyError::NotInitialized)));
    }

    /// `check_spend` must return `NotInitialized` when called before `initialize`.
    #[test]
    fn test_check_spend_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxSpendingPolicy);
        let client = MuxSpendingPolicyClient::new(&env, &contract_id);
        let account = Address::generate(&env);
        let asset = Address::generate(&env);
        let result = client.try_check_spend(&account, &asset, &100);
        assert_eq!(result, Err(Ok(SpendingPolicyError::NotInitialized)));
    }

    /// All NotInitialized error codes must match their declared discriminant.
    #[test]
    fn test_not_initialized_error_code() {
        assert_eq!(SpendingPolicyError::NotInitialized as u32, 1);
    }

    // ── symbol_short length audit (#496) ─────────────────────────────────────

    /// All contract tag and action symbols must be <= 8 bytes so that
    /// `symbol_short!` produces valid Soroban symbols.
    #[test]
    fn test_symbol_short_lengths_within_limit() {
        let tags = [
            symbol_short!("mux_spend"),
        ];
        let actions = [
            symbol_short!("init"),
            symbol_short!("lmt_set"),
        ];
        for sym in tags.iter().chain(actions.iter()) {
            assert!(sym.to_val().len() <= 8);
        }
    }
}
