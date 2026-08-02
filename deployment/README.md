# Deployment

Per-network configuration for the Heirloom contracts.

## Testnet

See [`testnet.toml`](./testnet.toml). Testnet is reset periodically by the
Stellar Development Foundation — expect deployed contracts and funded accounts
to disappear; redeploy with `scripts/deploy.sh` after a reset.

## Mainnet notes

- Network passphrase: `Public Global Stellar Network ; September 2015`
- Use a production Soroban RPC endpoint you trust (self-hosted or a paid
  provider); the public testnet RPC has no mainnet equivalent with the same
  free guarantees.
- Deployments are irreversible and fees are paid in real XLM. Before going to
  mainnet:
  - Pin the exact toolchain (`rust-toolchain.toml`) and reproduce the wasm
    build from a clean checkout.
  - Run the full test suite (`cargo test`) and, ideally, an external audit —
    this contract custodies user funds during the claim window.
  - Deploy with a hardware-backed or multisig-controlled key, never a
    development identity created with `--fund`.
  - Record the deployed contract id and wasm hash in a mainnet config file
    alongside this one.
