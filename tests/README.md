# Tests

Unit tests live next to the contract source at
`contracts/legacy/src/test.rs` and run natively (no wasm build needed).

## Running

```bash
# From the repository root — runs every workspace member's tests
cargo test

# Just the legacy contract, with output
cargo test -p legacy -- --nocapture
```

The tests use the Soroban SDK `testutils` feature (enabled in
dev-dependencies): a fresh in-memory `Env` per test, `mock_all_auths()` for
authorization, and a Stellar Asset Contract test token for transfers.

## Coverage

- Full happy path: create plan → `deposit` (Draft → Funded) → 2-of-3 guardian
  approvals → `Verified` → permissionless `finalize_release` splitting 10 000
  tokens 60/40 → both beneficiaries claim, token balances verified, contract
  drained.
- Dust rounding: a 33.34/33.33/33.33% split of an indivisible amount allocates
  the full total, with the remainder going to the last beneficiary.
- Beneficiary shares not summing to 10000 bps (and empty lists) →
  `InvalidShares`; non-positive amount → `InvalidInput`.
- Duplicate guardian or beneficiary addresses → `DuplicateAddress`.
- Approving before `deposit`, and depositing twice → `InvalidStatus`.
- Non-guardian approval → `NotGuardian`.
- Double approval by the same guardian → `AlreadyApproved`.
- Claiming before release (Draft, Funded, and Verified), and releasing an
  unverified plan → `InvalidStatus`.
- Double claim by the same beneficiary → `AlreadyClaimed` (other
  beneficiaries unaffected).
- Cancelling after release → `InvalidStatus`; cancelling a funded plan refunds
  the owner in full; cancelling a Draft plan flips status with no refund.
- Invalid guardian/threshold configurations → `InvalidInput`.
- Unknown plan / claim ids → `NotFound`.
