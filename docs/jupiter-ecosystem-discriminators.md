# Jupiter Ecosystem — Program & Instruction Discriminator Map

Reference for Faraday's offline transaction parser. Goal: enumerate every program a jup.ag user could land on, so unknown-discriminator screens can be replaced with at least an action label. Discriminators are 8-byte Anchor sighashes (`sha256("global:<snake_case>")[..8]`) unless noted. Anchor IDLs serialise instruction names in camelCase but on-chain sighashes are computed from the snake_case Rust handler — both are listed where they diverge.

Where a value below is marked **(unverified)** it could not be confirmed against an authoritative IDL or chain account; treat as a starting point and verify with `solana program dump` against the address before shipping.

---

## 1. Jupiter Aggregator v6 — `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`

Sources: [`jup-ag/jupiter-cpi/idl.json`](https://github.com/jup-ag/jupiter-cpi), [`jup-ag/instruction-parser/src/idl/jupiter.ts`](https://github.com/jup-ag/instruction-parser/blob/main/src/idl/jupiter.ts), Solana Explorer anchor IDL.

Faraday already covers the swap-side ix (see `src/parser/jupiter.rs`). What's left in the public IDL is the long tail — utility, claim, and DEX-CPI handlers:

| disc source | snake_case name | data layout (post-disc) | risk: moves user funds? |
|---|---|---|---|
| sighash | `route` | `route_plan: vec<Step> \| in_amount: u64 \| quoted_out: u64 \| slip: u16 \| fee: u8` | yes (covered) |
| sighash | `route_v2` | `in_amount: u64 \| quoted_out: u64 \| slip: u16 \| fee: u8` | yes (covered) |
| sighash | `route_with_token_ledger` | `route_plan \| quoted_out \| slip \| fee` (in_amount from ledger acct) | yes (covered) |
| sighash | `shared_accounts_route` | `id: u8 \| route_plan \| in_amount \| quoted_out \| slip \| fee` | yes (covered) |
| sighash | `shared_accounts_route_v2` | `id: u8 \| in_amount \| quoted_out \| slip \| fee` | yes (covered) |
| sighash | `shared_accounts_route_with_token_ledger` | `id: u8 \| route_plan \| quoted_out \| slip \| fee` | yes (covered) |
| sighash | `exact_out_route` | `route_plan \| out_amount \| quoted_in \| slip \| fee` | yes (covered) |
| sighash | `exact_out_route_v2` | `out_amount \| quoted_in \| slip \| fee` | yes (covered) |
| sighash | `shared_accounts_exact_out_route` | `id: u8 \| route_plan \| out_amount \| quoted_in \| slip \| fee` | yes (covered) |
| sighash | `shared_accounts_exact_out_route_v2` | `id: u8 \| out_amount \| quoted_in \| slip \| fee` | yes (covered) |
| sighash | `set_token_ledger` | `(no args)` | no (covered as non-swap) |
| sighash | `create_token_account` | `bump: u8` | no |
| sighash | `create_token_ledger` | `(no args)` | no (covered as non-swap) |
| sighash | `create_open_orders` | `(no args)` | no |
| sighash | `create_program_open_orders` | `id: u8` | no |
| sighash | `claim` | `id: u8` | yes — pulls referral / fee output to user |
| sighash | `claim_token` | `id: u8` | yes — token version of `claim` |

**DEX CPI handlers** (`whirlpool_swap`, `meteora_dlmm_swap`, `phoenix_swap`, `raydium_clmm_swap`, `mercurial_swap`, `cykura_swap`, `serum_swap`, `saber_swap`, `saber_add_decimals`, `token_swap`, `token_swap_v2`, etc.) are exposed in the IDL but only ever called as inner CPIs from inside `route`. They are **not** seen as top-level program ix on Jupiter v6 — skip in the top-level parser, which Faraday already does.

`open_order_initialize` / `close_order` currently in Faraday's `is_known_non_swap` list **do not appear in the public v6 IDL** — those are Limit Order v2 names (program below). Move them.

When jup.ag invokes this: every Swap, every Trigger Order *fill leg*, every DCA *swap leg*. The default lander `jup.ag/swap` is 100% this program.

---

## 2. Jupiter Ultra `iris` router — `proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u`

No public IDL, no `jup-ag` GitHub repo as of May 2026. Reverse-engineered from on-chain data only.

| disc (hex) | name | data layout | risk |
|---|---|---|---|
| `aa2955b184501f35` | `swap` (RE) | `disc \| opaque(8) \| in_amount: u64 \| min_out: u64 \| slip: u16 \| <route + remaining accts>` | yes (covered) |

Ultra ships a single-ix interface by design — the router is opaque to integrators. Additional swap discriminators are unlikely. Possible admin / quote-id init ix exist but don't move user funds. Verification path: `solana logs proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u`, group by `data[..8]`. **No coverage gap expected.**

When jup.ag invokes this: jup.ag/ultra and jup.ag/swap when "Ultra" mode is on (default for many users in 2025+).

---

## 3. Jupiter Limit Order v2 — `j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X` (per [Solscan](https://solscan.io/account/j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X))

Note: An older program at `jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZkyknqvzYNrNu` is the **v1** Limit Order (still referenced in Jupiter docs and the shapeshift agentic-chat issue). v2 launched 2024-10-29 with privacy/cloaking; jup.ag/limit routes to v2 today. Both are reachable via jup.ag — Faraday should cover both IDs and treat them as the same program family.

Public source: [Limit Order v2 Smart Contract Audit (Offside, Apr 2024)](https://dev.jup.ag/static/files/audits/limit-v2-offside.pdf). No public Anchor IDL JSON; instruction names below are from audit + API docs. Discriminators must be derived from the snake_case names (sighash) and verified on-chain.

| sighash source | snake_case name | data layout | risk |
|---|---|---|---|
| sighash | `initialize_order` (a.k.a. `create_order`) | `making_amount: u64 \| taking_amount: u64 \| expired_at: i64? \| fee_bps: u16` | yes — escrows maker token |
| sighash | `cancel_order` | `(no args)` | yes — returns escrowed maker token to user |
| sighash | `cancel_expired_order` | `(no args)` | no — anyone-callable |
| sighash | `flash_fill_order` | `making_amount: u64` | yes (taker side) |
| sighash | `fill_order` | `making_amount: u64 \| max_taking_amount: u64` | yes (taker side) |
| sighash | `withdraw_fee` | `(no args)` | no (admin/keeper) |
| sighash | `init_fee` / `init_pda` | varies | no (admin) |

**Faraday gap:** all of these. v6's `open_order_initialize`/`close_order` entries should be moved here and renamed to `initialize_order`/`cancel_order`. These are the most user-visible Jupiter ix Faraday currently can't name.

When jup.ag invokes this: jup.ag/limit (creating, cancelling). Fills are usually triggered by Jupiter's keepers but the CPI to v6 is what lets the order trade — so a user signing a "Cancel Order" will hit `cancel_order` here, not v6.

---

## 4. Jupiter DCA — `DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M`

Source: [`jup-ag/dca-cpi`](https://github.com/jup-ag/dca-cpi) (archived Nov 2025, IDL-derived crate), [Jupiter docs `dca/integration`](https://dev.jup.ag/docs/old/dca/integration), [Solscan](https://solscan.io/account/DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M).

| sighash source | snake_case name | data layout | risk |
|---|---|---|---|
| sighash | `open_dca` | `application_idx: u64 \| in_amount: u64 \| in_amount_per_cycle: u64 \| cycle_frequency: i64 \| min_out_amount: u64? \| max_out_amount: u64? \| start_at: i64?` | yes — escrows full `in_amount` |
| sighash | `open_dca_v2` | same shape with optional fields explicit | yes |
| sighash | `close_dca` | `(no args)` | yes — returns remaining input + collected output |
| sighash | `withdraw` | `withdraw_amount: u64 \| withdrawal: enum{In, Out}` | yes — partial withdraw mid-DCA |
| sighash | `deposit` | `deposit_in: u64` | yes — adds funds to active DCA |
| sighash | `transfer` | `(no args)` | no — keeper-only crank |
| sighash | `end_and_close` | `(no args)` | yes — final cleanup |

**Faraday gap:** all of these. Coverage priority: medium — DCA users sign `open_dca` once per recurring order then rarely touch the program until `close_dca`.

When jup.ag invokes this: jup.ag/dca (formerly), now folded into jup.ag/recurring. Both the legacy DCA UI and the new "time-based recurring" path go through this program.

---

## 5. Jupiter VA (Value Averaging) — program ID **(unverified)**

No public IDL, no `jup-ag` GitHub repo for VA found. Solscan's program-data-coverage page lists "Jupiter VA" but the address is not surfaced in public web search. Most likely candidate: a separate `VA…` prefixed program deployed alongside DCA (Jupiter's pattern). **Action item:** pull a `jup.ag/va` tx and read the program ID off it, then dump the IDL with `anchor idl fetch`.

Expected ix shape (mirroring DCA): `open_va`, `close_va`, `deposit`, `withdraw`, `end_and_close`. Same risk profile as DCA.

**Faraday gap:** complete. Coverage priority: low-medium — VA has far less volume than DCA but is a one-click feature on jup.ag.

When jup.ag invokes this: jup.ag/va, and jup.ag/recurring "price-based" mode.

---

## 6. Jupiter Perpetuals — `PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu`

Sources: [`Garrett-Weber/jupiter-perpetuals-cpi`](https://github.com/Garrett-Weber/jupiter-perpetuals-cpi) (`declare_id!("PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu")`), [`julianfssen/jupiter-perps-anchor-idl-parsing`](https://github.com/julianfssen/jupiter-perps-anchor-idl-parsing), [DeepWiki perps program-instructions](https://deepwiki.com/pengxuan37/jupiter-perps-anchor-idl-parsing/2.1-program-instructions).

User-facing instructions (camelCase from IDL → snake_case for sighash):

| name (camelCase / snake) | layout | risk |
|---|---|---|
| `createIncreasePositionMarketRequest` / `create_increase_position_market_request` | `size_usd_delta: u64 \| collateral_token_delta: u64 \| price_slippage: u64 \| jupiter_minimum_out: u64? \| counter: u64 \| triggerPrice` | yes — locks collateral |
| `createDecreasePositionRequest2` / `create_decrease_position_request2` | `size_usd_delta: u64 \| collateral_usd_delta: u64 \| trigger_price: u64 \| trigger_above_threshold: bool` | yes |
| `createDecreasePositionMarketRequest` / `create_decrease_position_market_request` | similar | yes |
| `updateDecreasePositionRequest2` | `size_usd_delta \| collateral_usd_delta \| trigger_price \| trigger_above_threshold` | no (modifies pending) |
| `closePositionRequest` / `close_position_request` | `(no args)` | yes — cancels & returns |
| `closePositionRequest2` | `(no args)` | yes |
| `instantCreateLimitOrder` / `instant_create_limit_order` | `size_usd \| collateral \| trigger_price \| ...` | yes |
| `instantCreateTpsl` / `instant_create_tpsl` | TP/SL params | yes |
| `instantUpdateLimitOrder` / `instantUpdateTpsl` | update params | no |
| `instantIncreasePosition` / `instant_increase_position` | direct (no two-step) | yes |
| `instantDecreasePosition` / `instant_decrease_position` | direct | yes |
| `addLiquidity2` / `add_liquidity2` | `token_amount_in: u64 \| min_lp_amount_out: u64` | yes — JLP mint |
| `removeLiquidity2` / `remove_liquidity2` | `lp_amount_in: u64 \| min_amount_out: u64` | yes — JLP burn |
| `swap2` / `swap2` | `amount_in: u64 \| min_amount_out: u64` | yes (JLP-internal swap) |

Keeper/admin (skip): `increasePosition4`, `decreasePosition4`, `liquidateFullPosition4`, `addPool`, `addCustody`, `setPerpetualsConfig`, `withdrawFees2`, `refreshAssetsUnderManagement`, `transferAdmin`, `init`, `setMaxGlobalSizes`, etc.

**Faraday gap:** complete. Coverage priority: high — jup.ag/perps users sign `create*Request` ix on every position open/close and `addLiquidity2`/`removeLiquidity2` for the JLP page (which is one click from the main nav).

When jup.ag invokes this: jup.ag/perps (positions), jup.ag/perps-earn or jup.ag/jlp (LP).

---

## 7. Jupiter Lend (Earn) — `jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9`

Source: [`jup-ag/jupiter-lend/docs/earn/cpi.md`](https://github.com/jup-ag/jupiter-lend/blob/main/docs/earn/cpi.md), [Code4rena audit `2026-02-jupiter-lend`](https://github.com/code-423n4/2026-02-jupiter-lend). Devnet ID: `7tjE28izRUjzmxC1QNXnNwcc4N82CNYCexf3k8mw67s3`.

| disc (bytes) | snake_case name | layout | risk |
|---|---|---|---|
| `f223c68952e1f2b6` (`[242, 35, 198, 137, 82, 225, 242, 182]`) | `deposit` | `amount: u64` | yes — pulls underlying, mints fToken |
| `b7124694946da122` (`[183, 18, 70, 156, 148, 109, 161, 34]`) | `withdraw` | `assets: u64` | yes — burns fToken, returns underlying |

Per the public CPI doc this is the entire user-facing Earn surface — the audit repo also lists `rebalance` (keeper) and `reserves_*` (admin) but those don't move user funds.

**Borrow/vault side** (separate sub-programs `vaults`, `liquidity`, `flashloan`, `oracle`) — public docs only ship the Earn CPI. The audit repo reveals additional programs:

- `liquidity` — single ix `operate` (variant-tagged: deposit / withdraw / borrow / payback)
- `vaults` — `operate`, `liquidate`, `rebalance`
- `flash_loan` — flash borrow/repay

Program IDs for the borrow / vaults / liquidity sub-programs are not in the public CPI doc as of May 2026. **Action item:** check the audit-repo `Anchor.toml`.

**Faraday gap:** Earn `deposit`/`withdraw` are easy wins (discriminators known). Borrow/vaults need ID lookup first.

When jup.ag invokes this: jup.ag/lend (Earn tab — deposits & withdrawals; the most-trafficked Lend page).

---

## 8. Jupiter Studio (token launch) — Meteora DBC `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` **(unverified)**

Studio is a launchpad UI on top of the Meteora Dynamic Bonding Curve (DBC) program. The DBC program is **not Jupiter** — it's a third-party AMM Jupiter resells. Buying/selling Studio tokens routes through Jupiter v6 (covered) → DBC pool (CPI). Direct top-level calls to DBC happen for token-creator actions (mint, claim creator fees, migrate to AMM).

Risk for end users buying tokens on jup.ag/studio: **none new** — they hit the v6 swap path. Risk for token *creators*: a separate exposure surface, lower priority for a hardware wallet aimed at swap users.

**Faraday gap:** acceptable to defer. If a creator path is in scope, dump the DBC IDL separately.

---

## 9. Jupiter Governance (Voter / Locked Voter) — program ID **(unverified)**

vote.jup.ag deposits JUP into a "Locked Voter" contract (the Tribeca-style locker pattern Jupiter forked, governance currently paused until 2026 per public statements). No `jup-ag` GitHub repo published for the voter program; the locker pattern is well-known (escrow init, increase locked amount, extend, cast vote, withdraw).

Expected ix names (from the Tribeca/Quarry pattern Jupiter inherited): `new_escrow`, `increase_locked_amount` / `lock_with_whitelist`, `toggle_max_lock`, `cast_vote`, `withdraw`, `claim`.

**Faraday gap:** complete. Coverage priority: low — voting is paused until 2026 and the address space is small (one program). When un-paused, this jumps to medium.

When jup.ag invokes this: vote.jup.ag (staking JUP, voting).

---

## 10. Jupiter Lock (vesting) — `LocpQgucEQHbqNABEYvBvwoxCPsSbG91A1QaQhQQqjn`

Source: [`jup-ag/jup-lock/programs/locker/src/lib.rs`](https://github.com/jup-ag/jup-lock).

| snake_case name | risk | notes |
|---|---|---|
| `create_vesting_escrow` | yes — locks creator's tokens | original v1 |
| `create_vesting_escrow_v2` | yes | adds Token-2022 support |
| `create_vesting_escrow_metadata` | no | label/description |
| `update_vesting_escrow_recipient` | no | reassign recipient |
| `claim` | yes — recipient-side | v1 |
| `claim_v2` | yes | v2 |
| `cancel_vesting_escrow` | yes — sender side | clawback |
| `close_vesting_escrow` | no | rent recovery |
| `create_root_escrow` | yes | merkle-root multi-recipient |
| `fund_root_escrow` | yes | |
| `create_vesting_escrow_from_root` | yes | merkle claim path |

Staging ID: `sLovrBvGxvyvBniMxj8uUt9CdD7CV4PhnBnBD6cPSXo`. Localnet: `2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg`.

**Faraday gap:** complete. Coverage priority: low for swap users, medium for token creators / claim recipients (lock.jup.ag is a one-off interaction for most users).

When jup.ag invokes this: lock.jup.ag (creator lock + recipient claim).

---

## Coverage Gap — ranked by jup.ag user-hit probability

1. **Limit Order v2** (`j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X` and v1 `jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZkyknqvzYNrNu`): `initialize_order`, `cancel_order`, `flash_fill_order`, `fill_order`. **Highest priority** — every limit-order user signs these. The v6 `open_order_initialize`/`close_order` table entries are misplaced and should be moved here.
2. **Perpetuals** (`PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu`): `create_increase_position_market_request`, `create_decrease_position_request2`, `instant_increase_position`, `instant_decrease_position`, `instant_create_tpsl`, `instant_create_limit_order`, `add_liquidity2`, `remove_liquidity2`, `swap2`. **High** — perps + JLP are top-level nav items.
3. **DCA** (`DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M`): `open_dca`, `open_dca_v2`, `close_dca`, `withdraw`, `deposit`, `end_and_close`. **Medium-high** — recurring orders are a marquee feature.
4. **Lend Earn** (`jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9`): `deposit` (`f223c68952e1f2b6`), `withdraw` (`b7124694946da122`). **Medium** — discriminators are already known, two ix away from full coverage.
5. **Lend Borrow / Vaults sub-programs**: program IDs not yet pulled. **Medium-low** — borrow has lower volume than earn.
6. **VA (Value Averaging)** — program ID **must be confirmed from chain** before parsing. **Low-medium**.
7. **Lock** (`LocpQgucEQHbqNABEYvBvwoxCPsSbG91A1QaQhQQqjn`): `create_vesting_escrow_v2`, `claim_v2`, `cancel_vesting_escrow`. **Low** for swap users.
8. **Governance / Locked Voter** — paused until 2026. **Low** today, **medium** post-resume.
9. **Studio** — runs through v6 → Meteora DBC, no new top-level Jupiter program for end-user buyers. **Defer.**

### Concrete next step for Faraday

Move `open_order_initialize` and `close_order` out of `jupiter.rs::is_known_non_swap` (they're not v6 ix) into a new `jupiter_limit_order.rs` parser keyed off both v1 and v2 program IDs. Then add minimum-viable name-only parsers for items 2–4 above — even returning `"DCA: open_dca"` is a strict UX improvement over `"unknown action"`.
