#![no_std]
//! # Heirloom Legacy contract
//!
//! Trust-critical, on-chain logic for the Heirloom digital-legacy platform.
//!
//! The contract models a single flow: an **owner** registers an inheritance
//! **legacy plan** naming **guardians** and **beneficiaries** and committing a
//! `token` + `total_amount`. The owner **deposits** those funds into the
//! contract (status `Funded`). If the owner later misses a life check-in,
//! guardians approve the plan; once a configurable **threshold** of approvals
//! is reached the plan is *Verified*. Because the owner is presumed gone at
//! that point, **`finalize_release` is permissionless** — anyone may trigger
//! it once the plan is Verified and the deposited balance is present. It splits
//! the assets across beneficiaries, and each beneficiary independently
//! withdraws their allocation. The owner may **cancel** any time before release
//! and is **refunded** the deposited balance.
//!
//! Deliberately out of scope (kept off-chain by design): document storage,
//! identity/KYC, notifications, and any non-trust-critical application logic.

mod error;
mod storage;
mod types;

pub use error::Error;
pub use types::{BeneficiaryShare, ClaimData, LegacyPlan, LegacyStatus};

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, Vec};

/// Basis-point denominator: 10000 bps == 100%.
const BPS_DENOMINATOR: i128 = 10_000;

#[contract]
pub struct LegacyContract;

#[contractimpl]
impl LegacyContract {
    /// Register a new legacy plan (status `Draft`).
    ///
    /// * `owner` — the plan creator; must authorize the call.
    /// * `token` — the asset the plan is funded in and pays out.
    /// * `total_amount` — the amount the owner commits to deposit (> 0).
    /// * `guardians` — non-empty set of *distinct* addresses that can approve.
    /// * `threshold` — approvals required to verify (1..=guardians.len()).
    /// * `beneficiaries` — *distinct* allocations whose `bps` sum to 10000.
    ///
    /// No funds move here; the owner must call [`deposit`] next. Returns the
    /// newly assigned legacy id.
    pub fn create_legacy(
        env: Env,
        owner: Address,
        token: Address,
        total_amount: i128,
        guardians: Vec<Address>,
        threshold: u32,
        beneficiaries: Vec<BeneficiaryShare>,
    ) -> Result<u64, Error> {
        owner.require_auth();

        // --- Validate amount ---
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // --- Validate guardians / threshold (reject duplicates) ---
        if guardians.is_empty() {
            return Err(Error::InvalidInput);
        }
        if threshold == 0 || threshold > guardians.len() {
            return Err(Error::InvalidInput);
        }
        for i in 0..guardians.len() {
            let g = guardians.get(i).unwrap();
            for j in (i + 1)..guardians.len() {
                if g == guardians.get(j).unwrap() {
                    return Err(Error::DuplicateAddress);
                }
            }
        }

        // --- Validate beneficiary shares: distinct, non-zero, sum == 10000 ---
        if beneficiaries.is_empty() {
            return Err(Error::InvalidShares);
        }
        let mut total_bps: u32 = 0;
        for i in 0..beneficiaries.len() {
            let share = beneficiaries.get(i).unwrap();
            if share.bps == 0 {
                return Err(Error::InvalidShares);
            }
            for j in (i + 1)..beneficiaries.len() {
                if share.beneficiary == beneficiaries.get(j).unwrap().beneficiary {
                    return Err(Error::DuplicateAddress);
                }
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
            status: LegacyStatus::Draft,
            token,
            total_amount,
            deposited: false,
        };
        storage::set_plan(&env, id, &plan);
        storage::set_approvals(&env, id, &Vec::new(&env));
        storage::extend_instance_ttl(&env);

        env.events()
            .publish((symbol_short!("created"), owner), (id, plan.threshold));

        Ok(id)
    }
    /// Deposit the committed `total_amount` of the plan's `token` into the
    /// contract, moving the plan `Draft -> Funded`.
    ///
    /// Only the owner may deposit, and only once. Pulls the funds via
    /// `token.transfer(owner -> contract)`, so the owner must have authorized
    /// the token allowance/transfer. Guardian approvals are only accepted once
    /// a plan is `Funded`, so a plan can never be verified while unfunded.
    pub fn deposit(env: Env, legacy_id: u64) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, legacy_id)?;
        plan.owner.require_auth();

        if plan.status != LegacyStatus::Draft {
            return Err(Error::InvalidStatus);
        }
        if plan.deposited {
            return Err(Error::AlreadyFunded);
        }

        // Pull the committed amount from the owner into the contract.
        let client = token::Client::new(&env, &plan.token);
        client.transfer(
            &plan.owner,
            &env.current_contract_address(),
            &plan.total_amount,
        );

        plan.deposited = true;
        plan.status = LegacyStatus::Funded;
        storage::set_plan(&env, legacy_id, &plan);
        storage::extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("deposited"), plan.owner),
            (legacy_id, plan.total_amount),
        );

        Ok(())
    }

    /// Record a guardian's approval for a plan.
    ///
    /// The plan must be `Funded`. The guardian must be part of the plan's
    /// guardian set and may not approve twice. When the number of approvals
    /// reaches the threshold the plan transitions `Funded -> Verified`.
    pub fn approve_guardian(env: Env, legacy_id: u64, guardian: Address) -> Result<(), Error> {
        guardian.require_auth();

        let mut plan = storage::get_plan(&env, legacy_id)?;
        if plan.status != LegacyStatus::Funded {
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
    /// Finalize the release of a verified plan — **permissionless**.
    ///
    /// By the time a plan is `Verified`, the owner is presumed gone (the
    /// guardians only approve after a missed check-in), so this call requires
    /// **no authorization**: any party may trigger it. It is gated instead by
    /// on-chain state — the plan must be `Verified` and the contract's actual
    /// `token` balance must cover `total_amount` (closing the underfunded
    /// loophole). Splits `total_amount` across beneficiaries by their
    /// basis-point shares, assigns integer-division dust to the final
    /// beneficiary, records a per-beneficiary [`ClaimData`], and moves the plan
    /// to `Released`.
    pub fn finalize_release(env: Env, legacy_id: u64) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, legacy_id)?;

        if plan.status != LegacyStatus::Verified {
            return Err(Error::InvalidStatus);
        }
        if !plan.deposited {
            return Err(Error::NotFunded);
        }

        // The funds must actually be present before we open claims. This closes
        // the underfunded-release loophole even if a token misbehaves.
        let token_client = token::Client::new(&env, &plan.token);
        let balance = token_client.balance(&env.current_contract_address());
        if balance < plan.total_amount {
            return Err(Error::InsufficientBalance);
        }

        // Allocate per-beneficiary amounts; give remainder to the last one.
        let total_amount = plan.total_amount;
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
                token: plan.token.clone(),
                amount,
                claimed: false,
            };
            storage::set_claim(&env, legacy_id, &share.beneficiary, &claim);
        }

        plan.status = LegacyStatus::Released;
        storage::set_plan(&env, legacy_id, &plan);
        storage::extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("released"), plan.token),
            (legacy_id, total_amount),
        );

        Ok(())
    }

    /// Withdraw the caller's allocated portion of a released plan.
    ///
    /// The beneficiary must authorize the call. Transfers their allocation from
    /// the contract to them, marks the portion claimed, and prevents any
    /// double-claim. Beneficiaries claim independently of one another.
    pub fn claim_assets(env: Env, legacy_id: u64, beneficiary: Address) -> Result<i128, Error> {
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
        client.transfer(&env.current_contract_address(), &beneficiary, &claim.amount);

        // Mark claimed only after the transfer succeeds.
        let amount = claim.amount;
        claim.claimed = true;
        storage::set_claim(&env, legacy_id, &beneficiary, &claim);
        storage::extend_instance_ttl(&env);

        env.events()
            .publish((symbol_short!("claimed"), beneficiary), (legacy_id, amount));

        Ok(amount)
    }

    /// Cancel a plan while it is still pre-release, refunding any deposit.
    ///
    /// Only the owner may cancel, and never once assets have been released. If
    /// the plan was funded, the full deposited `total_amount` is transferred
    /// back to the owner before the status moves to `Cancelled` — a cancelled
    /// plan never strands funds in the contract.
    pub fn cancel_legacy(env: Env, legacy_id: u64) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, legacy_id)?;
        plan.owner.require_auth();

        match plan.status {
            LegacyStatus::Draft | LegacyStatus::Funded | LegacyStatus::Verified => {
                // Refund the deposit, if any, before cancelling.
                if plan.deposited {
                    let client = token::Client::new(&env, &plan.token);
                    client.transfer(
                        &env.current_contract_address(),
                        &plan.owner,
                        &plan.total_amount,
                    );
                    plan.deposited = false;
                    env.events().publish(
                        (symbol_short!("refunded"), plan.owner.clone()),
                        (legacy_id, plan.total_amount),
                    );
                }

                plan.status = LegacyStatus::Cancelled;
                storage::set_plan(&env, legacy_id, &plan);
                storage::extend_instance_ttl(&env);
                env.events()
                    .publish((symbol_short!("cancelled"), plan.owner), legacy_id);
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
    pub fn get_claim(env: Env, legacy_id: u64, beneficiary: Address) -> Result<ClaimData, Error> {
        storage::get_claim(&env, legacy_id, &beneficiary)
    }
}

#[cfg(test)]
mod test;
