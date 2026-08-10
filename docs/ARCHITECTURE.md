# Heirloom Contracts — Architecture

The `legacy` contract holds the trust-critical core of the Heirloom
digital-legacy platform: plan registration, guardian verification, and asset
release to beneficiaries. Everything that is not trust-critical (document
storage, identity, notifications, life check-in scheduling) stays off-chain.

## Legacy lifecycle

```text
        deposit         approve_guardian       finalize_release
      (owner only)      (threshold met)        (permissionless)
 Draft ────────▶ Funded ────────────▶ Verified ────────────────▶ Released
   │               │                     │                          │
   │ cancel        │ cancel (refund)     │ cancel (refund)          │ claim_assets
   ▼               ▼                     ▼                          ▼ (per beneficiary)
Cancelled      Cancelled             Cancelled                 funds paid out
```

- **Draft** — plan registered by the owner with a committed token + amount, but
  no funds held yet. Guardians cannot approve.
- **Funded** — the owner has `deposit`ed the committed amount into the contract;
  guardians may now approve. Surfaced to users as "Protected".
- **Verified** — approvals reached the threshold. Because the owner is presumed
  gone, release is **permissionless**.
- **Released** — a claim exists per beneficiary; each withdraws independently.
- **Cancelled** — owner aborted the plan. Allowed from `Draft`, `Funded`, or
  `Verified`, never after release (`Error::InvalidStatus`). Any deposited
  balance is refunded to the owner.

## Guardian threshold model

Each plan names a set of guardian addresses and a `threshold` — a classic
"M of N" scheme, e.g. **2 of 3**: any two of the three named guardians must
call `approve_guardian` before the plan transitions to `Verified`.

Rules enforced on-chain:

- guardians must be non-empty and `1 <= threshold <= guardians.len()`
- only addresses in the plan's guardian set may approve (`NotGuardian`)
- each guardian approves at most once (`AlreadyApproved`)
- approvals are only accepted while the plan is `Funded`

## How funds flow

The contract is the temporary custodian of the estate, and never holds a
claim it cannot cover:

1. `create_legacy` commits the plan to a `token` and `total_amount` but moves
   no funds; the plan starts in `Draft`.
2. `deposit(legacy_id)` (owner-authorized) pulls exactly `total_amount` of the
   token from the owner **into the contract** via `token.transfer(owner →
   contract)` and moves the plan to `Funded`. Approvals are gated on `Funded`,
   so a plan can never be verified while unfunded.
3. `finalize_release(legacy_id)` is **permissionless** (the owner is gone by
   design) but gated on-chain: the plan must be `Verified` **and** the
   contract's actual token balance must be `>= total_amount` (`InsufficientBalance`
   otherwise). It splits `total_amount` across beneficiaries by their
   basis-point shares (10000 bps = 100%). Integer-division dust is assigned to
   the last beneficiary so the full amount is always allocated. One `ClaimData`
   record is written per beneficiary — a **claimable-balance-style** design.
4. Each beneficiary calls `claim_assets` independently. The contract transfers
   their allocation from its own balance and flips `claimed = true`, so a
   record can never pay out twice. One beneficiary claiming (or never
   claiming) does not block the others.
5. `cancel_legacy(legacy_id)` (owner-authorized, pre-release only) refunds any
   deposited balance back to the owner via `token.transfer(contract → owner)`
   before marking the plan `Cancelled`, so cancelling never strands funds.

## Storage layout

| Key                          | Storage type | Value                | Purpose                                    |
| ---------------------------- | ------------ | -------------------- | ------------------------------------------ |
| `Counter`                    | instance     | `u64`                | Monotonic id counter for new plans         |
| `Legacy(u64)`                | persistent   | `LegacyPlan`         | Full plan record keyed by legacy id        |
| `Approvals(u64)`             | persistent   | `Vec<Address>`       | Guardians that have approved the plan      |
| `Claim(u64, Address)`        | persistent   | `ClaimData`          | Per-beneficiary allocation + claimed flag  |

Persistent entries get their TTL extended on every write (and the instance TTL
alongside), so long-running inheritance timelines are not archived out.

## Events

| Event topic   | Extra topic     | Data                      | Emitted when                          |
| ------------- | --------------- | ------------------------- | ------------------------------------- |
| `created`     | owner           | `(id, threshold)`         | A plan is registered                  |
| `deposited`   | owner           | `(legacy_id, amount)`     | The owner funds the plan              |
| `approved`    | guardian        | `(legacy_id, approvals)`  | A guardian approval is recorded       |
| `verified`    | —               | `legacy_id`               | Approvals reach the threshold         |
| `released`    | token           | `(legacy_id, total)`      | The plan is finalized for release     |
| `claimed`     | beneficiary     | `(legacy_id, amount)`     | A beneficiary withdraws their share   |
| `refunded`    | owner           | `(legacy_id, amount)`     | A funded plan is cancelled + refunded |
| `cancelled`   | owner           | `legacy_id`               | The owner cancels the plan            |

## Product mapping

| Heirloom product concept | On-chain representation                                        |
| ------------------------ | -------------------------------------------------------------- |
| Guardians                | The plan's guardian set; threshold ("2 of 3") approvers         |
| Protecting a legacy      | `create_legacy` + `deposit` (Draft → Funded)                    |
| Life Check-In            | Off-chain. A missed check-in triggers off-chain verification;   |
|                          | the outcome lands on-chain as guardian `approve_guardian` calls |
| Claim Packages           | `ClaimData` records — one claimable allocation per beneficiary  |
| Estate release           | `finalize_release`, permissionless (Verified → Released)        |
| Beneficiary payout       | `claim_assets`, independent per beneficiary                     |
