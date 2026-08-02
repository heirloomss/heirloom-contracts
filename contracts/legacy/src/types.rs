use soroban_sdk::{contracttype, Address, Vec};

/// Lifecycle status of a legacy plan.
///
/// ```text
/// Active ──approve(threshold)──▶ Verified ──create_claim──▶ Released
///   │                              │
///   └───────── cancel ────────────┘  (cancel forbidden once Released)
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyStatus {
    /// Plan registered and awaiting guardian verification.
    Active,
    /// Guardian approvals reached the threshold; release is authorized.
    Verified,
    /// A claim has been created; beneficiaries may withdraw their allocations.
    Released,
    /// Plan was cancelled by the owner before release.
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
    /// Owner who created the plan and controls cancellation / claim creation.
    pub owner: Address,
    /// Guardians who can approve verification when a check-in is missed.
    pub guardians: Vec<Address>,
    /// Number of guardian approvals required to move Active -> Verified.
    pub threshold: u32,
    /// Beneficiary allocations (bps summing to 10000).
    pub beneficiaries: Vec<BeneficiaryShare>,
    /// Current lifecycle status.
    pub status: LegacyStatus,
    /// Token distributed on release (set by `create_claim`).
    pub token: Option<Address>,
    /// Total amount split across beneficiaries on release.
    pub total_amount: i128,
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
