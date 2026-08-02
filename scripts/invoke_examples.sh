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
# 1. create_legacy — register a plan with 3 guardians, 2-of-3 threshold, and
#    two beneficiaries splitting the estate 60% / 40% (bps sum to 10000).
#    Returns the new legacy id (u64).
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- create_legacy \
#   --owner GOWNER... \
#   --guardians '["GGUARDIAN1...","GGUARDIAN2...","GGUARDIAN3..."]' \
#   --threshold 2 \
#   --beneficiaries '[{"beneficiary":"GBENEF1...","bps":6000},{"beneficiary":"GBENEF2...","bps":4000}]'

# ---------------------------------------------------------------------------
# 2. approve_guardian — a guardian records their approval for plan #1.
#    The --source must be (or sign for) the guardian address.
#    When approvals reach the threshold the plan becomes Verified.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source guardian-1 --network "$NETWORK" \
#   -- approve_guardian \
#   --legacy_id 1 \
#   --guardian GGUARDIAN1...

# ---------------------------------------------------------------------------
# 3. create_claim — owner releases 10000 units of a token across the
#    beneficiaries of plan #1. The contract must already hold the tokens
#    (transfer them to the contract address beforehand).
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- create_claim \
#   --legacy_id 1 \
#   --token CTOKEN... \
#   --total_amount 10000

# ---------------------------------------------------------------------------
# 4. claim_assets — a beneficiary withdraws their allocation from plan #1.
#    The --source must be (or sign for) the beneficiary address.
#    Returns the amount transferred.
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source beneficiary-1 --network "$NETWORK" \
#   -- claim_assets \
#   --legacy_id 1 \
#   --beneficiary GBENEF1...

# ---------------------------------------------------------------------------
# 5. get_legacy — read-only fetch of the full plan record (status, guardians,
#    beneficiaries, token, total_amount).
# ---------------------------------------------------------------------------
# stellar contract invoke \
#   --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
#   -- get_legacy \
#   --legacy_id 1

echo "This script is a set of commented examples — open it and copy the call you need."
