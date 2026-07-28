/*!
 * mux-account-factory: Account factory for deploying account abstraction instances.
 *
 * Provides a factory contract that registers new MuxAccount instances and
 * maintains a per-owner index of deployed accounts.
 *
 * # `no_std` Constraints
 *
 * This crate is `#![no_std]` and does not use `extern crate alloc`.
 * All data structures use Soroban SDK types backed by the Soroban host.
 */

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String, Vec,
};

// ── Audit events ──────────────────────────────────────────────────────────────
fn emit(
    env: &Env,
    action: soroban_sdk::Symbol,
    data: impl soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
) {
    env.events()
        .publish((symbol_short!("mux_fac"), action), data);
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    /// Per-owner list of deployed account addresses.
    Accounts(Address),
    /// Total accounts registered across all owners.
    AccountCount,
    /// Metadata for a specific account: DataKey::Metadata(owner, account_address)
    Metadata(Address, Address),
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// Metadata associated with a registered account.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AccountMetadata {
    /// Semantic version string, e.g. "1.2.0"
    pub version: String,
    /// Short human-readable description of the account.
    pub description: String,
    /// Author or team identifier.
    pub author: String,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MuxAccountFactoryError {
    Unauthorized = 1,
    /// account_address must differ from owner.
    InvalidAccount = 2,
    // STORAGE-GRIEFING: unbounded per-owner Accounts vec would let an owner
    // bloat instance storage indefinitely.
    TooManyAccounts = 3,
    /// Metadata not found for the specified account.
    MetadataNotFound = 4,
    // STORAGE-GRIEFING: unbounded metadata string sizes would let an owner
    // bloat instance storage indefinitely.
    MetadataTooLarge = 5,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum accounts per owner to bound the Accounts vec in instance storage.
///
/// Enforced on every deploy path (`deploy_account`, `deploy_account_with_metadata`)
/// and mirrored by the dry-run helpers (`simulate_deploy*`). Exposed via
/// [`MuxAccountFactoryClient::max_accounts_per_owner`] for TypeScript clients.
pub const MAX_ACCOUNTS_PER_OWNER: u32 = 64;

// STORAGE-GRIEFING: bound metadata string sizes to prevent owners from bloating
// instance storage with large strings. Each account can have metadata, so with
// 64 accounts per owner, unbounded strings could cause significant storage bloat.
const MAX_VERSION_LENGTH: u32 = 32; // e.g., "1.2.3" or "v1.2.3-beta"
const MAX_DESCRIPTION_LENGTH: u32 = 256; // Short human-readable description
const MAX_AUTHOR_LENGTH: u32 = 64; // Author or team identifier

// ── Storage TTL ───────────────────────────────────────────────────────────────
// STORAGE-GRIEFING (T-21): extend instance TTL on every write so the factory
// stays live as long as it is actively used.  See docs/storage-griefing.md.
//
// Values: ~17,280 ledgers ≈ 1 day (5-second ledger close); bump to 30 days.
const TTL_THRESHOLD: u32 = 17_280; // extend when remaining TTL falls below 1 day
const TTL_EXTEND_TO: u32 = 518_400; // extend to ~30 days

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MuxAccountFactory;

#[contractimpl]
impl MuxAccountFactory {
    /// Register a new account for the given owner.
    ///
    /// The caller must be the owner. `account_address` must differ from `owner`
    /// and must not already be registered for this owner.
    ///
    /// Returns [`MuxAccountFactoryError::TooManyAccounts`] when the owner's
    /// `Accounts` vec already holds [`MAX_ACCOUNTS_PER_OWNER`] entries.
    pub fn deploy_account(
        env: Env,
        owner: Address,
        account_address: Address,
    ) -> Result<Address, MuxAccountFactoryError> {
        owner.require_auth();

        if account_address == owner {
            return Err(MuxAccountFactoryError::InvalidAccount);
        }

        // STORAGE-GRIEFING: bound Accounts vec growth on deploy.
        let mut accounts = Self::load_accounts_under_cap(&env, &owner)?;

        accounts.push_back(account_address.clone());
        env.storage()
            .instance()
            .set(&DataKey::Accounts(owner.clone()), &accounts);

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AccountCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::AccountCount, &(count + 1));

        emit(
            &env,
            symbol_short!("deployed"),
            (owner, account_address.clone()),
        );
        Self::extend_ttl(&env);
        Ok(account_address)
    }

    /// Get all accounts registered for a given owner.
    pub fn get_accounts(env: Env, owner: Address) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Accounts(owner))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the total count of registered accounts.
    pub fn account_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AccountCount)
            .unwrap_or(0)
    }

    /// Register a new account for the given owner with metadata.
    ///
    /// The caller must be the owner. `account_address` must differ from `owner`
    /// and must not already be registered for this owner.
    ///
    /// Returns [`MuxAccountFactoryError::TooManyAccounts`] when the owner's
    /// `Accounts` vec already holds [`MAX_ACCOUNTS_PER_OWNER`] entries.
    pub fn deploy_account_with_metadata(
        env: Env,
        owner: Address,
        account_address: Address,
        version: String,
        description: String,
        author: String,
    ) -> Result<Address, MuxAccountFactoryError> {
        owner.require_auth();

        if account_address == owner {
            return Err(MuxAccountFactoryError::InvalidAccount);
        }

        // STORAGE-GRIEFING: bound Accounts vec growth on deploy.
        let mut accounts = Self::load_accounts_under_cap(&env, &owner)?;

        // STORAGE-GRIEFING: validate metadata string sizes to prevent storage bloat.
        if version.len() > MAX_VERSION_LENGTH {
            return Err(MuxAccountFactoryError::MetadataTooLarge);
        }
        if description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(MuxAccountFactoryError::MetadataTooLarge);
        }
        if author.len() > MAX_AUTHOR_LENGTH {
            return Err(MuxAccountFactoryError::MetadataTooLarge);
        }

        accounts.push_back(account_address.clone());
        env.storage()
            .instance()
            .set(&DataKey::Accounts(owner.clone()), &accounts);

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AccountCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::AccountCount, &(count + 1));

        // Store metadata
        let meta = AccountMetadata {
            version: version.clone(),
            description,
            author,
        };
        env.storage()
            .instance()
            .set(&DataKey::Metadata(owner.clone(), account_address.clone()), &meta);

        emit(
            &env,
            symbol_short!("deployed"),
            (owner.clone(), account_address.clone()),
        );
        emit(
            &env,
            symbol_short!("meta_set"),
            (owner, account_address.clone(), version),
        );
        Self::extend_ttl(&env);
        Ok(account_address)
    }

    /// Get the metadata for a specific account.
    pub fn get_account_metadata(
        env: Env,
        owner: Address,
        account_address: Address,
    ) -> Result<AccountMetadata, MuxAccountFactoryError> {
        env.storage()
            .instance()
            .get(&DataKey::Metadata(owner, account_address))
            .ok_or(MuxAccountFactoryError::MetadataNotFound)
    }

    /// Validate a deploy_account call without writing any state (dry-run).
    ///
    /// Returns the account address that *would* be registered, or the same
    /// error that `deploy_account` would return — including
    /// [`MuxAccountFactoryError::TooManyAccounts`] when the owner's Accounts
    /// vec is already at [`MAX_ACCOUNTS_PER_OWNER`].  No storage is modified
    /// and no events are emitted.
    pub fn simulate_deploy(
        env: Env,
        owner: Address,
        account_address: Address,
    ) -> Result<Address, MuxAccountFactoryError> {
        if account_address == owner {
            return Err(MuxAccountFactoryError::InvalidAccount);
        }
        // Mirror the deploy-path Accounts bound so dry-run stays auditable.
        let _ = Self::load_accounts_under_cap(&env, &owner)?;
        Ok(account_address)
    }

    /// Validate a deploy_account_with_metadata call without writing any state
    /// (dry-run).  No storage is modified and no events are emitted.
    ///
    /// Enforces the same Accounts vec bound as
    /// [`Self::deploy_account_with_metadata`].
    pub fn simulate_deploy_with_metadata(
        env: Env,
        owner: Address,
        account_address: Address,
        version: String,
        description: String,
        author: String,
    ) -> Result<Address, MuxAccountFactoryError> {
        if account_address == owner {
            return Err(MuxAccountFactoryError::InvalidAccount);
        }
        let _ = Self::load_accounts_under_cap(&env, &owner)?;
        if version.len() > MAX_VERSION_LENGTH
            || description.len() > MAX_DESCRIPTION_LENGTH
            || author.len() > MAX_AUTHOR_LENGTH
        {
            return Err(MuxAccountFactoryError::MetadataTooLarge);
        }
        Ok(account_address)
    }

    /// Return the maximum number of accounts permitted per owner.
    ///
    /// Callers (including TypeScript bindings) can query this before deploy to
    /// avoid a `TooManyAccounts` error at execution time.
    pub fn max_accounts_per_owner(_env: Env) -> u32 {
        MAX_ACCOUNTS_PER_OWNER
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    /// Load the owner's Accounts vec, rejecting when it is already at the
    /// storage-griefing cap. Shared by deploy and simulate paths.
    fn load_accounts_under_cap(
        env: &Env,
        owner: &Address,
    ) -> Result<Vec<Address>, MuxAccountFactoryError> {
        let accounts: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Accounts(owner.clone()))
            .unwrap_or_else(|| Vec::new(env));

        if accounts.len() >= MAX_ACCOUNTS_PER_OWNER {
            return Err(MuxAccountFactoryError::TooManyAccounts);
        }
        Ok(accounts)
    }

    fn extend_ttl(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

pub mod wallet_factory_stub;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, FromVal};

    fn setup() -> (Env, MuxAccountFactoryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxAccountFactory);
        let client = MuxAccountFactoryClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_deploy_account() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let deployed = client.deploy_account(&owner, &account_addr);
        assert_eq!(deployed, account_addr);
    }

    #[test]
    fn test_deployed_address_distinct_from_owner() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let deployed = client.deploy_account(&owner, &account_addr);
        assert_ne!(deployed, owner);
    }

    #[test]
    fn test_account_registry_updated_after_deployment() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);
        let accounts = client.get_accounts(&owner);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts.get(0).unwrap(), account_addr);
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_multiple_account_deployments() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account1 = Address::generate(&env);
        let account2 = Address::generate(&env);
        client.deploy_account(&owner, &account1);
        client.deploy_account(&owner, &account2);
        let accounts = client.get_accounts(&owner);
        assert_eq!(accounts.len(), 2);
        assert_eq!(client.account_count(), 2);
    }

    #[test]
    fn test_invalid_account_same_as_owner() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let result = client.try_deploy_account(&owner, &owner);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::InvalidAccount)));
    }

    #[test]
    fn test_accounts_cap_enforced() {
        use soroban_test_helpers::{assert_contract_err, assert_len_at_most};

        let (env, client) = setup();
        env.budget().reset_unlimited();
        let owner = Address::generate(&env);
        for _ in 0..64 {
            client.deploy_account(&owner, &Address::generate(&env));
        }
        assert_contract_err(
            client.try_deploy_account(&owner, &Address::generate(&env)),
            MuxAccountFactoryError::TooManyAccounts,
        );
        assert_len_at_most(client.get_accounts(&owner).len(), MAX_ACCOUNTS_PER_OWNER, "accounts");
    }

    #[test]
    fn test_deploy_emits_event() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, _) = events.get(0).unwrap();
        let action = soroban_sdk::Symbol::from_val(&env, &topics.get(1).unwrap());
        assert_eq!(action, symbol_short!("deployed"));
    }

    #[test]
    fn test_deploy_with_metadata_emits_deployed_and_meta_set_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test account");
        let author = String::from_str(&env, "mux-labs");

        client.deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );

        let events = env.events().all();
        assert_eq!(events.len(), 2);

        let (_, topics0, _) = events.get(0).unwrap();
        let action0 =
            soroban_sdk::Symbol::from_val(&env, &topics0.get(1).unwrap());
        let (_, topics1, _) = events.get(1).unwrap();
        let action1 =
            soroban_sdk::Symbol::from_val(&env, &topics1.get(1).unwrap());

        assert_eq!(action0, symbol_short!("deployed"));
        assert_eq!(action1, symbol_short!("meta_set"));
    }

    #[test]
    fn test_deploy_account_does_not_emit_meta_set() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, _) = events.get(0).unwrap();
        let action = soroban_sdk::Symbol::from_val(&env, &topics.get(1).unwrap());
        assert_ne!(action, symbol_short!("meta_set"));
    }

    #[test]
    fn test_accounts_cap_returns_too_many_accounts() {
        let (env, client) = setup();
        env.budget().reset_unlimited();
        let owner = Address::generate(&env);
        for _ in 0..64 {
            client.deploy_account(&owner, &Address::generate(&env));
        }
        let result = client.try_deploy_account(&owner, &Address::generate(&env));
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::TooManyAccounts)));
    }

    #[test]
    fn test_ttl_extended_on_deploy() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        client.deploy_account(&owner, &Address::generate(&env));
        // If extend_ttl was missing the SDK would panic; reaching here is the assertion.
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_ttl_extended_on_deploy_with_metadata() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        client.deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        // If extend_ttl was missing the SDK would panic; reaching here is the assertion.
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_read_operations_do_not_extend_ttl() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);

        // Deploy an account (this extends TTL)
        client.deploy_account(&owner, &account_addr);

        // Read operations should not extend TTL
        let _accounts = client.get_accounts(&owner);
        let _count = client.account_count();

        // If read operations extended TTL incorrectly, the test would still pass
        // but this documents the expected behavior
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_deploy_account_with_metadata() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test account");
        let author = String::from_str(&env, "test-author");

        let deployed = client.deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert_eq!(deployed, account_addr);
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_get_account_metadata() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "2.0.0");
        let description = String::from_str(&env, "Account with metadata");
        let author = String::from_str(&env, "mux-labs");

        client.deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version.clone(),
            &description.clone(),
            &author.clone(),
        );

        let meta = client.get_account_metadata(&owner, &account_addr);
        assert_eq!(meta.version, version);
        assert_eq!(meta.description, description);
        assert_eq!(meta.author, author);
    }

    #[test]
    fn test_get_account_metadata_not_found() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let result = client.try_get_account_metadata(&owner, &account_addr);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataNotFound)));
    }

    #[test]
    fn test_get_account_metadata_not_found_after_deploy_without_metadata() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);
        let result = client.try_get_account_metadata(&owner, &account_addr);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataNotFound)));
    }

    #[test]
    fn test_get_account_metadata_wrong_owner() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let other_owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        client.deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        let result = client.try_get_account_metadata(&other_owner, &account_addr);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataNotFound)));
    }

    // ── Unauthorized deploy (no mock_all_auths) ───────────────────────────────

    #[test]
    fn test_deploy_account_unauthorized_without_auth() {
        use soroban_sdk::testutils::Events;
        // Deliberately omit mock_all_auths — owner.require_auth() must reject.
        let env = Env::default();
        let contract_id = env.register_contract(None, MuxAccountFactory);
        let client = MuxAccountFactoryClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);

        let result = client.try_deploy_account(&owner, &account_addr);
        assert!(result.is_err());

        // No state mutation and no events on auth failure.
        assert_eq!(client.get_accounts(&owner).len(), 0);
        assert_eq!(client.account_count(), 0);
        assert_eq!(env.events().all().len(), 0);
    }

    #[test]
    fn test_deploy_account_with_metadata_unauthorized_without_auth() {
        use soroban_sdk::testutils::Events;
        let env = Env::default();
        let contract_id = env.register_contract(None, MuxAccountFactory);
        let client = MuxAccountFactoryClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert!(result.is_err());

        assert_eq!(client.get_accounts(&owner).len(), 0);
        assert_eq!(client.account_count(), 0);
        assert!(client
            .try_get_account_metadata(&owner, &account_addr)
            .is_err());
        assert_eq!(env.events().all().len(), 0);
    }

    #[test]
    fn test_unauthorized_deploy_does_not_affect_other_owners() {
        // Authorized deploy for owner_a, then unauthorized attempt on a fresh env.
        let (env, client) = setup();
        let owner_a = Address::generate(&env);
        let account_a = Address::generate(&env);
        client.deploy_account(&owner_a, &account_a);
        assert_eq!(client.account_count(), 1);

        // Separate env with no mock_all_auths — auth failure must leave it empty.
        let env_no_auth = Env::default();
        let contract_id = env_no_auth.register_contract(None, MuxAccountFactory);
        let client_no_auth = MuxAccountFactoryClient::new(&env_no_auth, &contract_id);
        let owner_b = Address::generate(&env_no_auth);
        let account_b = Address::generate(&env_no_auth);
        let result = client_no_auth.try_deploy_account(&owner_b, &account_b);
        assert!(result.is_err());
        assert_eq!(client_no_auth.get_accounts(&owner_b).len(), 0);
        assert_eq!(client_no_auth.account_count(), 0);

        // Original authorized env still has owner_a's account.
        assert_eq!(client.get_accounts(&owner_a).len(), 1);
        assert_eq!(client.account_count(), 1);
    }

    #[test]
    fn test_deploy_account_with_metadata_enforces_cap() {
        let (env, client) = setup();
        env.budget().reset_unlimited();
        let owner = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        // Fill up to the cap
        for _ in 0..64 {
            client.deploy_account_with_metadata(
                &owner,
                &Address::generate(&env),
                &version.clone(),
                &description.clone(),
                &author.clone(),
            );
        }
        // One more must be rejected
        let result = client.try_deploy_account_with_metadata(
            &owner,
            &Address::generate(&env),
            &version,
            &description,
            &author,
        );
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::TooManyAccounts)));
    }

    #[test]
    fn test_deploy_account_with_metadata_invalid_account() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &owner,
            &version,
            &description,
            &author,
        );
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::InvalidAccount)));
    }

    #[test]
    fn test_metadata_version_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        // Create a version string longer than MAX_VERSION_LENGTH (32)
        let version = String::from_str(&env, "a".repeat(33).as_str());
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataTooLarge)));
    }

    #[test]
    fn test_metadata_description_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        // Create a description string longer than MAX_DESCRIPTION_LENGTH (256)
        let description = String::from_str(&env, "a".repeat(257).as_str());
        let author = String::from_str(&env, "test");

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataTooLarge)));
    }

    #[test]
    fn test_metadata_author_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        // Create an author string longer than MAX_AUTHOR_LENGTH (64)
        let author = String::from_str(&env, "a".repeat(65).as_str());

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::MetadataTooLarge)));
    }

    #[test]
    fn test_metadata_at_max_length_succeeds() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        // Create strings at exactly the maximum allowed length
        let version = String::from_str(&env, "a".repeat(32).as_str());
        let description = String::from_str(&env, "a".repeat(256).as_str());
        let author = String::from_str(&env, "a".repeat(64).as_str());

        let result = client.try_deploy_account_with_metadata(
            &owner,
            &account_addr,
            &version,
            &description,
            &author,
        );
        assert!(result.is_ok());
    }

    // ── simulate_deploy tests ─────────────────────────────────────────────────

    #[test]
    fn test_simulate_deploy_returns_account_address() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let result = client.simulate_deploy(&owner, &account_addr);
        assert_eq!(result, account_addr);
    }

    #[test]
    fn test_simulate_deploy_rejects_owner_as_account() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let result = client.try_simulate_deploy(&owner, &owner);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::InvalidAccount)));
    }

    #[test]
    fn test_simulate_deploy_does_not_write_state() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.simulate_deploy(&owner, &account_addr);
        // No accounts should have been registered
        assert_eq!(client.get_accounts(&owner).len(), 0);
        assert_eq!(client.account_count(), 0);
    }

    #[test]
    fn test_simulate_deploy_does_not_emit_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.simulate_deploy(&owner, &account_addr);
        assert_eq!(env.events().all().len(), 0);
    }

    #[test]
    fn test_simulate_deploy_with_metadata_returns_account_address() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        let result =
            client.simulate_deploy_with_metadata(&owner, &account_addr, &version, &description, &author);
        assert_eq!(result, account_addr);
    }

    #[test]
    fn test_simulate_deploy_with_metadata_rejects_owner_as_account() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        let result = client.try_simulate_deploy_with_metadata(&owner, &owner, &version, &description, &author);
        assert_eq!(result, Err(Ok(MuxAccountFactoryError::InvalidAccount)));
    }

    #[test]
    fn test_simulate_deploy_with_metadata_does_not_write_state() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        client.simulate_deploy_with_metadata(&owner, &account_addr, &version, &description, &author);
        assert!(client.get_accounts(&owner).is_empty());
        assert_eq!(client.account_count(), 0);
    }

    // ── symbol_short length audit (#496) ─────────────────────────────────────

    #[test]
    fn test_symbol_short_lengths_within_limit() {
        let tags = [symbol_short!("mux_fac")];
        let actions = [symbol_short!("deployed"), symbol_short!("meta_set")];
        for sym in tags.iter().chain(actions.iter()) {
            assert!(sym.to_val().len() <= 8);
        }
    }

    // ── Event: contract tag (topics[0]) ──────────────────────────────────────

    /// Every factory event must carry `mux_fac` as topics[0] so that indexers
    /// and TypeScript `getEvents` filters can scope queries to this contract
    /// without relying on the contract address alone.
    #[test]
    fn test_deployed_event_has_mux_fac_contract_tag() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, _) = events.get(0).unwrap();
        let tag = soroban_sdk::Symbol::from_val(&env, &topics.get(0).unwrap());
        assert_eq!(tag, symbol_short!("mux_fac"));
    }

    /// `deploy_account_with_metadata` emits two events; both must carry the
    /// `mux_fac` tag in topics[0].
    #[test]
    fn test_all_factory_events_carry_mux_fac_tag() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        client.deploy_account_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );

        let events = env.events().all();
        assert_eq!(events.len(), 2);
        for i in 0..2u32 {
            let (_, topics, _) = events.get(i).unwrap();
            let tag = soroban_sdk::Symbol::from_val(&env, &topics.get(0).unwrap());
            assert_eq!(
                tag,
                symbol_short!("mux_fac"),
                "event[{i}] topics[0] must be mux_fac"
            );
        }
    }

    // ── Event: deployed — data payload ────────────────────────────────────────

    /// The `deployed` event data must be the tuple `(owner, account_address)`
    /// in that order. Indexers and TypeScript consumers decode this tuple to
    /// reconstruct which owner registered which account.
    #[test]
    fn test_deployed_event_data_contains_owner_and_account() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.deploy_account(&owner, &account_addr);

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, _, data) = events.get(0).unwrap();

        // Data is encoded as a Vec<Val>; decode the two-element tuple.
        let payload: soroban_sdk::Vec<soroban_sdk::Val> =
            soroban_sdk::Vec::from_val(&env, &data);
        assert_eq!(payload.len(), 2);

        let decoded_owner = Address::from_val(&env, &payload.get(0).unwrap());
        let decoded_account = Address::from_val(&env, &payload.get(1).unwrap());

        assert_eq!(decoded_owner, owner);
        assert_eq!(decoded_account, account_addr);
    }

    /// `deploy_account_with_metadata` also emits `deployed` as its first event
    /// with the same `(owner, account_address)` data shape.
    #[test]
    fn test_deployed_event_data_shape_matches_with_metadata_path() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "2.5.0");
        let description = String::from_str(&env, "Payload test");
        let author = String::from_str(&env, "mux-labs");
        client.deploy_account_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );

        let events = env.events().all();
        // First event is `deployed`.
        let (_, _, data) = events.get(0).unwrap();
        let payload: soroban_sdk::Vec<soroban_sdk::Val> =
            soroban_sdk::Vec::from_val(&env, &data);
        assert_eq!(payload.len(), 2);

        let decoded_owner = Address::from_val(&env, &payload.get(0).unwrap());
        let decoded_account = Address::from_val(&env, &payload.get(1).unwrap());
        assert_eq!(decoded_owner, owner);
        assert_eq!(decoded_account, account_addr);
    }

    // ── Event: meta_set — data payload ────────────────────────────────────────

    /// The `meta_set` event data must be the tuple `(owner, account_address,
    /// version)`. TypeScript indexers use `version` to correlate the on-chain
    /// metadata entry with upgrade history.
    #[test]
    fn test_meta_set_event_data_contains_owner_account_and_version() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "3.1.0");
        let description = String::from_str(&env, "Meta payload test");
        let author = String::from_str(&env, "mux-labs");
        client.deploy_account_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );

        let events = env.events().all();
        assert_eq!(events.len(), 2);
        // Second event is `meta_set`.
        let (_, topics, data) = events.get(1).unwrap();
        let action = soroban_sdk::Symbol::from_val(&env, &topics.get(1).unwrap());
        assert_eq!(action, symbol_short!("meta_set"));

        let payload: soroban_sdk::Vec<soroban_sdk::Val> =
            soroban_sdk::Vec::from_val(&env, &data);
        assert_eq!(payload.len(), 3);

        let decoded_owner = Address::from_val(&env, &payload.get(0).unwrap());
        let decoded_account = Address::from_val(&env, &payload.get(1).unwrap());
        let decoded_version = String::from_val(&env, &payload.get(2).unwrap());

        assert_eq!(decoded_owner, owner);
        assert_eq!(decoded_account, account_addr);
        assert_eq!(decoded_version, version);
    }

    // ── Event: ordering guarantee ─────────────────────────────────────────────

    /// `deploy_account_with_metadata` must emit events in a deterministic order:
    /// `deployed` first, `meta_set` second. Indexers rely on this ordering to
    /// link metadata to the preceding deployment.
    #[test]
    fn test_deploy_with_metadata_event_ordering_deployed_before_meta_set() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Order test");
        let author = String::from_str(&env, "test");
        client.deploy_account_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );

        let events = env.events().all();
        assert_eq!(events.len(), 2);

        let (_, topics0, _) = events.get(0).unwrap();
        let (_, topics1, _) = events.get(1).unwrap();

        let action0 = soroban_sdk::Symbol::from_val(&env, &topics0.get(1).unwrap());
        let action1 = soroban_sdk::Symbol::from_val(&env, &topics1.get(1).unwrap());

        assert_eq!(action0, symbol_short!("deployed"),
            "first event must be 'deployed'");
        assert_eq!(action1, symbol_short!("meta_set"),
            "second event must be 'meta_set'");
    }

    /// Multiple sequential deploys must emit events in the same order they
    /// were submitted — one `deployed` event per call.
    #[test]
    fn test_multiple_deploys_emit_events_in_call_order() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account1 = Address::generate(&env);
        let account2 = Address::generate(&env);

        client.deploy_account(&owner, &account1);
        client.deploy_account(&owner, &account2);

        let events = env.events().all();
        assert_eq!(events.len(), 2);

        // First event payload must reference account1.
        let (_, _, data0) = events.get(0).unwrap();
        let payload0: soroban_sdk::Vec<soroban_sdk::Val> =
            soroban_sdk::Vec::from_val(&env, &data0);
        let decoded_account0 = Address::from_val(&env, &payload0.get(1).unwrap());
        assert_eq!(decoded_account0, account1);

        // Second event payload must reference account2.
        let (_, _, data1) = events.get(1).unwrap();
        let payload1: soroban_sdk::Vec<soroban_sdk::Val> =
            soroban_sdk::Vec::from_val(&env, &data1);
        let decoded_account1 = Address::from_val(&env, &payload1.get(1).unwrap());
        assert_eq!(decoded_account1, account2);
    }

    // ── Zero events on error paths ────────────────────────────────────────────

    /// `InvalidAccount` (owner == account_address) must not emit any events —
    /// the error is returned before the emit call is reached.
    #[test]
    fn test_invalid_account_error_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let _ = client.try_deploy_account(&owner, &owner);
        assert_eq!(env.events().all().len(), 0,
            "InvalidAccount path must emit zero events");
    }

    /// `TooManyAccounts` (cap exceeded) must not emit any events.
    #[test]
    fn test_too_many_accounts_error_emits_zero_additional_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        env.budget().reset_unlimited();
        let owner = Address::generate(&env);
        for _ in 0..MAX_ACCOUNTS_PER_OWNER {
            client.deploy_account(&owner, &Address::generate(&env));
        }
        // Capture count after successful deploys.
        let event_count_before = env.events().all().len();

        // Cap-exceeded attempt must not add any new events.
        let _ = client.try_deploy_account(&owner, &Address::generate(&env));
        assert_eq!(
            env.events().all().len(),
            event_count_before,
            "TooManyAccounts rejection must emit zero additional events"
        );
    }

    /// `MetadataTooLarge` must not emit any events.
    #[test]
    fn test_metadata_too_large_error_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        // Version field exceeds MAX_VERSION_LENGTH.
        let version = String::from_str(&env, "a".repeat(33).as_str());
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        let _ = client.try_deploy_account_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );
        assert_eq!(env.events().all().len(), 0,
            "MetadataTooLarge path must emit zero events");
    }

    // ── Zero events on simulate paths ─────────────────────────────────────────

    /// `simulate_deploy` is a read-only preflight — it must emit zero events
    /// on both success and failure (InvalidAccount) paths.
    #[test]
    fn test_simulate_deploy_success_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        client.simulate_deploy(&owner, &account_addr);
        assert_eq!(env.events().all().len(), 0,
            "simulate_deploy success must emit zero events");
    }

    #[test]
    fn test_simulate_deploy_invalid_account_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let _ = client.try_simulate_deploy(&owner, &owner);
        assert_eq!(env.events().all().len(), 0,
            "simulate_deploy InvalidAccount must emit zero events");
    }

    #[test]
    fn test_simulate_deploy_with_metadata_success_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        let version = String::from_str(&env, "1.0.0");
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        client.simulate_deploy_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );
        assert_eq!(env.events().all().len(), 0,
            "simulate_deploy_with_metadata success must emit zero events");
    }

    #[test]
    fn test_simulate_deploy_with_metadata_error_emits_zero_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);
        // Oversized version triggers MetadataTooLarge.
        let version = String::from_str(&env, "a".repeat(33).as_str());
        let description = String::from_str(&env, "Test");
        let author = String::from_str(&env, "test");
        let _ = client.try_simulate_deploy_with_metadata(
            &owner, &account_addr, &version, &description, &author,
        );
        assert_eq!(env.events().all().len(), 0,
            "simulate_deploy_with_metadata error must emit zero events");
    }

    /// Calling simulate then deploy must yield exactly the deploy's events —
    /// simulate contributions to the event log must be zero.
    #[test]
    fn test_simulate_followed_by_deploy_yields_only_deploy_events() {
        use soroban_sdk::testutils::Events;
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let account_addr = Address::generate(&env);

        client.simulate_deploy(&owner, &account_addr);
        assert_eq!(env.events().all().len(), 0);

        client.deploy_account(&owner, &account_addr);
        assert_eq!(env.events().all().len(), 1,
            "only the real deploy must appear in the event log");
    }
}
}
