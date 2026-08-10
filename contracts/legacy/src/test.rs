#![cfg(test)]

//! Unit tests for the Heirloom Legacy contract.
//!
//! Every test runs against a fresh in-memory `Env` with all auths mocked, so
//! `require_auth` calls pass while still being recorded. Token flows use the
//! built-in Stellar Asset Contract test deployment. The lifecycle under test is
//! create -> deposit -> approve(threshold) -> finalize_release -> claim, with
//! cancel/refund available before release.

use crate::{BeneficiaryShare, Error, LegacyContract, LegacyContractClient, LegacyStatus};
use soroban_sdk::{
    testutils::Address as _, token, vec, Address, Env, Vec,
};

/// Register a fresh instance of the contract and return a client for it.
fn create_client(env: &Env) -> LegacyContractClient<'_> {
    let contract_id = env.register(LegacyContract, ());
    LegacyContractClient::new(env, &contract_id)
}

/// Deploy a Stellar Asset Contract test token and mint `amount` to `to`.
/// Returns both the token contract address and its admin (mint) client target.
fn setup_token(env: &Env, mint_to: &Address, amount: i128) -> Address {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token_address = sac.address();
    token::StellarAssetClient::new(env, &token_address).mint(mint_to, &amount);
    token_address
}

/// Everything a test needs about a freshly created 2-of-3 plan with a
/// 60/40 beneficiary split, funded in a fresh test token.
struct PlanCtx {
    id: u64,
    owner: Address,
    guardians: Vec<Address>,
    beneficiary_1: Address, // 60%
    beneficiary_2: Address, // 40%
    token: Address,
}

/// Create a standard plan: 3 guardians, threshold 2, beneficiaries 60/40,
/// funded with `total` of a fresh token minted to the owner. The plan is
/// created in `Draft` (not yet deposited).
fn create_default_plan(env: &Env, client: &LegacyContractClient, total: i128) -> PlanCtx {
    let owner = Address::generate(env);
    let g1 = Address::generate(env);
    let g2 = Address::generate(env);
    let g3 = Address::generate(env);
    let b1 = Address::generate(env);
    let b2 = Address::generate(env);

    let token = setup_token(env, &owner, total);
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

    let id = client.create_legacy(&owner, &token, &total, &guardians, &2, &beneficiaries);

    PlanCtx {
        id,
        owner,
        guardians,
        beneficiary_1: b1,
        beneficiary_2: b2,
        token,
    }
}

/// Drive a plan through deposit + 2 approvals so it becomes `Verified`.
fn fund_and_verify(client: &LegacyContractClient, ctx: &PlanCtx) {
    client.deposit(&ctx.id);
    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());
}
/// (a) Full happy path: create -> deposit (Funded) -> 2-of-3 approvals
/// (Verified) -> permissionless finalize_release splits 10_000 tokens 60/40 ->
/// both beneficiaries claim and end up with the correct balances.
#[test]
fn test_happy_path_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let total: i128 = 10_000;
    let ctx = create_default_plan(&env, &client, total);

    // Freshly created plan is Draft with no approvals and no deposit.
    let plan = client.get_legacy(&ctx.id);
    assert_eq!(plan.status, LegacyStatus::Draft);
    assert_eq!(plan.owner, ctx.owner);
    assert_eq!(plan.threshold, 2);
    assert_eq!(plan.token, ctx.token);
    assert_eq!(plan.total_amount, total);
    assert!(!plan.deposited);
    assert_eq!(client.get_approvals(&ctx.id).len(), 0);

    // Deposit moves the funds in and the plan to Funded.
    client.deposit(&ctx.id);
    let plan = client.get_legacy(&ctx.id);
    assert_eq!(plan.status, LegacyStatus::Funded);
    assert!(plan.deposited);
    let token_client = token::Client::new(&env, &ctx.token);
    assert_eq!(token_client.balance(&client.address), total);
    assert_eq!(token_client.balance(&ctx.owner), 0);

    // First approval: still Funded (1 of 2).
    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Funded);
    assert_eq!(client.get_approvals(&ctx.id).len(), 1);

    // Second approval: threshold met, plan becomes Verified.
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Verified);
    assert_eq!(client.get_approvals(&ctx.id).len(), 2);

    // Release is permissionless: no owner auth needed.
    client.finalize_release(&ctx.id);
    let plan = client.get_legacy(&ctx.id);
    assert_eq!(plan.status, LegacyStatus::Released);

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
    assert_eq!(token_client.balance(&ctx.beneficiary_1), 6_000);
    assert_eq!(token_client.balance(&ctx.beneficiary_2), 4_000);
    assert_eq!(token_client.balance(&client.address), 0);

    // Claim records are marked claimed.
    assert!(client.get_claim(&ctx.id, &ctx.beneficiary_1).claimed);
    assert!(client.get_claim(&ctx.id, &ctx.beneficiary_2).claimed);
}

/// Dust rounding: a 1/3 : 1/3 : 1/3 split of an amount not divisible by 3 must
/// still allocate the full amount, with the remainder going to the last
/// beneficiary.
#[test]
fn test_dust_goes_to_last_beneficiary() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let g1 = Address::generate(&env);
    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    let b3 = Address::generate(&env);
    let total: i128 = 100; // 3334/3333/3333 bps -> 33/33/34 (34 absorbs dust)

    let token = setup_token(&env, &owner, total);
    let guardians = vec![&env, g1.clone()];
    let beneficiaries = vec![
        &env,
        BeneficiaryShare { beneficiary: b1.clone(), bps: 3_334 },
        BeneficiaryShare { beneficiary: b2.clone(), bps: 3_333 },
        BeneficiaryShare { beneficiary: b3.clone(), bps: 3_333 },
    ];
    let id = client.create_legacy(&owner, &token, &total, &guardians, &1, &beneficiaries);

    client.deposit(&id);
    client.approve_guardian(&id, &g1);
    client.finalize_release(&id);

    // 3334 bps of 100 = 33 (33.34 floored); 3333 bps = 33; last absorbs dust.
    assert_eq!(client.get_claim(&id, &b1).amount, 33);
    assert_eq!(client.get_claim(&id, &b2).amount, 33);
    assert_eq!(client.get_claim(&id, &b3).amount, 34);
    // Full amount is allocated.
    assert_eq!(33 + 33 + 34, total);
}

/// (b) Beneficiary shares that do not sum to exactly 10_000 bps are rejected.
#[test]
fn test_invalid_shares_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let token = setup_token(&env, &owner, 1_000);
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

    let result =
        client.try_create_legacy(&owner, &token, &1_000, &guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidShares)));

    // Empty beneficiary list is also invalid.
    let empty: Vec<BeneficiaryShare> = vec![&env];
    let result = client.try_create_legacy(&owner, &token, &1_000, &guardians, &1, &empty);
    assert_eq!(result, Err(Ok(Error::InvalidShares)));
}

/// Amount must be positive.
#[test]
fn test_non_positive_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let token = setup_token(&env, &owner, 1_000);
    let guardians = vec![&env, Address::generate(&env)];
    let beneficiaries = vec![
        &env,
        BeneficiaryShare { beneficiary: Address::generate(&env), bps: 10_000 },
    ];

    let result = client.try_create_legacy(&owner, &token, &0, &guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

/// Duplicate guardian or beneficiary addresses are rejected at creation.
#[test]
fn test_duplicate_addresses_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let token = setup_token(&env, &owner, 1_000);
    let dup_guardian = Address::generate(&env);
    let b1 = Address::generate(&env);

    // Duplicate guardian.
    let guardians = vec![&env, dup_guardian.clone(), dup_guardian.clone()];
    let beneficiaries = vec![
        &env,
        BeneficiaryShare { beneficiary: b1.clone(), bps: 10_000 },
    ];
    let result =
        client.try_create_legacy(&owner, &token, &1_000, &guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::DuplicateAddress)));

    // Duplicate beneficiary.
    let guardians = vec![&env, Address::generate(&env)];
    let dup_beneficiary = Address::generate(&env);
    let beneficiaries = vec![
        &env,
        BeneficiaryShare { beneficiary: dup_beneficiary.clone(), bps: 5_000 },
        BeneficiaryShare { beneficiary: dup_beneficiary.clone(), bps: 5_000 },
    ];
    let result =
        client.try_create_legacy(&owner, &token, &1_000, &guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::DuplicateAddress)));
}

/// Approvals are only accepted once the plan is Funded.
#[test]
fn test_approve_before_deposit_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client, 10_000);

    // Plan is still Draft — approval must fail.
    let result = client.try_approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Depositing twice is rejected.
#[test]
fn test_double_deposit_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client, 10_000);

    client.deposit(&ctx.id);
    let result = client.try_deposit(&ctx.id);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// (c) An address outside the guardian set cannot approve.
#[test]
fn test_non_guardian_cannot_approve() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client, 10_000);
    client.deposit(&ctx.id);

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
    let ctx = create_default_plan(&env, &client, 10_000);
    client.deposit(&ctx.id);

    let guardian = ctx.guardians.get(0).unwrap();
    client.approve_guardian(&ctx.id, &guardian);

    let result = client.try_approve_guardian(&ctx.id, &guardian);
    assert_eq!(result, Err(Ok(Error::AlreadyApproved)));
}

/// (e) Beneficiaries cannot claim before the plan is Released, and
/// finalize_release cannot run before the plan is Verified.
#[test]
fn test_claim_and_release_before_verified_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client, 10_000);

    // Plan is Draft: claim fails.
    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // finalize_release on a Funded-but-unverified plan fails.
    client.deposit(&ctx.id);
    client.approve_guardian(&ctx.id, &ctx.guardians.get(0).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Funded);
    let result = client.try_finalize_release(&ctx.id);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Once Verified but before release, claiming still fails.
    client.approve_guardian(&ctx.id, &ctx.guardians.get(1).unwrap());
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Verified);
    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// (f) A beneficiary cannot withdraw the same allocation twice.
#[test]
fn test_double_claim_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let ctx = create_default_plan(&env, &client, 10_000);
    fund_and_verify(&client, &ctx);
    client.finalize_release(&ctx.id);

    // First claim succeeds.
    assert_eq!(client.claim_assets(&ctx.id, &ctx.beneficiary_1), 6_000);

    // Second claim by the same beneficiary is rejected.
    let result = client.try_claim_assets(&ctx.id, &ctx.beneficiary_1);
    assert_eq!(result, Err(Ok(Error::AlreadyClaimed)));

    // The other beneficiary is unaffected.
    assert_eq!(client.claim_assets(&ctx.id, &ctx.beneficiary_2), 4_000);
}

/// (g) A plan cannot be cancelled once assets have been released; cancelling a
/// funded plan refunds the owner.
#[test]
fn test_cancel_refunds_and_release_blocks_cancel() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);
    let token_client_for = |addr: &Address| token::Client::new(&env, addr);

    // Cancel after release is rejected.
    let ctx = create_default_plan(&env, &client, 10_000);
    fund_and_verify(&client, &ctx);
    client.finalize_release(&ctx.id);
    assert_eq!(client.get_legacy(&ctx.id).status, LegacyStatus::Released);
    let result = client.try_cancel_legacy(&ctx.id);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Cancel a funded (but unverified) plan: owner is fully refunded.
    let ctx2 = create_default_plan(&env, &client, 10_000);
    client.deposit(&ctx2.id);
    assert_eq!(token_client_for(&ctx2.token).balance(&ctx2.owner), 0);
    client.cancel_legacy(&ctx2.id);
    let plan = client.get_legacy(&ctx2.id);
    assert_eq!(plan.status, LegacyStatus::Cancelled);
    assert!(!plan.deposited);
    assert_eq!(token_client_for(&ctx2.token).balance(&ctx2.owner), 10_000);
    assert_eq!(token_client_for(&ctx2.token).balance(&client.address), 0);

    // Cancel a Draft plan (never deposited): no refund needed, status flips.
    let ctx3 = create_default_plan(&env, &client, 5_000);
    client.cancel_legacy(&ctx3.id);
    assert_eq!(client.get_legacy(&ctx3.id).status, LegacyStatus::Cancelled);
}

/// Extra guards: invalid guardian/threshold configurations are rejected.
#[test]
fn test_invalid_guardian_config_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let token = setup_token(&env, &owner, 1_000);
    let beneficiaries = vec![
        &env,
        BeneficiaryShare {
            beneficiary: Address::generate(&env),
            bps: 10_000,
        },
    ];

    // No guardians at all.
    let no_guardians: Vec<Address> = vec![&env];
    let result =
        client.try_create_legacy(&owner, &token, &1_000, &no_guardians, &1, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    let guardians = vec![&env, Address::generate(&env)];

    // Zero threshold.
    let result =
        client.try_create_legacy(&owner, &token, &1_000, &guardians, &0, &beneficiaries);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    // Threshold larger than the guardian set.
    let result =
        client.try_create_legacy(&owner, &token, &1_000, &guardians, &2, &beneficiaries);
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
