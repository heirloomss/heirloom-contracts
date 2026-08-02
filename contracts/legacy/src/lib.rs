#![no_std]
//! # Heirloom Legacy contract
//!
//! Trust-critical, on-chain logic for the Heirloom digital-legacy platform.
//!
//! The contract models a single flow: an **owner** registers an inheritance
//! **legacy plan** naming **guardians** and **beneficiaries**. If the owner
//! misses a life check-in, guardians approve the plan. Once a configurable
//! **threshold** of approvals is reached the plan is *Verified*, a **claim**
//! can be created that splits assets across beneficiaries, and each
//! beneficiary independently withdraws their allocation.
//!
//! Deliberately out of scope (kept off-chain by design): document storage,
//! identity/KYC, notifications, and any non-trust-critical application logic.

mod error;
mod storage;
mod types;

pub use error::Error;
pub use types::{BeneficiaryShare, ClaimData, LegacyPlan, LegacyStatus};

use soroban_sdk::{
    contract, contractimpl, symbol_short, token, Address, Env, Vec,
};

/// Basis-point denominator: 10000 bps == 100%.
const BPS_DENOMINATOR: i128 = 10_000;

#[contract]
pub struct LegacyContract;

#[contractimpl]
impl LegacyContract {
    /// Register a new legacy plan.
    ///
    /// * `owner` — the plan creator; must authorize the call.
    /// * `guardians` — non-empty set of addresses that can approve verification.
    /// * `threshold` — approvals required to verify (1..=guardians.len()).
    /// * `beneficiaries` — allocations whose `bps` must sum to exactly 10000.
    ///
    /// Returns the newly assigned legacy id.
    pub fn create_legacy(
        env: Env,
        owner: Address,
        guardians: Vec<Address>,
        threshold: u32,
        beneficiaries: Vec<BeneficiaryShare>,
    ) -> Result<u64, Error> {
        owner.require_auth();

        // --- Validate guardians / threshold ---
        if guardians.is_empty() {
            return Err(Error::InvalidInput);
        }
        if threshold == 0 || threshold > guardians.len() {
            return Err(Error::InvalidInput);
        }

        // --- Validate beneficiary shares sum to exactly 10000 bps ---
        if beneficiaries.is_empty() {
            return Err(Error::InvalidShares);
        }
        let mut total_bps: u32 = 0;
        for share in beneficiaries.iter() {
            if share.bps == 0 {
                return Err(Error::InvalidShares);
            }
            total_bps = total_bps
                .checked_add(share.bps)
                .ok_or(Error::InvalidShares)?;
        }
        if total_bps as i128 != BPS_DENOMINATOR {
            return Err(Error::InvalidShares);
        }

        let id = storage::next_legacy_id(&env);
        let plan = LegacyPlan {
            owner: owner.clone(),
            guardians,
            threshold,
            beneficiaries,
            status: LegacyStatus::Active,
            token: None,
            total_amount: 0,
        };
        storage::set_plan(&env, id, &plan);
        storage::set_approvals(&env, id, &Vec::new(&env));
        storage::extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("created"), owner),
            (id, plan.threshold),
        );

        Ok(id)
    }

    /// Record a guardian's approval for a plan.
    ///
    /// The guardian must be part of the plan's guardian set and may not approve
    /// twice. When the number of approvals reaches the threshold the plan
    /// transitions `Active -> Verified`.
    pub fn approve_guardian(
        env: Env,
        legacy_id: u64,
        guardian: Address,
    ) -> Result<(), Error> {
        guardian.require_auth();

        let mut plan = storage::get_plan(&env, legacy_id)?;
        if plan.status != LegacyStatus::Active {
            return Err(Error::InvalidStatus);
        }

        // Guardian must belong to the plan's guardian set.
        if !plan.guardians.contains(&guardian) {
            return Err(Error::NotGuardian);
        }

        // No double-approval.
        let mut approvals = storage::get_approvals(&env, legacy_id);
        if approvals.contains(&guardian) {
            return Err(Error::AlreadyApproved);
        }
        approvals.push_back(guardian.clone());
        storage::set_approvals(&env, legacy_id, &approvals);

        env.events().publish(
            (symbol_short!("approved"), guardian),
            (legacy_id, approvals.len()),
        );

        // Threshold reached -> verify.
        if approvals.len() >= plan.threshold {
            plan.status = LegacyStatus::Verified;
            storage::set_plan(&env, legacy_id, &plan);
            env.events()
                .publish((symbol_short!("verified"),), legacy_id);
        }

        storage::extend_instance_ttl(&env);
        Ok(())
    }

    /// Create the payout claim for a verified plan.
    ///
    /// Callable by the owner once the plan is `Verified`. Splits `total_amount`
    /// of `token` across beneficiaries by their basis-point shares and records
    /// a per-beneficiary [`ClaimData`]. Any integer-division dust is assigned to
    /// the final beneficiary so the full amount is always allocated. Moves the
    /// plan to `Released`.
    ///
    /// Note: this records claimable allocations; funds must be held by the
    /// contract (transferred in out-of-band) so beneficiaries can withdraw.
    pub fn create_claim(
        env: Env,
        legacy_id: u64,
        token: Address,
        total_amount: i128,
    ) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, legacy_id)?;

        // Only the owner may trigger release of the estate.
        plan.owner.require_auth();

        if plan.status != LegacyStatus::Verified {
            return Err(Error::InvalidStatus);
        }
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Allocate per-beneficiary amounts; give remainder to the last one.
        let count = plan.beneficiaries.len();
        let mut distributed: i128 = 0;
        let mut index: u32 = 0;
        for share in plan.beneficiaries.iter() {
            index += 1;
            let amount = if index == count {
                // Last beneficiary absorbs any rounding dust.
                total_amount - distributed
            } else {
                let a = total_amount
                    .checked_mul(share.bps as i128)
                    .ok_or(Error::InvalidInput)?
                    / BPS_DENOMINATOR;
                distributed += a;
                a
            };

            let claim = ClaimData {
                token: token.clone(),
                amount,
                claimed: false,
            };
            storage::set_claim(&env, legacy_id, &share.beneficiary, &claim);
        }

        plan.status = LegacyStatus::Released;
        plan.token = Some(token.clone());
        plan.total_amount = total_amount;
        storage::set_plan(&env, legacy_id, &plan);
        storage::extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("claim_new"), token),
            (legacy_id, total_amount),
        );

        Ok(())
    }

    /// Withdraw the caller's allocated portion of a released plan.
    ///
    /// The beneficiary must authorize the call. Transfers their allocation from
    /// the contract to them, marks the portion claimed, and prevents any
    /// double-claim. Beneficiaries claim independently of one another.
    pub fn claim_assets(
        env: Env,
        legacy_id: u64,
        beneficiary: Address,
    ) -> Result<i128, Error> {
        beneficiary.require_auth();

        let plan = storage::get_plan(&env, legacy_id)?;
        if plan.status != LegacyStatus::Released {
            return Err(Error::InvalidStatus);
        }

        let mut claim = storage::get_claim(&env, legacy_id, &beneficiary)?;
        if claim.claimed {
            return Err(Error::AlreadyClaimed);
        }
        if claim.amount <= 0 {
            return Err(Error::NothingToClaim);
        }

        // Transfer the tokens held by the contract to the beneficiary.
        let client = token::Client::new(&env, &claim.token);
        client.transfer(
            &env.current_contract_address(),
            &beneficiary,
            &claim.amount,
        );

        // Mark claimed only after the transfer succeeds.
        let amount = claim.amount;
        claim.claimed = true;
        storage::set_claim(&env, legacy_id, &beneficiary, &claim);
        storage::extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("claimed"), beneficiary),
            (legacy_id, amount),
        );

        Ok(amount)
    }

    /// Cancel a plan while it is still `Active` or `Verified`.
    ///
    /// Only the owner may cancel, and never once assets have been released.
    pub fn cancel_legacy(env: Env, legacy_id: u64) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, legacy_id)?;
        plan.owner.require_auth();

        match plan.status {
            LegacyStatus::Active | LegacyStatus::Verified => {
                plan.status = LegacyStatus::Cancelled;
                storage::set_plan(&env, legacy_id, &plan);
                storage::extend_instance_ttl(&env);
                env.events().publish(
                    (symbol_short!("cancelled"), plan.owner),
                    legacy_id,
                );
                Ok(())
            }
            _ => Err(Error::InvalidStatus),
        }
    }

    // --- Read-only getters ---

    /// Fetch the full plan record.
    pub fn get_legacy(env: Env, legacy_id: u64) -> Result<LegacyPlan, Error> {
        storage::get_plan(&env, legacy_id)
    }

    /// Fetch the set of guardians that have approved the plan.
    pub fn get_approvals(env: Env, legacy_id: u64) -> Vec<Address> {
        storage::get_approvals(&env, legacy_id)
    }

    /// Fetch a single beneficiary's claim record.
    pub fn get_claim(
        env: Env,
        legacy_id: u64,
        beneficiary: Address,
    ) -> Result<ClaimData, Error> {
        storage::get_claim(&env, legacy_id, &beneficiary)
    }
}

#[cfg(test)]
mod test;
