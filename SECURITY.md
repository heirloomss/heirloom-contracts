# Security Policy

## Audit status

**Heirloom is unaudited.** The `legacy` contract has not undergone a third-party
security audit. It is deployed on Stellar **testnet** only. Do not use it with
real value on mainnet until an audit has been completed.

## Reporting a vulnerability

Please report security issues **privately**. Do not open a public GitHub issue
for anything that could put user funds or data at risk.

- Email: **chijiokejoseph2022@gmail.com**
- Telegram: **@cjay**

Include: a description of the issue, the contract function or file involved,
steps to reproduce (or a proof-of-concept), and the impact you foresee.

You will get an acknowledgement within **72 hours**. We ask that you give us a
reasonable window to ship a fix before any public disclosure. We will credit
reporters who want credit.

## Scope

In scope:

- `contracts/legacy` — the Legacy inheritance contract (state machine, auth,
  fund custody, claim accounting, refund logic).
- Deployment and initialization scripts under `scripts/`.

Out of scope:

- The off-chain services (`heirloom-api`, `heirloom-web`) — report those in
  their own repositories.
- Testnet lumen exhaustion / rate limiting on public RPC.
- Findings that require a compromised owner, guardian, or beneficiary key.
