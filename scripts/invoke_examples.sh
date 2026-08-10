#!/usr/bin/env bash
#
# Example `stellar contract invoke` calls against a deployed Legacy contract.
#
# These are templates — replace the placeholder G.../C... addresses with real
# ones before running. Set CONTRACT_ID (see .env) and make sure the source
# identity exists (scripts/init_identity.sh).
#
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-heirloom-deployer}"
CONTRACT_ID="${CONTRACT_ID:?set CONTRACT_ID to the deployed contract id}"

# ---------------------------------------------------------------------------
# 1. create_legacy — register a plan committing to CTOKEN + 10000 units, with
#    3 guardians, 2-of-3 threshold, and two beneficiaries splitting the estate
#    60% / 40% (bps sum to 10000). No funds move yet; plan starts in Draft.
#    Returns the new legacy id (u64).
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- create_legacy \
#   --owner GOWNER... \
#   --token CTOKEN... \
#   --total_amount 10000 \
#   --guardians '["GGUARDIAN1...","GGUARDIAN2...","GGUARDIAN3..."]' \
#   --threshold 2 \
#   --beneficiaries '[{"beneficiary":"GBENEF1...","bps":6000},{"beneficiary":"GBENEF2...","bps":4000}]'

# ---------------------------------------------------------------------------
# 2. deposit — the owner funds plan #1, moving total_amount of the token into
#    the contract. Draft -> Funded. --source must be (or sign for) the owner.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- deposit \
#   --legacy_id 1

# ---------------------------------------------------------------------------
# 3. approve_guardian — a guardian records their approval for plan #1.
#    Only allowed once the plan is Funded. The --source must be (or sign for)
#    the guardian address. When approvals reach the threshold the plan becomes
#    Verified.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source guardian-1 --network "$NETWORK" \
#   -- approve_guardian \
#   --legacy_id 1 \
#   --guardian GGUARDIAN1...

# ---------------------------------------------------------------------------
# 4. finalize_release — split the deposited estate across the beneficiaries of
#    plan #1. Permissionless (anyone may call once Verified); gated on-chain by
#    a balance check. Verified -> Released. --source can be any funded account.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- finalize_release \
#   --legacy_id 1

# ---------------------------------------------------------------------------
# 5. claim_assets — a beneficiary withdraws their allocation from plan #1.
#    The --source must be (or sign for) the beneficiary address.
#    Returns the amount transferred.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source beneficiary-1 --network "$NETWORK" \
#   -- claim_assets \
#   --legacy_id 1 \
#   --beneficiary GBENEF1...

# ---------------------------------------------------------------------------
# 6. cancel_legacy — the owner aborts plan #1 before release. Refunds any
#    deposited balance to the owner. --source must be (or sign for) the owner.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- cancel_legacy \
#   --legacy_id 1

# ---------------------------------------------------------------------------
# 7. get_legacy — read-only fetch of the full plan record (status, guardians,
#    beneficiaries, token, total_amount, deposited).
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- get_legacy \
#   --legacy_id 1

echo "This script is a set of commented examples — open it and copy the call you need."
