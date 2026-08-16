# 2026-08-16 — FA-26 in review: durable nonce for dapp transactions

Dapp-built transactions (Jupiter swaps, playground, the Ika demo) now get a
durable-nonce lifetime before signing, when it's safe to rewrite them —
closing the gap FA-09 (PR #112) left open. Branch `feat/dapp-nonce-rewrite`,
PR #134.

**Product decision revised (owner: cxalem):** FA-09 said dapp-built
transaction messages are never modified. That's now revised: the background
sign-session handler rewrites a dapp's unsigned transaction to durable-nonce
form when it's provably safe to do so, and falls back to byte-exact
passthrough on every case it isn't. A dapp sign must never hard-fail because
of the rewrite — the fallback path is the same "pass the bytes through
untouched" behavior FA-09 shipped everywhere else.

## Extension (`extension/`)
- New `src/lib/nonce-rewrite.ts`: `rewriteDappTxToDurableNonce` never throws.
  Gates run cheapest-first, no RPC before the wallet's nonce account is
  confirmed to exist: partial signatures already present, fee payer isn't the
  wallet, already durable-nonce, no nonce account provisioned — then the
  actual rewrite (decompile, fetching ALT contents only when the message
  references any → set durable-nonce lifetime → recompile → size gate) under
  a ~4s timeout. Never calls
  `compressTransactionMessageUsingAddressLookupTables` after rewriting — it
  could fold the nonce account itself into a dapp-supplied lookup table.
- `entrypoints/background.ts`: `faraday:create-sign-session` runs the rewrite
  before risk analysis, so the risk report describes what's actually signed.
  A wallet with no nonce account gets `needsNonceProvision: true` on the
  session (original bytes, untouched) instead of a rewrite attempt. New
  `faraday:rewrite-session-nonce` handler retries the rewrite once
  provisioning completes.
- `entrypoints/sign/sign-app.tsx`: one-time provisioning interstitial in the
  sign popup — reuses the existing Display/Scan QR screens for the
  create-nonce-account transaction. Skip, or any failure at any step
  (broadcast, confirmation wait, background retry), falls through to signing
  the original transaction normally; provisioning never blocks a dapp sign.
  Small "won't expire mid-scan" badge when a tx was actually rewritten.
- `src/lib/types.ts` / `storage.ts`: `ExtensionState.dappNonceRewrite`
  kill-switch (undefined ⇒ ON). Settings → Network gets an ON/OFF pill row
  ("Durable nonce for dapp transactions") for dapps that broadcast their own
  original copy of the transaction instead of the signed bytes Faraday
  returns.
- `src/lib/solana.ts`: exported `parseEnvelope`/`TxEnvelope` for reuse by the
  new module (previously module-private).
- New `src/lib/nonce-rewrite.test.ts`: 10 cases, fixtures built with
  `@solana/kit` itself. Legacy rewrite; v0+ALT rewrite (asserts
  `addressTableLookups` + the writable index survive on the raw recompiled
  bytes); already-durable-nonce; partial-signature multi-signer tx; wrong fee
  payer; no nonce account (asserts zero fetch calls); oversize (tuned
  21-recipient tx); RPC error; stalled-RPC timeout; wallet's own address also
  reachable via an ALT lookup (kit 5.5.1 dedups rather than throwing — pinned
  so a future kit upgrade that changes this surfaces here, not as a silent
  dapp-sign failure).
- 167 extension tests pass; typecheck (`wxt prepare && tsc --noEmit`) and the
  MV3 build (`npm run build`) are clean.

## Not yet verified (owner to confirm on devnet)
- Acceptance (a): a dapp-signed devnet transaction submitting 2+ minutes
  after signing — needs a live devnet RPC + real dapp flow, not run in this
  environment.
- Acceptance (d): the provisioning interstitial's live QR round-trip (Set up →
  create-nonce-account signs on-device → real tx follows) and the Skip /
  failure fallback — implemented and typechecked, but there's no existing
  React-component or `background.ts` test harness in this repo to exercise it
  headlessly, so it wasn't run end-to-end here.
- `docs/state.md` doesn't exist in this repo (checked at `origin/main`) — not
  created as part of this card; flagging rather than inventing a new doc file
  outside the card's scope.

## Known limitation (same as FA-09)
One nonce account per wallet: two dapp transactions signed concurrently race
the same nonce and one fails to land. Documented in the module header;
future work is a nonce pool.

## Follow-up
- Firmware `extract_ika` doesn't yet tolerate a leading
  `AdvanceNonceAccount` the way `extract_send` does — the Ika demo won't
  benefit from this rewrite until that's fixed. Out of scope for this card
  (extension-only); no backlog card cut for it yet.
