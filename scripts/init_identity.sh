#!/usr/bin/env bash
#
# One-time setup of a deployer identity for the Heirloom contracts.
#
# Creates a keypair named "heirloom-deployer" in the global Stellar CLI
# keystore and funds it on testnet via Friendbot. Safe to re-run; generating
# an identity that already exists will fail harmlessly.
#
# Usage:
#   ./scripts/init_identity.sh
#
set -euo pipefail

IDENTITY="${IDENTITY:-heirloom-deployer}"
NETWORK="${NETWORK:-testnet}"

# Generate a new keypair and immediately fund it with testnet lumens.
#   --global : store the key in the user-level keystore (usable from any dir)
#   --fund   : hit Friendbot so the account exists and can pay fees
echo "Generating identity '$IDENTITY' on $NETWORK..."
stellar keys generate --global "$IDENTITY" --network "$NETWORK" --fund

# Show the public key so it can be shared / whitelisted.
echo ""
echo "Identity ready:"
echo "  alias:   $IDENTITY"
echo "  address: $(stellar keys address "$IDENTITY")"
echo ""
echo "Use it with: SOURCE=$IDENTITY ./scripts/deploy.sh"
