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

- Full happy path: create plan → 2-of-3 guardian approvals → `Verified` →
  `create_claim` splitting 10 000 tokens 60/40 → both beneficiaries claim,
  token balances verified, contract drained.
- Beneficiary shares not summing to 10000 bps (and empty lists) →
  `InvalidShares`.
- Non-guardian approval → `NotGuardian`.
- Double approval by the same guardian → `AlreadyApproved`.
- Claiming before release (Active and Verified), and releasing an unverified
  plan → `InvalidStatus`.
- Double claim by the same beneficiary → `AlreadyClaimed` (other
  beneficiaries unaffected).
- Cancelling after release → `InvalidStatus`; cancelling while Active works.
- Invalid guardian/threshold configurations → `InvalidInput`.
- Unknown plan / claim ids → `NotFound`.
