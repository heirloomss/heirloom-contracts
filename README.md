<!-- Add a banner image here: upload to a GitHub issue/comment and paste the
     user-attachments URL, matching the approved-repo convention. -->
<p align="center">
  <strong>Heirloom Contracts</strong><br />
  Soroban smart contracts for a digital-legacy platform on Stellar.
</p>

<p align="center">
  <a href="https://github.com/heirloomss/heirloom-contracts/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/heirloomss/heirloom-contracts/actions/workflows/ci.yml/badge.svg" /></a>
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.88.0-orange.svg" />
  <img alt="Soroban SDK" src="https://img.shields.io/badge/soroban--sdk-22-blue.svg" />
  <img alt="Network" src="https://img.shields.io/badge/network-testnet-purple.svg" />
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-green.svg" /></a>
</p>

<p align="center">
  <strong>🌐 Live app: <a href="https://heirloome.vercel.app">heirloome.vercel.app</a></strong>
</p>

<p align="center">
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="https://github.com/heirloomss/heirloom-api">API repo</a> ·
  <a href="https://github.com/heirloomss/heirloom-web">Web repo</a> ·
  <a href="#deployed-contract">Deployed contract</a>
</p>

---

## What this is

The `legacy` contract holds the trust-critical part of **Heirloom**: the
inheritance flow that must be enforced by code rather than by a company.

An owner registers a **legacy plan** — a guardian set, an approval threshold, a
committed token and amount, and basis-point allocations for each beneficiary —
then **deposits** the committed funds into the contract. While the owner keeps
doing their periodic Life Check-In, nothing moves. If check-ins stop, the named
guardians **verify** the plan on-chain. Once a configurable **M-of-N** threshold
of approvals is reached, **release is permissionless** — the owner is presumed
gone, so anyone can finalize it — and each beneficiary **claims** their share
independently. The owner can **cancel and be refunded** at any point before
release.

Everything that does not need to be trustless — accounts, the encrypted archive,
messages, reminders, email — lives in [`heirloom-api`](https://github.com/heirloomss/heirloom-api).

## Maintainers · [Telegram](https://t.me/cjay)

<table align="center">
  <tr>
    <td align="center">
      <img src="https://github.com/Cjay-Cyber-2.png" width="120" alt="Cjay" /><br /><br />
      <strong>Cjay — Maintainer</strong><br /><br />
      <a href="https://github.com/Cjay-Cyber-2">Cjay-Cyber-2</a><br />
      <a href="https://t.me/cjay">Telegram</a><br />
      <a href="mailto:chijiokejoseph2022@gmail.com">Email</a>
    </td>
  </tr>
</table>

## Contents

- [Features](#features)
- [State machine](#state-machine)
- [Tech stack](#tech-stack)
- [Quick start](#quick-start)
- [Deploy](#deploy)
- [Deployed contract](#deployed-contract)
- [Environment variables](#environment-variables)
- [Repository layout](#repository-layout)
- [Contributing](#contributing)
- [Security](#security)
- [Contributors](#contributors)
- [License](#license)

## Features

- **Legacy plans** — owner, guardian set, approval threshold, committed token +
  amount, and basis-point beneficiary allocations that must sum to exactly
  `10_000` bps. Duplicate guardian/beneficiary addresses are rejected.
- **Funding model** — the owner `deposit`s the committed amount (`Draft` →
  `Funded`); approvals are only accepted once funded, so a plan can never be
  verified while unfunded.
- **M-of-N guardian verification** — e.g. 2-of-3 approvals move a plan from
  `Funded` to `Verified`. Double approvals and non-guardians are rejected.
- **Permissionless release** — once `Verified`, anyone may call
  `finalize_release`; it is gated by an on-chain balance check so an underfunded
  plan cannot release.
- **Claimable-balance-style payouts** — the contract custodies the token,
  records a per-beneficiary claim, and each beneficiary withdraws independently
  with double-claim protection.
- **Owner cancellation with refund** — allowed while `Draft` / `Funded` /
  `Verified`, never after release; any deposited balance is refunded to the
  owner.
- **Typed errors and events** — stable error codes and `created` / `deposited` /
  `approved` / `verified` / `released` / `claimed` / `refunded` / `cancelled`
  events for off-chain indexing.

## State machine

```
Draft ──deposit──▶ Funded ──approve_guardian × threshold──▶ Verified ──finalize_release──▶ Released
  │                  │                                         │
  └──── cancel ──────┴──────────────── cancel ─────────────────┘         claim_assets × N
        (refund)                       (refund)                          (per beneficiary)
```

Full lifecycle, storage layout, and event schema: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Tech stack

- **Rust** — toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml)
  (currently `1.88.0`), target `wasm32v1-none`
- **[Soroban SDK 22](https://docs.rs/soroban-sdk/22.0.0)**
- **[Stellar CLI](https://developers.stellar.org/docs/tools/cli)** for building,
  deploying, and invoking

## Quick start

```bash
rustup show                        # installs the pinned toolchain
rustup target add wasm32v1-none
cargo test --workspace             # 14 unit tests, native

stellar contract build             # optimized wasm -> target/wasm32v1-none/release/legacy.wasm
```

`cargo build --release --target wasm32v1-none -p legacy` is enough for a plain
compile check without the Stellar CLI. `stellar contract build` reproduces the
exact wasm hash that is deployed (see below).

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

## Deployed contract

| Field | Value |
| --- | --- |
| Network | Stellar **testnet** |
| Contract ID | `CAA55GCID6DTTQNUFMNT2PNKSBIDMMMKEPP6GKUKL3WJ3SH6QRRSXUNE` |
| Wasm hash | `d52d35a5cb25c249dfcbdb8602435bdaee43bde07254f44198c2793ba4bfad80` |
| Explorer | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CAA55GCID6DTTQNUFMNT2PNKSBIDMMMKEPP6GKUKL3WJ3SH6QRRSXUNE) |

## Environment variables

Copy `.env.example` to `.env`:

| Variable | Description | Example |
| --- | --- | --- |
| `NETWORK` | Stellar network to target | `testnet` |
| `RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `CONTRACT_ID` | Deployed Legacy contract id (`C…`) | *(set after deploy)* |

## Repository layout

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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). `main` is protected — open a PR, keep CI
green (`fmt`, `clippy`, `test`, `build`), one logical change per PR.

## Security

Unaudited, testnet only. Report vulnerabilities privately — see
[SECURITY.md](SECURITY.md).

## Contributors

<a href="https://github.com/heirloomss/heirloom-contracts/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=heirloomss/heirloom-contracts" />
</a>

## License

MIT — see [LICENSE](LICENSE).
