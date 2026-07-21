# 2026-07-22 — FA-09 in review: durable-nonce transactions

Signed transactions relayed as QR codes between browser and device no longer
depend on a perishable blockhash. Branch `feat/durable-nonce`, PR #112.

**Product decision (owner: cxalem, not re-litigated):** durable nonce **always,
wherever Faraday itself builds the transaction**. Dapp-built transaction
messages are left untouched — altering them would break what the dapp expects
to submit.

## Device (`hardware/`)
- System-program classifier now labels `AdvanceNonceAccount` (disc 4) and
  `InitializeNonceAccount` (disc 6) as System instructions with nonce-account +
  authority rows, instead of hitting the unknown-instruction warning path.
  Malformed/truncated variants still fail safe (warn, never guess).
- New `gen-nonce-fixtures` binary + `just nonce-fixtures`; committed legacy + v0
  fixtures (`testdata/examples/nonce/`) whose first instruction is
  `AdvanceNonceAccount`. Fixture-backed tests pin the labeling at the byte level.
- 286 hardware tests pass; `cargo check` host + ARM cross-compile clean under
  `-D warnings`.

## Extension (`extension/`)
- `src/lib/nonce.ts`: durable-nonce transfer builder (leads with
  `AdvanceNonceAccount` via `setTransactionMessageLifetimeUsingDurableNonce`),
  one-time create+initialize builder (ephemeral nonce keypair pre-signed, wallet
  signs on device), fetch-nonce, rent + provisioning orchestration.
- Storage/types: one nonce account per wallet (address only — the nonce keypair
  is ephemeral and never persisted).
- Send flow: first send from a wallet provisions the nonce account (a create
  round-trip), waits for it to confirm, then signs the transfer; stale-nonce
  rebuild re-fetches the current nonce on retry.
- 150 extension tests pass (byte-level nonce coverage added); typecheck + MV3
  build clean.

## Not yet verified (owner to confirm on devnet + a real device)
- Acceptance #1 (end-to-end devnet round-trip: sign → wait 2+ min → submit) and
  the on-device 2-signer create-nonce relay were **not** run in this environment
  (no devnet/hardware access here). The logic is unit-tested; the live flow
  needs a device pass before this ships to users.

## Follow-up
- **FA-19** cut: port durable nonces to the mobile send flow (extension-only
  here, per "one concern per PR"). Reuses the shared device parser support.
