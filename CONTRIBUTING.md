# Contributing to Heirloom Contracts

Thanks for helping build Heirloom. This repo holds the trust-critical on-chain
code, so the bar for changes is deliberately high.

## Ground rules

- Every change goes through a pull request. `main` is protected — no direct
  pushes.
- One logical change per PR. A new function, a bug fix, a test block — not five
  unrelated things.
- CI must be green: `fmt`, `clippy`, `test`, `build`.
- No `unwrap()` / `expect()` / `panic!` in contract code outside `#[cfg(test)]`.
  Return a typed `Error` variant instead.
- No floating point. All proportional math is integer basis points
  (`10_000` bps = 100%).
- Never widen `pub` surface "just in case". Every public function must map to a
  real step in the product flow.

## Setup

```bash
rustup show                      # installs the pinned toolchain from rust-toolchain.toml
rustup target add wasm32v1-none
cargo test --workspace           # unit tests, native
stellar contract build           # produces the optimized wasm
```

You need the [Stellar CLI](https://developers.stellar.org/docs/tools/cli) for
`stellar contract build` and for deploying. `cargo build --release --target
wasm32v1-none -p legacy` is enough for a plain compile check.

## Commit format

Conventional commits, present tense, scoped:

```
feat(legacy): add per-beneficiary claim expiry
fix(storage): extend persistent TTL on approval writes
test(legacy): cover double-claim rejection
docs(readme): document the deploy script
```

Commit one logical unit at a time and push after each. Do not run `git add .` —
stage the specific files you changed.

## Pull request checklist

- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] New behaviour has tests (happy path + at least one rejection path)
- [ ] Auth: every state-changing function calls `require_auth()` on the right
      address, and the PR description says which
- [ ] Events: any new state transition emits an event
- [ ] No change to the deployed wasm hash unless the PR is explicitly a contract
      upgrade

## Reporting bugs

Functional bugs: open a GitHub issue with a failing test if you can.
Security issues: **do not** open an issue — see [SECURITY.md](SECURITY.md).
