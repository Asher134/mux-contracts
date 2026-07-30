use mux_recovery::MuxRecoveryClient;
use mux_recovery::RecoveryStatus;
use soroban_sdk::vec;
use soroban_sdk::{Env, Address};

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
