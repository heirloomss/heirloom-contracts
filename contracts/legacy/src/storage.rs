use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::error::Error;
use crate::types::{ClaimData, LegacyPlan};

/// Keys for all contract storage entries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Monotonic counter for the next legacy id. Stored in instance storage.
    Counter,
    /// A legacy plan keyed by id. Stored in persistent storage.
    Legacy(u64),
    /// The set of guardian addresses that have approved a plan.
    Approvals(u64),
    /// A per-beneficiary claim record: (legacy_id, beneficiary) -> ClaimData.
    Claim(u64, Address),
}

// Persistent entries live longer than a single transaction and are the natural
// home for plans/approvals/claims. We bump their TTL on write so active plans
// are not archived out from under long-running inheritance timelines.
const PERSISTENT_BUMP_AMOUNT: u32 = 3_456_000; // ~ >6 months of ledgers
const PERSISTENT_LIFETIME_THRESHOLD: u32 = 1_728_000; // extend when < ~3 months

/// Read the next legacy id and increment the stored counter.
pub fn next_legacy_id(env: &Env) -> u64 {
    let key = DataKey::Counter;
    let current: u64 = env.storage().instance().get(&key).unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&key, &next);
    next
}

/// Store (create or overwrite) a legacy plan and refresh its TTL.
pub fn set_plan(env: &Env, id: u64, plan: &LegacyPlan) {
    let key = DataKey::Legacy(id);
    env.storage().persistent().set(&key, plan);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Load a legacy plan or return `Error::NotFound`.
pub fn get_plan(env: &Env, id: u64) -> Result<LegacyPlan, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Legacy(id))
        .ok_or(Error::NotFound)
}

/// Store the approvals set for a plan and refresh its TTL.
pub fn set_approvals(env: &Env, id: u64, approvals: &Vec<Address>) {
    let key = DataKey::Approvals(id);
    env.storage().persistent().set(&key, approvals);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Load the approvals set for a plan, defaulting to an empty vec.
pub fn get_approvals(env: &Env, id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Approvals(id))
        .unwrap_or_else(|| Vec::new(env))
}

/// Store a per-beneficiary claim record and refresh its TTL.
pub fn set_claim(env: &Env, id: u64, beneficiary: &Address, claim: &ClaimData) {
    let key = DataKey::Claim(id, beneficiary.clone());
    env.storage().persistent().set(&key, claim);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Load a per-beneficiary claim record or return `Error::NotFound`.
pub fn get_claim(env: &Env, id: u64, beneficiary: &Address) -> Result<ClaimData, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Claim(id, beneficiary.clone()))
        .ok_or(Error::NotFound)
}

/// Extend the TTL of the contract instance so config/counter survive.
pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}
