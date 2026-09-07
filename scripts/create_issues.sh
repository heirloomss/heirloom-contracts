#!/usr/bin/env bash
# Create the planned issue backlog for heirloom-contracts in one run.
# Requires: gh auth login with `repo` scope on heirloomss/heirloom-contracts.
set -euo pipefail
REPO=heirloomss/heirloom-contracts

label() { gh label create "$1" --repo "$REPO" --color "$2" --description "$3" --force >/dev/null; }
label "type: feature"       "1d76db" "New capability"
label "type: test"          "0e8a16" "Test coverage"
label "type: chore"         "fef2c0" "Tooling / CI / housekeeping"
label "type: docs"          "5319e7" "Documentation"
label "complexity: small"   "c2e0c6" "< 1 day"
label "complexity: medium"  "fbca04" "1-3 days"
label "complexity: large"   "d93f0b" "> 3 days, may touch core architecture"
label "area: contract"      "bfd4f2" "legacy contract logic"

mk() { # title  labels  body
  gh issue create --repo "$REPO" --title "$1" --label "$2" --body "$3"
}

mk "feat(legacy): per-beneficiary claim expiry and owner reclaim" \
"type: feature,complexity: large,area: contract" \
"## Summary
A released plan can leave funds parked forever if a beneficiary never claims. Add an optional expiry: after a configurable number of ledgers past release, an unclaimed portion can be reclaimed (by the owner if alive, else redistributed to the other beneficiaries — needs a design decision).

## Design options
- A: reclaim to owner only.
- B: redistribute pro-rata to beneficiaries who have claimed.
- C: send to a fallback address set at create time.

## Acceptance criteria
- [ ] \`create_legacy\` accepts an optional \`claim_expiry_ledgers\`
- [ ] New \`reclaim_unclaimed(legacy_id)\` with explicit auth rules
- [ ] Cannot reclaim before expiry; cannot reclaim an already-claimed portion
- [ ] Event \`reclaimed\` emitted
- [ ] Tests: expiry not reached, expiry reached + partial claims, double reclaim

## Tech stack
Rust, soroban-sdk 22, \`env.ledger().sequence()\`"

mk "feat(legacy): support more than one asset per plan" \
"type: feature,complexity: large,area: contract" \
"## Summary
A plan currently commits to a single \`token\` + \`total_amount\`. Real estates hold several assets. Allow N (token, amount) commitments in one plan.

## Acceptance criteria
- [ ] Storage holds a \`Vec<(Address, i128)>\` commitment set
- [ ] \`deposit\` pulls each asset; \`finalize_release\` splits each by the same bps
- [ ] \`claim_assets\` pays every asset owed to the caller in one call
- [ ] \`cancel_legacy\` refunds every deposited asset
- [ ] Tests updated for multi-asset happy path + partial funding

## Tech stack
Rust, soroban-sdk 22, SEP-41 token client"

mk "feat(legacy): edit guardian set on a Draft plan" \
"type: feature,complexity: medium,area: contract" \
"## Summary
Today changing a guardian means recreating the plan. Allow add/remove while status is \`Draft\` only.

## Acceptance criteria
- [ ] \`add_guardian\` / \`remove_guardian\` (owner auth, Draft only)
- [ ] Threshold re-validated (1..=guardians.len()); reject duplicates
- [ ] Rejected once \`Funded\`
- [ ] Events \`guardian_added\` / \`guardian_removed\`
- [ ] Tests for each rejection path

## Tech stack
Rust, soroban-sdk 22"

mk "feat(legacy): partial cancellation / top-up" \
"type: feature,complexity: medium,area: contract" \
"## Summary
Allow the owner to withdraw part of a \`Funded\` deposit, or add more, without ending the plan.

## Acceptance criteria
- [ ] \`adjust_deposit(legacy_id, delta: i128)\` — positive pulls in, negative refunds out
- [ ] Cannot reduce below 0; \`total_amount\` updated consistently
- [ ] Only \`Draft\`/\`Funded\`; owner auth
- [ ] Event \`deposit_adjusted\`
- [ ] Tests: increase, decrease, decrease-to-zero, wrong status

## Tech stack
Rust, soroban-sdk 22"

mk "test(legacy): property tests for the bps split dust invariant" \
"type: test,complexity: small,area: contract" \
"## Summary
Prove \`sum(claim amounts) == total_amount\` for arbitrary bps partitions and amounts.

## Acceptance criteria
- [ ] Randomised test over (n beneficiaries, bps summing to 10000, total_amount)
- [ ] Asserts exact reconciliation and that dust <= n-1 stroops on the last beneficiary
- [ ] Runs in CI

## Tech stack
Rust, proptest or a hand-rolled loop over fixed seeds"

mk "chore(ci): verify the built wasm hash matches the committed value" \
"type: chore,complexity: small,area: contract" \
"## Summary
Guard against accidental contract changes: CI builds with \`stellar contract build\` and compares the wasm hash to a value committed in the repo.

## Acceptance criteria
- [ ] \`EXPECTED_WASM_HASH\` file or workflow env
- [ ] CI job fails if \`stellar contract build\` output differs
- [ ] README 'Deployed contract' table stays in sync

## Tech stack
GitHub Actions, stellar-cli"

mk "docs(contracts): publish generated contract bindings" \
"type: docs,complexity: small,area: contract" \
"## Summary
Generate and commit TypeScript bindings (\`stellar contract bindings typescript\`) so the API/web don't hand-roll XDR.

## Acceptance criteria
- [ ] \`bindings/\` package generated from the deployed contract
- [ ] CI regenerates and diffs on PRs touching \`contracts/legacy/src\`
- [ ] Referenced from heirloom-docs developer guide

## Tech stack
stellar-cli, TypeScript"
