# Jupiter Swap-Path Coverage Audit

Scope: every program ID and discriminator a **swap** transaction signed via jup.ag (or any wallet that integrates Jupiter's swap API) could land on at the **top level**. Limit orders, DCA, perps, lend, vesting are explicitly out of scope — see `jupiter-ecosystem-discriminators.md` for those.

Sources cross-checked: `jup-ag/jupiter-cpi/idl.json`, `jup-ag/instruction-parser/src/idl/jupiter.ts`, Jupiter docs (`developers.jup.ag/docs/swap/routing`), Solscan, on-chain RE for `iris`, community parsers (`sevenlabs-hq/carbon`, `debridge-finance/solana-tx-parser`). Anchor sighashes computed as `sha256("global:<snake_case_name>")[..8]`.

## Consolidated table

| program ID | program name | disc (hex) | ix name | layout (offsets after disc) | covered by Faraday? |
|---|---|---|---|---|---|
| `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` | Jupiter v6 | `e517cb977ae3ad2a` | `route` | `route_plan vec | in_amount u64 | quoted_out u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | computed sighash | `route_v2` | `in_amount u64 | quoted_out u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | `936ec24a76b15be9` | `route_with_token_ledger` | `route_plan vec | quoted_out u64 | slip u16 | fee u8` (in from ledger) | yes |
| `JUP6Lkb…` | Jupiter v6 | `c1209b3341d69c81` | `shared_accounts_route` | `id u8 | route_plan vec | in_amount u64 | quoted_out u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | `d19853937cfed8e9` | `shared_accounts_route_v2` | `id u8 | in_amount u64 | quoted_out u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | `e6798f50779f6aaa` | `shared_accounts_route_with_token_ledger` | `id u8 | route_plan vec | quoted_out u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | `b04d09bff5fe6a9c` | `exact_out_route` | `route_plan vec | out_amount u64 | quoted_in u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | computed sighash | `exact_out_route_v2` | `out_amount u64 | quoted_in u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | `b8d4544c2dc56e9c` | `shared_accounts_exact_out_route` | `id u8 | route_plan vec | out_amount u64 | quoted_in u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | computed sighash | `shared_accounts_exact_out_route_v2` | `id u8 | out_amount u64 | quoted_in u64 | slip u16 | fee u8` | yes |
| `JUP6Lkb…` | Jupiter v6 | sighash | `set_token_ledger` | `(no args)` — companion to `*_with_token_ledger` swaps | yes (non-swap) |
| `JUP6Lkb…` | Jupiter v6 | sighash | `create_token_account` | `bump u8` | yes (non-swap) |
| `JUP6Lkb…` | Jupiter v6 | sighash | `create_token_ledger` | `(no args)` | yes (non-swap) |
| `JUP6Lkb…` | Jupiter v6 | sighash | `claim` | `id u8` — pulls referral / fee output | partial (filtered as non-swap; *does* move funds) |
| `JUP6Lkb…` | Jupiter v6 | sighash | `claim_token` | `id u8` — token version of `claim` | partial (same) |
| `proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u` | Ultra `iris` | `aa2955b184501f35` | `swap` (RE) | `opaque(8) | in_amount u64 | min_out u64 | slip u16 | <route+remaining>` | yes |
| `proVF4pMXVa…` | Ultra `iris` | unknown | additional ix? | not observed in production swap flows | n/a |
| `JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB` | Jupiter v4 (legacy) | varies — `instruction enum` | `route` / `tokenSwap` family | non-Anchor wrapper enum, layouts in old `@jup-ag/core` | **no** |
| `JUP3c25kjT8nVtFzjLrBFG…` (unverified) | Jupiter v3 (legacy) | unknown | — | program not confirmed live; not in any current routing table | **no — investigate** |
| `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA` | PumpSwap (CPI-only from v6) | n/a top-level for jup users | — | reached only as inner CPI from Jupiter v6 `route` | n/a |
| `DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH` | DFlow (CPI-only from `iris`) | n/a top-level for jup users | — | reached only as inner CPI from Iris when DFlow wins the RFQ leg | n/a (covered separately for non-Jupiter direct DFlow users) |
| `HFLowSwapV1aaaaaa...` (Hashflow, unverified) | Hashflow (CPI-only from `iris`) | n/a top-level | — | RFQ leg under Iris meta-aggregator | n/a |

Notes:
- `JUP6Lkb…` discriminators marked "computed sighash" instead of an explicit hex value: Faraday's parser builds them at runtime via `anchor::discriminator(name)`, so the literal bytes never appear in code. They are correct by construction. The table lists explicit hex only where a primary source (instruction-parser, jupiter-cpi/idl.json) publishes the precomputed value.
- `claim` and `claim_token` move the user's accumulated referral / platform-fee output out of the v6 program's escrow into a wallet. They are user-signed and currently silently filtered as non-swap; see punch list.
- "covered by Faraday? = partial" means parser doesn't crash and reaches the right code path, but the screen the user sees is generic / understates risk.

## Missing-coverage punch list (ranked by user-hit probability)

1. **Jupiter v4 — `JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB`** *(low traffic, but non-zero)*. Legacy aggregator; still receives traffic from old integrators that pinned `@jup-ag/core` and never moved to the v6 swap API. Solscan still lists it as "Jupiter Aggregator v4". v4 is **not Anchor** — it's a native program with a u8 enum-tag discriminator (the old `JupiterIxId`), data shapes documented in `@jup-ag/core` v3.x source. *Action*: add a top-level program-ID match in `programs.rs` that returns a non-fatal "Jupiter v4 (legacy aggregator) — verify amounts on dApp" review screen even before parsing data; this is strictly better than "unknown program". Full parse is low priority.

2. **`claim` and `claim_token` on v6**. Currently filtered into the `is_known_non_swap` bucket, which renders the same as `set_token_ledger`. These ix actually transfer fee/referral output to the user — the right UX is a distinct "Jupiter — Claim Token" header with the destination account shown. *Action*: add `JupiterInstruction::Claim` and `ClaimToken` variants, parse `id: u8`, render destination ATA from the named account slot. Layout is trivial.

3. **Verify `route_v2` / `exact_out_route_v2` / `shared_accounts_*_v2` are still authoritative**. These names are not in the public `jup-ag/jupiter-cpi/idl.json` as of last verification but Faraday's tests show they parse correctly against captured production txs. The risk is that Jupiter renames or retires them silently. *Action*: keep an integration test pinned to a known-good captured tx for each variant; the test will start failing the day Jupiter changes the discriminator.

4. **Ultra `iris` companion ix**. `aa2955b184501f35` is the only swap discriminator we've ever seen on `proVF4pMXVa…`, but Iris is closed-source and Jupiter has shipped Ultra v3 (Q4 2025) with new features — predictive execution, ShadowLane signaling, and Ultra Signaling. Some of those *may* have surfaced as new top-level discriminators, though more likely they're ix arguments. *Action*: run `solana logs proVF4pMXVa…` for ~24h and group by `data[..8]`; alert on any disc that isn't `aa2955b184501f35`. Add a defensive "Jupiter Ultra — unknown variant, verify on dApp" path so an unknown disc doesn't fail closed.

5. **`route_with_token_ledger` and `shared_accounts_route_with_token_ledger`**. Already covered by the parser, but the in_amount comes from the **ledger account state**, not the ix data — Faraday currently shows `0` or "could not parse" for `in_amount` on these. *Action*: this is a known structural limit of an offline parser; render an explicit "Input amount: from ledger account (resolved at execution)" line so the user knows it's intentional, not a bug.

## Confirmed safe to ignore (CPI-only — never top-level on jup.ag swap)

These programs are reached only as inner CPIs from Jupiter v6 `route` / `iris` `swap`. A user signing a Jupiter swap will *never* see them as the top-level program in their tx. If a top-level call to one of these appears, it means the user is on a non-Jupiter dApp (Raydium UI, Orca UI, Pump.fun, etc.) and a different parser path applies.

- `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` (Orca Whirlpools)
- `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` (Raydium CLMM)
- `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` (Raydium CPMM)
- `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` (Raydium AMM v4)
- `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` (Meteora DLMM)
- `PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY` (Phoenix)
- `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA` (PumpSwap AMM)
- `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` (Pump.fun bonding curve)
- `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` (Meteora DBC — Studio backing pool)
- `DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH` (DFlow — CPI from Iris when DFlow wins; users hitting DFlow directly have their own integrator UIs)
- Hashflow / OKX RFQ programs (program IDs not yet confirmed; CPI from Iris)
- "JupiterZ" RFQ does **not** have a separate top-level program ID — the public docs and team statements describe it as a quote-source feeding the v6 / Iris routers, with on-chain settlement going through the same `JUP6Lkb…` shared_accounts_route path. There is an internal "Order Engine" program referenced in docs, but no evidence yet that a user signs against it directly.

If a top-level call lands on any of these on a Jupiter-flow tx, treat as a parser bug — Jupiter doesn't route that way for swaps.

## Unknowns / verification needed

1. **JupiterZ Order Engine program ID.** Jupiter docs reference an "Order Engine Program" in `programs/order-engine` (not public on `github.com/jup-ag` as of May 2026). Verify by inspecting a JupiterZ-routed swap tx and listing all program IDs invoked at the top level. Command: `solana confirm -v <tx-sig>` and check the program IDs in the message. If none is new beyond `JUP6Lkb…` / `proVF4pMXVa…`, RFQ settlement is fully under one of those and no new top-level coverage is needed.

2. **Jupiter v3 program ID.** Searches for `JUP3c…` / `JUP2…` returned no public hits. There is no community confirmation that v3 ever had a published Solana program (v4 may have been the first on-chain release of the Anchor-style aggregator). Treat v3 as nonexistent until proven otherwise.

3. **v4 instruction enum dump.** To confirm v4 layout, run:
   ```
   solana program dump JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB jup_v4.so
   ```
   then disassemble or feed to `solana-tx-parser` against a captured v4 tx. The first byte of `data` is the enum tag; v4 has < 16 variants. Realistically, the right call is the lightweight "v4 detected, verify on dApp" screen rather than a full parser.

4. **Ultra additional discriminators.** Tail a sampling window of `proVF4pMXVa…` logs (e.g. 24h) and bucket by `data[..8]`. If `aa2955b184501f35` is > 99.9% of the volume, single-disc coverage is bulletproof. If not, capture a tx for each new disc and RE the layout.

5. **`shared_accounts_route_with_token_ledger` discriminator.** Faraday currently uses `e6798f50779f6aaa`. The `instruction-parser/src/idl/jupiter.ts` IDL is the authoritative source — pin a unit test that loads that file directly and asserts the disc on every CI run, so a Jupiter-side rename trips the test.

6. **`route_v2` precomputed disc.** Faraday computes via `anchor::discriminator("route_v2")` at runtime. To make the table above complete, run once locally: `python -c 'import hashlib; print(hashlib.sha256(b"global:route_v2").hexdigest()[:16])'` and paste back into the table.

## TL;DR for the parser

For swap-only coverage on a Jupiter user, Faraday is **already complete** on the two programs that handle ~100% of jup.ag swap volume (`JUP6Lkb…` + `proVF4pMXVa…`). The realistic gaps are:

- v4 legacy traffic — fix with a one-line "verify on dApp" stub, not a full parser.
- `claim` / `claim_token` — currently mislabeled as no-op when they actually move funds.
- Ultra ix surface drift — defend with a fall-through warning rather than a hard fail.

Everything else listed above is either CPI-only (won't be seen top-level) or out of scope (limit/DCA/perps/lend, covered in `jupiter-ecosystem-discriminators.md`).
