# Heirloom Contracts

Soroban smart contracts for **Heirloom**, a digital-legacy platform on
Stellar. The `legacy` contract holds the trust-critical inheritance flow: an
owner registers a legacy plan naming guardians and beneficiaries; if a life
check-in is missed, guardians verify the plan on-chain; once a configurable
threshold of approvals is reached the estate is released and each beneficiary
independently claims their allocation.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full design.

## Features

- **Legacy plans** — owner, guardian set, approval threshold, and
  basis-point beneficiary allocations (must sum to exactly 10000 bps).
- **M-of-N guardian verification** — e.g. 2-of-3 approvals move a plan from
  `Active` to `Verified`; double approvals and non-guardians are rejected.
- **Claimable-balance-style payouts** — the contract custodies the token,
  records a per-beneficiary claim, and each beneficiary withdraws
  independently with double-claim protection.
- **Owner cancellation** — allowed while `Active`/`Verified`, never after
  release.
- **Typed errors and events** — stable error codes and `created` /
  `approved` / `verified` / `claim_new` / `claimed` / `cancelled` events for
  off-chain indexing.

## Tech stack

- Rust (pinned via `rust-toolchain.toml`)
- [Soroban SDK 22](https://docs.rs/soroban-sdk/22.0.0)
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli) for building,
  deploying, and invoking

## Prerequisites

- Rust with the `wasm32-unknown-unknown` target (installed automatically via
  `rust-toolchain.toml`)
- Stellar CLI: `cargo install --locked stellar-cli`

## Build

```bash
stellar contract build
# wasm artifact: target/wasm32-unknown-unknown/release/legacy.wasm
```

## Test

```bash
cargo test
```

See [`tests/README.md`](tests/README.md) for what the suite covers.

## Deploy

```bash
# One-time: create and fund a testnet deployer identity
./scripts/init_identity.sh

# Build + deploy, prints the contract id
NETWORK=testnet SOURCE=heirloom-deployer ./scripts/deploy.sh

# Example invocations (commented templates)
./scripts/invoke_examples.sh
```

Network configuration lives in [`deployment/`](deployment/); mainnet notes in
[`deployment/README.md`](deployment/README.md).

## Environment variables

Copy `.env.example` to `.env`:

| Variable      | Description                              | Example                               |
| ------------- | ---------------------------------------- | ------------------------------------- |
| `NETWORK`     | Stellar network to target                | `testnet`                             |
| `RPC_URL`     | Soroban RPC endpoint                     | `https://soroban-testnet.stellar.org` |
| `CONTRACT_ID` | Deployed Legacy contract id (`C...`)     | *(set after deploy)*                  |

## Folder structure

```text
heirloom-contracts/
├── Cargo.toml                 # Workspace root (soroban-sdk 22, release profile)
├── rust-toolchain.toml        # Pinned Rust + wasm target
├── contracts/
│   └── legacy/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs         # Contract entrypoints
│           ├── types.rs       # LegacyPlan, BeneficiaryShare, ClaimData, status
│           ├── storage.rs     # DataKey layout + TTL management
│           ├── error.rs       # Stable error codes
│           └── test.rs        # Unit tests
├── scripts/
│   ├── init_identity.sh       # Create + fund a deployer identity
│   ├── deploy.sh              # Build + deploy, prints contract id
│   └── invoke_examples.sh     # Example `stellar contract invoke` calls
├── deployment/
│   ├── testnet.toml           # Testnet RPC + passphrase
│   └── README.md              # Mainnet notes
├── docs/
│   └── ARCHITECTURE.md        # Lifecycle, threshold model, storage, events
└── tests/
    └── README.md              # How to run tests and coverage summary
```

## License

MIT — see [LICENSE](LICENSE).
