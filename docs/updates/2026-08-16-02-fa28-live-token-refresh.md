# 2026-08-16 — FA-28 in review: token list refreshes on the live push

The sidepanel's live account WebSocket (`use-live-balance.ts`) already
revalidated the SOL balance on every push; the SPL token list stayed on its
60s poll, so a token from a swap/receive could sit stale up to a minute.
Wired the same push to a debounced token-list refresh. Branch
`feat/live-token-refresh`, PR #135.

## Extension (`extension/`)
- `src/lib/use-tokens.ts`: factored the SWR cache key into `tokensKey(pubkey)`
  (used by `useTokens` itself, unchanged behavior) and added
  `refreshTokens(pubkey)` — calls SWR's global `mutate` on that same key so a
  revalidation can be triggered from outside the hook. It's a no-op unless
  some mounted `useTokens(pubkey)` (e.g. `TokensSection` or
  `TokenDetailScreen`, both already mounted alongside `useWallet` wherever
  tokens are shown) has registered that key's fetcher — no new fetch source,
  no restructuring of either screen.
- `src/lib/use-wallet.ts`: the live-push callback passed to `useLiveBalance`
  is now `onLivePush`, which calls `refreshBalance()` (unchanged, synchronous,
  every push) and schedules a trailing-debounced (2s) call to
  `refreshTokens(pairedPubkey)`. A burst of notifications resets the timer
  rather than firing once per notification; the timer is cleared on unmount.
  `useLiveBalance`'s internals (reconnect/backoff) were not touched — it just
  receives a different `onChange` function via its existing `onChangeRef`
  keep-latest pattern.
- 167 extension tests pass; typecheck (`wxt prepare && tsc --noEmit`) and the
  MV3 build (`npm run build`) are clean.

## Not verified here
- No React hook-testing infra exists in this repo (`vitest` only, no
  `@testing-library/react`/jsdom) — a render-level test of the debounce
  behavior would need adding that infra, which is out of scope for this small
  card. Verified instead by typecheck + full test suite + code review per the
  card's acceptance criteria (a)'s "else" branch.
- Live devnet behavior (swap → token appears within ~2s of the push instead
  of up to 60s) not exercised in this environment — no live wallet/RPC here.

## Not repeated (already flagged in the FA-26 update)
- `docs/state.md` doesn't exist in this repo; not created here either.
