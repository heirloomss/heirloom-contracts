#!/usr/bin/env bash
#
# Build and deploy the Heirloom Legacy contract.
#
# Usage:
#   NETWORK=testnet SOURCE=heirloom-deployer ./scripts/deploy.sh
#
# Prerequisites:
#   - Stellar CLI installed (https://developers.stellar.org/docs/tools/cli)
#   - An identity created and funded (see scripts/init_identity.sh)
#
set -euo pipefail

# Network to deploy to (testnet by default). Matches an entry in the
# Stellar CLI network config or a well-known network name.
NETWORK="${NETWORK:-testnet}"

# Identity (key alias) that signs and pays for the deployment.
SOURCE="${SOURCE:-heirloom-deployer}"

# 1. Compile the workspace contracts to wasm. Modern Stellar CLI builds
#    Soroban contracts for the wasm32v1-none target.
echo "Building contracts..."
stellar contract build

WASM="target/wasm32v1-none/release/legacy.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "error: expected wasm artifact not found at $WASM" >&2
  exit 1
fi

# 2. Deploy the wasm to the target network. The CLI uploads the code and
#    instantiates the contract, printing its contract id (C...).
echo "Deploying $WASM to $NETWORK as $SOURCE..."
CONTRACT_ID="$(stellar contract deploy \
  --wasm "$WASM" \
  --source "$SOURCE" \
  --network "$NETWORK")"

# 3. Print the contract id. Copy this into your .env as HEIRLOOM_CONTRACT_ID.
echo ""
echo "Deployed Legacy contract:"
echo "  network:     $NETWORK"
echo "  contract id: $CONTRACT_ID"
echo ""
echo "Next: set HEIRLOOM_CONTRACT_ID=$CONTRACT_ID in heirloom-api/.env"
