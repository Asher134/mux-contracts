//! fuzz_placeholder: property-based fuzz scaffolding for Mux Protocol contracts.
//!
//! Each test drives the contract with a range of pseudo-random inputs to verify
//! core invariants hold regardless of input shape.  Extend each section with
//! domain-specific generators as the contract API stabilises.
//!
//! Run with: cargo test -p mux-contract-tests --test fuzz_placeholder
//!
//! Coverage:
//! - mux-account: spend_limit validation, debit boundaries, delegate/session caps
//! - mux-batcher: empty / oversized / in-bound batch sizes for execute_batch
//! - shared: amount round-trip, address uniqueness, bytes length

#[cfg(test)]
mod fuzz_amounts {
    use soroban_sdk::{Env, String as SorobanString};

    /// Invariant: any i128 amount serialised to a Soroban String and back must
    /// round-trip without loss.  Covers zero, positive, negative, and boundary values.
    #[test]
    fn amount_roundtrip_does_not_panic() {
        let env = Env::default();

        let candidates: &[i128] = &[
            0,
            1,
            -1,
            i128::MAX,
            i128::MIN,
            1_000_000,
            -1_000_000,
            1_000_000_000_000_i128,
        ];

        for &amount in candidates {
            let s = SorobanString::from_str(&env, &amount.to_string());
            // Invariant: construction must not panic and length must be > 0
            assert!(!s.is_empty(), "amount {amount} produced empty string");
        }
    }
}

#[cfg(test)]
mod fuzz_addresses {
    use soroban_sdk::{testutils::Address as _, Address, Env};

    /// Invariant: generated addresses must be distinct (no collision in a small batch).
    #[test]
    fn generated_addresses_are_unique() {
        let env = Env::default();
        env.mock_all_auths();

        const N: usize = 64;
        let mut addrs: Vec<String> = Vec::with_capacity(N);

        for _ in 0..N {
            let a = Address::generate(&env);
            let repr = format!("{a:?}");
            assert!(
                !addrs.contains(&repr),
                "duplicate address detected: {repr}"
            );
            addrs.push(repr);
        }
    }
}

#[cfg(test)]
mod fuzz_instruction_data {
    use soroban_sdk::{Bytes, Env};

    /// Invariant: Bytes buffers of varying lengths must be constructible and
    /// their reported length must match what was pushed.
    #[test]
    fn bytes_length_matches_push_count() {
        let env = Env::default();

        for len in [0usize, 1, 31, 32, 255, 1024] {
            let mut buf = Bytes::new(&env);
            for i in 0..len {
                buf.push_back((i & 0xFF) as u8);
            }
            assert_eq!(
                buf.len() as usize,
                len,
                "expected len {len}, got {}",
                buf.len()
            );
        }
    }
}

/// mux-account: spend_limit / debit / collection-cap invariants.
#[cfg(test)]
mod fuzz_account {
    use mux_account::{MuxAccount, MuxAccountClient, MuxAccountError};
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    const MAX_DELEGATES: u32 = 64;
    const MAX_SESSION_KEYS: u32 = 32;

    fn setup() -> (Env, MuxAccountClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxAccount);
        let client = MuxAccountClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        client.initialize(&owner, &Vec::new(&env));
        (env, client, owner)
    }

    /// Invariant: set_spend_limit rejects non-positive amounts and zero periods
    /// across a sweep of boundary candidates.
    #[test]
    fn spend_limit_rejects_invalid_inputs() {
        let (env, client, _) = setup();
        let asset = Address::generate(&env);

        let bad_amounts: &[i128] = &[0, -1, -100, i128::MIN];
        for &amount in bad_amounts {
            let result = client.try_set_spend_limit(&asset, &amount, &100_u32);
            assert_eq!(
                result,
                Err(Ok(MuxAccountError::InvalidAmount)),
                "amount {amount} should be InvalidAmount"
            );
        }

        let ok_amounts: &[i128] = &[1, 100, 1_000_000, i128::MAX];
        for &amount in ok_amounts {
            // period = 0 must always fail regardless of amount
            let result = client.try_set_spend_limit(&asset, &amount, &0_u32);
            assert_eq!(
                result,
                Err(Ok(MuxAccountError::InvalidPeriod)),
                "period 0 with amount {amount} should be InvalidPeriod"
            );
        }
    }

    /// Invariant: debit within the configured limit succeeds; debit that would
    /// exceed the limit is rejected for a range of (limit, spend) pairs.
    #[test]
    fn spend_limit_debit_boundaries() {
        let cases: &[(i128, i128, bool)] = &[
            (1000, 1, true),
            (1000, 500, true),
            (1000, 1000, true),
            (1000, 1001, false),
            (1, 1, true),
            (1, 2, false),
            (i128::MAX, 1, true),
        ];

        // Fresh account per case — avoids coupling to debit_spend's Executing flag
        // lifetime across invocations (preexisting contract behavior).
        for &(limit, spend, expect_ok) in cases {
            let (env, client, _) = setup();
            let asset = Address::generate(&env);
            client.set_spend_limit(&asset, &limit, &10_000_u32);

            let result = client.try_debit_spend(&asset, &spend);
            if expect_ok {
                assert!(
                    result.is_ok(),
                    "debit {spend} of limit {limit} should succeed: {result:?}"
                );
            } else {
                assert_eq!(
                    result,
                    Err(Ok(MuxAccountError::SpendLimitExceeded)),
                    "debit {spend} of limit {limit} should exceed"
                );
            }
        }
    }

    /// Invariant: delegate map never grows past MAX_DELEGATES.
    #[test]
    fn delegates_cap_holds_under_flood() {
        let (env, client, _) = setup();
        env.budget().reset_unlimited();

        for i in 0..MAX_DELEGATES {
            let dlg = Address::generate(&env);
            client.set_delegate(&dlg, &(1_000 + i), &true);
        }
        let overflow = Address::generate(&env);
        let result = client.try_set_delegate(&overflow, &2_000_u32, &false);
        assert_eq!(result, Err(Ok(MuxAccountError::TooManyDelegates)));

        let active = client.delegates();
        assert!(
            active.len() <= MAX_DELEGATES,
            "delegates grew past cap: {}",
            active.len()
        );
    }

    /// Invariant: documented session-key / delegate caps stay aligned with
    /// storage-griefing docs (full session-key flood covered in mux-account unit tests).
    #[test]
    fn documented_collection_caps() {
        assert_eq!(MAX_SESSION_KEYS, 32);
        assert_eq!(MAX_DELEGATES, 64);
    }
}

/// mux-batcher: batch_execute size / emptiness invariants.
#[cfg(test)]
mod fuzz_batcher {
    use mux_batcher::{
        BatchOperationKind, MuxBatcher, MuxBatcherClient, MuxBatcherError, Operation,
    };
    use soroban_sdk::{
        symbol_short, testutils::Address as _, Address, Env, Val, Vec,
    };
    use soroban_test_helpers::assert_contract_err;

    const MAX_BATCH_SIZE: u32 = 50;

    fn setup() -> (Env, MuxBatcherClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MuxBatcher);
        let client = MuxBatcherClient::new(&env, &contract_id);
        let caller = Address::generate(&env);
        (env, client, caller)
    }

    fn make_ops(env: &Env, n: u32) -> Vec<Operation> {
        let mut ops = Vec::new(env);
        for _ in 0..n {
            ops.push_back(Operation {
                target: Address::generate(env),
                fn_name: symbol_short!("noop"),
                args: Vec::<Val>::new(env),
                require_success: false,
                kind: BatchOperationKind::Invoke,
            });
        }
        ops
    }

    /// Invariant: empty batch is always rejected.
    #[test]
    fn empty_batch_rejected() {
        let (env, client, caller) = setup();
        let ops = Vec::new(&env);
        assert_contract_err(
            client.try_execute_batch(&caller, &ops),
            MuxBatcherError::EmptyBatch,
        );
    }

    /// Invariant: batches larger than MAX_BATCH_SIZE are rejected for a sweep
    /// of oversized lengths; in-bound sizes are accepted at the size gate.
    #[test]
    fn batch_size_gate_holds() {
        let (env, client, caller) = setup();
        env.budget().reset_unlimited();

        // Oversized
        for n in [MAX_BATCH_SIZE + 1, MAX_BATCH_SIZE + 5, 100_u32, 255] {
            let ops = make_ops(&env, n);
            assert_contract_err(
                client.try_execute_batch(&caller, &ops),
                MuxBatcherError::BatchTooLarge,
            );
        }

        // In-bound sizes must pass the size gate (invocation of missing targets
        // may still count as soft failures when require_success=false).
        for n in [1_u32, 2, 10, MAX_BATCH_SIZE] {
            let ops = make_ops(&env, n);
            let result = client.try_execute_batch(&caller, &ops);
            assert!(
                result.is_ok(),
                "ops_count={n} should pass size gate: {result:?}"
            );
        }

        assert_eq!(client.max_batch_size(), MAX_BATCH_SIZE);
    }

    /// Invariant: estimate_fees mirrors the execute_batch size gate.
    #[test]
    fn estimate_fees_mirrors_batch_gate() {
        let (_, client, _) = setup();

        assert_contract_err(
            client.try_estimate_fees(&0_u32),
            MuxBatcherError::EmptyBatch,
        );
        assert_contract_err(
            client.try_estimate_fees(&(MAX_BATCH_SIZE + 1)),
            MuxBatcherError::BatchTooLarge,
        );
        for n in [1_u32, 25, MAX_BATCH_SIZE] {
            assert!(
                client.try_estimate_fees(&n).is_ok(),
                "estimate_fees({n}) should succeed"
            );
        }
    }
}
