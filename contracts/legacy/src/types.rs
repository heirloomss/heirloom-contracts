use soroban_sdk::{contracttype, Address, Vec};

/// Lifecycle status of a legacy plan.
///
/// ```text
/// Draft ──deposit──▶ Funded ──approve(threshold)──▶ Verified ──finalize_release──▶ Released
///   │                  │                               │
///   └──────────────────┴─────────── cancel ───────────┘  (cancel forbidden once Released)
/// ```
///
/// A plan is created in `Draft`: it names its guardians and beneficiaries and
/// commits to a `token` + `total_amount`, but holds no funds yet. `deposit`
/// pulls the funds in and moves it to `Funded` (surfaced to users as
/// "Protected"). Only a `Funded` plan can gather guardian approvals, so a plan
/// can never be verified — or released — without the assets actually present.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyStatus {
    /// Plan registered with a funding commitment, but not yet deposited.
    Draft,
    /// Assets have been deposited into the contract; awaiting verification.
    Funded,
    /// Guardian approvals reached the threshold; release is authorized.
    Verified,
    /// Assets have been split into per-beneficiary claims; withdrawals open.
    Released,
    /// Plan was cancelled by the owner before release; funds refunded.
    Cancelled,
}

/// A single beneficiary allocation, expressed in basis points.
///
/// `10000` bps == 100%. The sum of all `bps` in a plan must equal `10000`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeneficiaryShare {
    /// Address entitled to claim this share.
    pub beneficiary: Address,
    /// Allocation in basis points (1/100th of a percent).
    pub bps: u32,
}

/// The full on-chain record of a legacy plan.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyPlan {
    /// Owner who created the plan and controls deposit / cancellation.
    pub owner: Address,
    /// Guardians who can approve verification when a check-in is missed.
    pub guardians: Vec<Address>,
    /// Number of guardian approvals required to move Funded -> Verified.
    pub threshold: u32,
    /// Beneficiary allocations (bps summing to 10000).
    pub beneficiaries: Vec<BeneficiaryShare>,
    /// Current lifecycle status.
    pub status: LegacyStatus,
    /// Token the plan is funded in and distributes on release. Committed at
    /// creation so the deposit and payout can never disagree on the asset.
    pub token: Address,
    /// Total amount the owner commits to deposit and split across beneficiaries.
    pub total_amount: i128,
    /// Whether the owner's `total_amount` deposit is currently held by the
    /// contract. Set on `deposit`, cleared on `cancel` refund. Drives refunds.
    pub deposited: bool,
}

/// Per-beneficiary claim record created when the plan is released.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimData {
    /// Token this claim pays out in.
    pub token: Address,
    /// Amount (in token stroops) allocated to the beneficiary.
    pub amount: i128,
    /// Whether the beneficiary has already withdrawn.
    pub claimed: bool,
}
