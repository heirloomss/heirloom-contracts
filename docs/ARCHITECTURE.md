# Heirloom Contracts — Architecture

The `legacy` contract holds the trust-critical core of the Heirloom
digital-legacy platform: plan registration, guardian verification, and asset
release to beneficiaries. Everything that is not trust-critical (document
storage, identity, notifications, life check-in scheduling) stays off-chain.

## Legacy lifecycle

```text
            approve_guardian            create_claim
            (threshold met)             (owner only)
  Active ─────────────────▶ Verified ─────────────────▶ Released
    │                          │                            │
    │ cancel_legacy            │ cancel_legacy              │  claim_assets
    ▼                          ▼                            ▼  (per beneficiary)
 Cancelled                 Cancelled                   funds paid out
```

- **Active** — plan registered by the owner; guardians may approve.
- **Verified** — approvals reached the threshold; the owner may release.
- **Released** — a claim exists; each beneficiary withdraws independently.
- **Cancelled** — owner aborted the plan. Allowed from `Active` or
  `Verified`, never after release (`Error::InvalidStatus`).

## Guardian threshold model

Each plan names a set of guardian addresses and a `threshold` — a classic
"M of N" scheme, e.g. **2 of 3**: any two of the three named guardians must
call `approve_guardian` before the plan transitions to `Verified`.

Rules enforced on-chain:

- guardians must be non-empty and `1 <= threshold <= guardians.len()`
- only addresses in the plan's guardian set may approve (`NotGuardian`)
- each guardian approves at most once (`AlreadyApproved`)
- approvals are only accepted while the plan is `Active`

## How funds flow

The contract is the temporary custodian of the estate:

1. The owner (or their tooling) transfers the token amount **to the contract
   address** — the contract holds the balance.
2. `create_claim(legacy_id, token, total_amount)` splits `total_amount` across
   beneficiaries by their basis-point shares (10000 bps = 100%). Integer
   division dust is assigned to the last beneficiary so the full amount is
   always allocated. One `ClaimData` record is written per beneficiary —
   this is a **claimable-balance-style** design.
3. Each beneficiary calls `claim_assets` independently. The contract transfers
   their allocation from its own balance and flips `claimed = true`, so a
   record can never pay out twice. One beneficiary claiming (or never
   claiming) does not block the others.

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
| `approved`    | guardian        | `(legacy_id, approvals)`  | A guardian approval is recorded       |
| `verified`    | —               | `legacy_id`               | Approvals reach the threshold         |
| `claim_new`   | token           | `(legacy_id, total)`      | The owner releases the estate         |
| `claimed`     | beneficiary     | `(legacy_id, amount)`     | A beneficiary withdraws their share   |
| `cancelled`   | owner           | `legacy_id`               | The owner cancels the plan            |

## Product mapping

| Heirloom product concept | On-chain representation                                        |
| ------------------------ | -------------------------------------------------------------- |
| Guardians                | The plan's guardian set; threshold ("2 of 3") approvers         |
| Life Check-In            | Off-chain. A missed check-in triggers off-chain verification;   |
|                          | the outcome lands on-chain as guardian `approve_guardian` calls |
| Claim Packages           | `ClaimData` records — one claimable allocation per beneficiary  |
| Estate release           | `create_claim` (Verified → Released)                            |
| Beneficiary payout       | `claim_assets`, independent per beneficiary                     |
