use soroban_sdk::contracterror;

/// Errors returned by the Legacy contract.
///
/// Each variant maps to a stable `u32` code so off-chain clients can match on
/// specific failure modes. Codes are intentionally never reordered or reused.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The requested legacy plan (or a sub-record) does not exist.
    NotFound = 1,
    /// Caller is not permitted to perform this action.
    NotAuthorized = 2,
    /// A guardian tried to approve a plan they have already approved.
    AlreadyApproved = 3,
    /// The caller is not part of the plan's guardian set.
    NotGuardian = 4,
    /// The number of guardian approvals has not yet reached the threshold.
    ThresholdNotMet = 5,
    /// Beneficiary shares are invalid (empty, or do not sum to 10000 bps).
    InvalidShares = 6,
    /// A beneficiary tried to claim a portion that was already claimed.
    AlreadyClaimed = 7,
    /// The plan is not in the correct status for the requested action.
    InvalidStatus = 8,
    /// There is nothing (a zero or missing allocation) for the caller to claim.
    NothingToClaim = 9,
    /// Provided configuration values (threshold, amount, etc.) are invalid.
    InvalidInput = 10,
}
