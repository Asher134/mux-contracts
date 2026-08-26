use mux_recovery::MuxRecoveryClient;
use mux_recovery::RecoveryStatus;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::vec;
use soroban_sdk::{Address, Env};

#[test]
fn test_owner_approve_recovery_executes() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    // initialize contract with owner and one guardian
    client.initialize(&owner, &vec![&env, guardian.clone()]);

    // guardian initiates recovery
    client.initiate_recovery(&guardian, &new_owner);

    // owner approves the pending recovery via admin-approved path
    assert!(client.try_approve_recovery_admin().is_ok());

    // ownership must have transferred
    let current_owner = client.owner();
    assert_eq!(current_owner, new_owner);
    // recovery status must be Executed
    assert_eq!(client.recovery_status(), RecoveryStatus::Executed);
}

// ── Unauthorized callers ─────────────────────────────────────────────────────

/// A guardian holding a perfectly valid signature over `approve_recovery_admin`
/// must not be able to substitute for the owner: the entrypoint is gated by
/// `require_owner`, which checks the *owner* address specifically — being a
/// registered guardian (and even being the guardian who initiated the
/// recovery) grants no authority over this admin-only fast path.
#[test]
fn test_approve_recovery_admin_rejects_guardian_only_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client.initialize(&owner, &vec![&env, guardian.clone()]);
    client.initiate_recovery(&guardian, &new_owner);

    // Restrict authorization to exactly the guardian's own signature over
    // the admin-approve call; the owner never authorizes anything.
    env.mock_auths(&[MockAuth {
        address: &guardian,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "approve_recovery_admin",
            args: soroban_sdk::Vec::new(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_approve_recovery_admin();
    assert!(
        result.is_err(),
        "guardian's own valid signature must not satisfy require_owner"
    );
    // Nothing changed: still pending, owner untouched.
    assert_eq!(client.recovery_status(), RecoveryStatus::Pending);
    assert_eq!(client.owner(), owner);
}

/// With no authorization present at all for the admin-approve call, the
/// timelock bypass must be rejected outright rather than silently allowed.
#[test]
fn test_approve_recovery_admin_rejects_when_no_auth_present() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client.initialize(&owner, &vec![&env, guardian.clone()]);
    client.initiate_recovery(&guardian, &new_owner);

    // Drop all authorization (this also disables mock_all_auths' recording
    // mode, per soroban-sdk's `set_auths` docs), so `owner.require_auth()`
    // has nothing valid to check against.
    env.set_auths(&[]);

    let result = client.try_approve_recovery_admin();
    assert!(
        result.is_err(),
        "approve_recovery_admin must reject when no auth is present"
    );
    assert_eq!(client.recovery_status(), RecoveryStatus::Pending);
}

// ── Interaction with the guardian-initiated flow ─────────────────────────────

/// The admin-approve fast path only ever approves an *existing*,
/// guardian-initiated request — the owner cannot fabricate one unilaterally
/// without a guardian having called `initiate_recovery` first.
#[test]
fn test_approve_recovery_admin_without_pending_recovery_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);

    client.initialize(&owner, &vec![&env, guardian]);

    let result = client.try_approve_recovery_admin();
    assert!(
        result.is_err(),
        "approve_recovery_admin must reject with no active recovery request"
    );
    assert_eq!(client.recovery_status(), RecoveryStatus::None);
}

/// Once the owner cancels a guardian-initiated recovery, the admin-approve
/// fast path must no longer apply to that (now-cancelled) request — the
/// owner cannot resurrect a cancelled request via the bypass path.
#[test]
fn test_approve_recovery_admin_rejects_after_owner_cancels() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client.initialize(&owner, &vec![&env, guardian.clone()]);
    client.initiate_recovery(&guardian, &new_owner);
    client.cancel_recovery();

    let result = client.try_approve_recovery_admin();
    assert!(
        result.is_err(),
        "approve_recovery_admin must reject a cancelled recovery request"
    );
    assert_eq!(client.recovery_status(), RecoveryStatus::Cancelled);
    assert_eq!(client.owner(), owner);
}

/// After the owner uses the admin-approve fast path, the request is
/// terminal (`Executed`): the guardian who initiated it must not
/// subsequently be able to `execute_recovery` the same request again via
/// the normal timelocked path.
#[test]
fn test_guardian_cannot_execute_recovery_after_admin_approve() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, mux_recovery::MuxRecovery);
    let client = MuxRecoveryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client.initialize(&owner, &vec![&env, guardian.clone()]);
    client.initiate_recovery(&guardian, &new_owner);

    assert!(client.try_approve_recovery_admin().is_ok());
    assert_eq!(client.recovery_status(), RecoveryStatus::Executed);

    // The guardian's own timelock-gated path must find nothing left to
    // execute; the request is terminal, not merely "not yet due".
    let result = client.try_execute_recovery(&guardian);
    assert!(
        result.is_err(),
        "execute_recovery must reject once the request was already admin-approved"
    );
    assert_eq!(client.owner(), new_owner);
    assert_eq!(client.recovery_status(), RecoveryStatus::Executed);
}
