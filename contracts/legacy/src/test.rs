#![cfg(test)]

//! Unit tests for the Heirloom Legacy contract.
//!
//! Every test runs against a fresh in-memory `Env` with all auths mocked, so
//! `require_auth` calls pass while still being recorded. Token flows use the
//! built-in Stellar Asset Contract test deployment.

use crate::{BeneficiaryShare, Error, LegacyContract, LegacyContractClient, LegacyStatus};
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, Vec};

/// Register a fresh instance of the contract and return a client for it.
fn create_client(env: &Env) -> LegacyContractClient<'_> {
    let contract_id = env.register(LegacyContract, ());
    LegacyContractClient::new(env, &contract_id)
}

/// Everything a test needs about a freshly created 2-of-3 plan with a
/// 60/40 beneficiary split.
struct PlanCtx {
    id: u64,
    owner: Address,
    guardians: Vec<Address>,
    beneficiary_1: Address, // 60%
    beneficiary_2: Address, // 40%
}

/// Create a standard plan: 3 guardians, threshold 2, beneficiaries 60/40.
fn create_default_plan(env: &Env, client: &LegacyContractClient) -> PlanCtx {
    let owner = Address::generate(env);
    let g1 = Address::generate(env);
    let g2 = Address::generate(env);
    let g3 = Address::generate(env);
    let b1 = Address::generate(env);
    let b2 = Address::generate(env);

    let guardians = vec![env, g1, g2, g3];
    let beneficiaries = vec![
        env,
        BeneficiaryShare {
            beneficiary: b1.clone(),
            bps: 6_000,
        },
        BeneficiaryShare {
            beneficiary: b2.clone(),
            bps: 4_000,
        },
    ];

    let id = client.create_legacy(&owner, &guardians, &2, &beneficiaries);

    PlanCtx {
        id,
        owner,
        guardians,
        beneficiary_1: b1,
        beneficiary_2: b2,
    }
}

/// Deploy a Stellar Asset Contract test token and mint `amount` to `to`.
/// Returns the token contract address.
fn setup_token(env: &Env, to: &Address, amount: i128) -> Address {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token_address = sac.address();
    token::StellarAssetClient::new(env, &token_address).mint(to, &amount);
    token_address
}

/// (a) Full happy path: create -> 2-of-3 approvals -> Verified ->
/// create_claim splits 10_000 tokens 60/40 -> both beneficiaries claim and
/// end up with the correct balances.
#[test]
fn test_happy_path_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    // Freshly created plan is Active with no approvals.
    let plan = client.get_legacy(&ctx.id);
    assert_eq!(plan.status, LegacyStatus::Active);
    assert_eq!(plan.owner, ctx.owner);
    assert_eq!(plan.threshold, 2);
    assert_eq!(client.get_approvals(&ctx.id).len(), 0);

    // First approval: still Active (1 of 2).
    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Active);
    assert_eq!(client.get_approvals(&ctx.id).len(), 1);

    // Second approval: threshold met, plan becomes Verified.
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Verified);
    assert_eq!(client.get_approvals(&ctx.id).len(), 2);

    // Fund the contract with 10_000 test tokens, then release.
    let total: i128 = 10_000;
    let token_address = setup_token(&env, &client.address, total);
    client.create_claim(&ctx.id, &token_address, &total);

    let plan = client.get_legacy(&ctx.id);
    assert_eq!(plan.status, LegacyStatus::Released);
    assert_eq!(plan.token, Some(token_address.clone()));
    assert_eq!(plan.total_amount, total);

    // Per-beneficiary claim records: 60% / 40%.
    let claim_1 = client.get_claim(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(claim_1.amount, 6_000);
    assert!(!claim_1.claimed);
    let claim_2 = client.get_claim(&ctx.id, &ctx.beneficiary_2);
    assert_eq!(claim_2.amount, 4_000);
    assert!(!claim_2.claimed);

    // Both beneficiaries claim independently.
    let paid_1 = client.claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(paid_1, 6_000);
    let paid_2 = client.claim_assets(&ctx.id, &ctx.beneficiary_2);
    assert_eq!(paid_2, 4_000);

    // Token balances line up and the contract is fully drained.
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&ctx.beneficiary_1), 6_000);
    assert_eq!(token_client.balance(&ctx.beneficiary_2), 4_000);
    assert_eq!(token_client.balance(&client.address), 0);

    // Claim records are marked claimed.
    assert!(client.get_claim(&ctx.id, &ctx.beneficiary_1).claimed);
    assert!(client.get_claim(&ctx.id, &ctx.beneficiary_2).claimed);
}

/// (b) Beneficiary shares that do not sum to exactly 10_000 bps are rejected.
#[test]
fn test_invalid_shares_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let guardian = Address::generate(&env);
    let guardians = vec![&env, guardian];

    // Sums to 9_000, not 10_000.
    let beneficiaries = vec![
        &env,
        BeneficiaryShare {
            beneficiary: Address::generate(&env),
            bps: 5_000,
        },
        BeneficiaryShare {
            beneficiary: Address::generate(&env),
            bps: 4_000,
        },
    ];

    let result = client.try_create_legacy(&owner, &guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidShares)));

    // Empty beneficiary list is also invalid.
    let empty: Vec<BeneficiaryShare> = vec![&env];
    let result = client.try_create_legacy(&owner, &guardians, &1, &empty);
    assert_eq!(result, Err(Ok(Error::InvalidShares)));
}

/// (c) An address outside the guardian set cannot approve.
#[test]
fn test_non_guardian_cannot_approve() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    let stranger = Address::generate(&env);
    let result = client.try_approve_guardian(&ctx.id, &stranger);
    assert_eq!(result, Err(Ok(Error::NotGuardian)));
}

/// (d) A guardian cannot approve the same plan twice.
#[test]
fn test_double_approve_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    let guardian = ctx.guardians.get(0).unwrap();
    client.approve_guardian(&ctx.id, &guardian);

    let result = client.try_approve_guardian(&ctx.id, &guardian);
    assert_eq!(result, Err(Ok(Error::AlreadyApproved)));
}

/// (e) Beneficiaries cannot claim before the plan is Released.
#[test]
fn test_claim_before_release_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    // Plan is still Active.
    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Even when Verified (but not yet Released), claiming must fail.
    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Verified);

    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // create_claim is also gated: it requires Verified, so it must fail on a
    // brand-new (Active) plan.
    let ctx2 = create_default_plan(&env, &client);
    let token_address = setup_token(&env, &client.address, 1_000);
    let result = client.try_create_claim(&ctx2.id, &token_address, &1_000);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// (f) A beneficiary cannot withdraw the same allocation twice.
#[test]
fn test_double_claim_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());

    let total: i128 = 10_000;
    let token_address = setup_token(&env, &client.address, total);
    client.create_claim(&ctx.id, &token_address, &total);

    // First claim succeeds.
    assert_eq!(client.claim_assets(&ctx.id, &ctx.beneficiary_1), 6_000);

    // Second claim by the same beneficiary is rejected.
    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::AlreadyClaimed)));

    // The other beneficiary is unaffected.
    assert_eq!(client.claim_assets(&ctx.id, &ctx.beneficiary_2), 4_000);
}

/// (g) A plan cannot be cancelled once assets have been released.
#[test]
fn test_cancel_after_release_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client);

    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());

    let total: i128 = 10_000;
    let token_address = setup_token(&env, &client.address, total);
    client.create_claim(&ctx.id, &token_address, &total);
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Released);

    let result = client.try_cancel_legacy(&ctx.id);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Sanity check: cancelling while still Active works.
    let ctx2 = create_default_plan(&env, &client);
    client.cancel_legacy(&ctx2.id);
    assert_eq!(client.get_legacy(&ctx2.id).status, LegacyStatus::Cancelled);
}

/// Extra guards: invalid guardian/threshold configurations are rejected.
#[test]
fn test_invalid_guardian_config_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let beneficiaries = vec![
        &env,
        BeneficiaryShare {
            beneficiary: Address::generate(&env),
            bps: 10_000,
        },
    ];

    // No guardians at all.
    let no_guardians: Vec<Address> = vec![&env];
    let result = client.try_create_legacy(&owner, &no_guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    let guardians = vec![&env, Address::generate(&env)];

    // Zero threshold.
    let result = client.try_create_legacy(&owner, &guardians, &0, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    // Threshold larger than the guardian set.
    let result = client.try_create_legacy(&owner, &guardians, &2, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

/// Unknown ids surface NotFound rather than panicking.
#[test]
fn test_missing_plan_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let result = client.try_get_legacy(&999);
    assert_eq!(result, Err(Ok(Error::NotFound)));

    let stranger = Address::generate(&env);
    let result = client.try_get_claim(&999, &stranger);
    assert_eq!(result, Err(Ok(Error::NotFound)));
}
