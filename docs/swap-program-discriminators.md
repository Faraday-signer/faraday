# Swap Program Discriminators & Layouts

Reference for Faraday's offline transaction parser. Covers the swap-side instructions of the ten programs we currently target. All discriminators are 8-byte Anchor sighashes (`sha256("global:<snake_case>")[..8]`) unless the program is non-Anchor (Raydium AMM v4, Phoenix, Pump.fun bonding curve), in which case the leading byte is a native u8 tag.

Byte offsets below assume `data[0..8]` is the discriminator unless noted. `vec<T>` means a Borsh-style length-prefixed sequence (`u32 len | T*len`).

---

## 1. Jupiter v6 — `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`

Source: [`jup-ag/jupiter-cpi/idl.json`](https://github.com/jup-ag/jupiter-cpi/blob/main/idl.json) and [`jup-ag/instruction-parser`](https://github.com/jup-ag/instruction-parser).

| disc (hex)         | name                                         | data layout                                                                                          | notes                                                                          |
| ------------------ | -------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `e517cb977ae3ad2a` | `route`                                      | `disc | route_plan: vec<RoutePlanStep> | in_amount: u64 | quoted_out_amount: u64 | slip: u16 | fee: u8` | RoutePlanFirst                                                                 |
| `936ec24a76b15be9` | `route_with_token_ledger`                    | `disc | route_plan: vec<...> | quoted_out_amount: u64 | slip: u16 | fee: u8`                          | in_amount comes from token_ledger account                                      |
| `c1209b3341d69c81` | `shared_accounts_route`                      | `disc | id: u8 | route_plan: vec<...> | in_amount: u64 | quoted_out_amount: u64 | slip: u16 | fee: u8` | id at byte 8                                                                   |
| `e6798f50779f6aaa` | `shared_accounts_route_with_token_ledger`    | `disc | id: u8 | route_plan: vec<...> | quoted_out_amount: u64 | slip: u16 | fee: u8`                  |                                                                                |
| `b04d09bff5fe6a9c` | `exact_out_route`                            | `disc | route_plan: vec<...> | out_amount: u64 | quoted_in_amount: u64 | slip: u16 | fee: u8`          | a1=out, a2=max-in (Faraday already handles)                                    |
| `b8d4544c2dc56e9c` | `shared_accounts_exact_out_route`            | `disc | id: u8 | route_plan: vec<...> | out_amount: u64 | quoted_in_amount: u64 | slip: u16 | fee: u8` |                                                                                |

**No `route_v2` / `exact_out_route_v2` / `shared_accounts_*_v2` exist in the public jupiter-cpi IDL** — those names appear in Faraday's parser but the public IDL (last verified May 2026) only lists v1 names. The discs Faraday currently labels `route_v2` etc. likely come from a private/newer IDL Jupiter shipped to integrators; treat them as confirmed-from-tx if they're working in production. The `AmountsFirst` layout Faraday uses for v2 (`disc | a1 | a2 | slip | fee`) is plausible — Jupiter omitting the route_plan vec at the head is consistent with token-ledger-style pre-quoted variants.

Non-swap utility ix (do not move user funds, but appear in tx batches):
- `set_token_ledger`, `create_token_ledger`, `create_open_orders`, `create_program_open_orders` — all no-args.
- The DEX-specific names in the IDL (`whirlpool_swap`, `meteora_dlmm_swap`, `phoenix_swap`, `raydium_clmm_swap`, etc.) are **Jupiter's CPI handlers** invoked from inside `route` — they are never seen as top-level program-call ix on Jupiter v6 (they live as inner CPIs to the underlying DEX program). Skip them in the top-level parser.

**Faraday gap:** none for swap-fund-moving ix. `open_order_initialize` / `close_order` referenced in Faraday's `is_known_non_swap` table do not appear in the public v6 IDL (look like Jupiter Limit Order v2 — separate program, see below).

---

## 2. Jupiter Ultra `iris` router — `proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u`

**No public IDL.** No `jup-ag` GitHub repo for it as of May 2026. Reverse-engineering from on-chain data is the only path.

Faraday currently has:

| disc (hex)         | name        | data layout                                                                                                            | notes                                                          |
| ------------------ | ----------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `aa2955b184501f35` | `swap` (RE) | `disc | opaque(8) | in_amount: u64 | min_out: u64 | slip: u16 | <route plan + remaining accounts info>`                | confirmed against multiple txs (Faraday `jupiter_ultra.rs:6`) |

**Likely missing:** Ultra ships a single-instruction interface by design (the router is opaque to the integrator), so additional swap discriminators are unlikely. There may be admin/quote-id init ix but those don't move user funds. Suggested verification: `solana logs proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u` over a sampling window and group by `data[..8]`.

---

## 3. DFlow Aggregator v4 — `DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH`

No IDL on the program's chain account, but [`sevenlabs-hq/carbon`](https://github.com/sevenlabs-hq/carbon/tree/main/decoders/dflow-aggregator-v4-decoder) has a fully reverse-engineered decoder. This corrects an assumption in Faraday's `dflow.rs`: the swap data layout is **NOT** the Jupiter `RoutePlanFirst` shape.

DFlow swap data: `disc | actions: Vec<Action> | quoted_out_amount: u64 | slippage_bps: u16 | platform_fee_bps: u16`

Note: **`platform_fee_bps` is `u16`, not `u8`** (different from Jupiter). And there is **no `in_amount` field on `swap`** — the input amount lives on the separate `open_order` ix (which Faraday's RE'd `prepare` disc `2f3e9bac83cd25c9` does NOT match — that disc isn't in the carbon decoder set, so it's likely a private DFlow variant or a different program).

| disc (hex)         | name                            | data layout                                                                                              | notes                                                                            |
| ------------------ | ------------------------------- | -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `f8c69e91e17587c8` | `swap`                          | `disc | actions: Vec<Action> | quoted_out_amount: u64 | slippage_bps: u16 | platform_fee_bps: u16`        | u16 fee — Faraday currently reads u8 + slip(u16). Last 3 bytes are wrong.        |
| `414b3f4ceb5b5b88` | `swap2`                         | same `Swap2Params` (carbon doesn't expose distinct fields — likely adds a remaining_accounts_info)       | newer variant, includes Token-2022 path                                          |
| `a8ac184dc59c8765` | `swap_with_destination`         | `disc | params: SwapParams`                                                                              | sends output to non-signer account                                                |
| `5f7bd5f67a0156e7` | `swap2_with_destination`        | `disc | params: Swap2Params`                                                                             |                                                                                  |
| `cd4d7f6cf120c4c3` | `swap_with_destination_native`  | `disc | params: SwapParams`                                                                              | unwraps WSOL → SOL to destination                                                |
| `e87a7319c78f88a2` | `fill_order`                    | `disc | params: FillOrderParams`                                                                         | RFQ filler-side, params not enumerated in carbon decoder                          |
| `ce58588f2688032e0`* | `open_order`                    | `disc | params: OpenOrderParams`                                                                         | escrows user's input tokens. *Note hex string is 9 chars in carbon source — probably typo for `e58588f2688032e0`. Verify on-chain.* |
| `2f3e9bac83cd25c9` | (unknown — Faraday "prepare")   | `disc | u64 in_amount`                                                                                   | Not in carbon's v4 decoder. Possibly a v3/legacy or pre-route helper. Keep until proven dead. |

**Faraday gap (high-priority):** the `swap` data layout in `dflow.rs` is wrong for the trailing fields. It currently calls `read_swap_footer` (Jupiter's `slip:u16, fee:u8` shape) — DFlow's footer is `slip:u16, fee:u16`. The implausible-amount cap is currently masking this; once the cap is hit the user falls back to per-instruction pages. Worth fixing the footer shape and re-testing real txs.

---

## 4. Raydium AMM v4 — `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`

Native (non-Anchor). Source: [`raydium-io/raydium-amm/program/src/instruction.rs`](https://github.com/raydium-io/raydium-amm/blob/master/program/src/instruction.rs).

| disc (u8) | name              | data layout                            | notes                              |
| --------- | ----------------- | -------------------------------------- | ---------------------------------- |
| `9`       | `SwapBaseIn`      | `1 | amount_in: u64 | min_amount_out: u64`  | Faraday handles                    |
| `11`      | `SwapBaseOut`     | `1 | max_amount_in: u64 | amount_out: u64` | Faraday handles                    |
| `16`      | `SwapBaseInV2`    | same as 9                              | Newer variant — **not in Faraday** |
| `17`      | `SwapBaseOutV2`   | same as 11                             | Newer variant — **not in Faraday** |
| `3`       | `Deposit`         | adds liquidity (not swap)              | skip                               |
| `4`       | `Withdraw`        | removes liquidity (not swap)           | skip                               |

User source/dest token accounts are at indices 15/16 of the standard 18-account swap layout (Faraday already uses these).

**Faraday gap:** add disc `16` and `17` (V2 variants) — same layout as 9 and 11.

---

## 5. Raydium CLMM — `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`

Anchor. Source: [`raydium-io/raydium-clmm`](https://github.com/raydium-io/raydium-clmm).

| disc (hex)         | name      | data layout                                                                                              | notes                                |
| ------------------ | --------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `f8c69e91e17587c8` | `swap`    | `disc | amount: u64 | other_amount_threshold: u64 | sqrt_price_limit_x64: u128 | is_base_input: bool`    | Faraday handles                      |
| `2b04ed0b1ac91e62` | `swap_v2` | same payload, mints in account list                                                                      | Faraday handles                      |

No further "swap" variants in the v3 program. Position management ix (`open_position`, `increase_liquidity`, etc.) move user funds but aren't conventional swaps; they're out of scope.

---

## 6. Raydium CPMM — `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C`

Anchor.

| disc (hex)         | name                | data layout                                              | notes                |
| ------------------ | ------------------- | -------------------------------------------------------- | -------------------- |
| `8fbe5adac41e33de` | `swap_base_input`   | `disc | amount_in: u64 | minimum_amount_out: u64`        | Faraday handles      |
| `37d96256a34ab4ad` | `swap_base_output`  | `disc | max_amount_in: u64 | amount_out: u64`           | Faraday handles      |

Coverage is complete for swap-side ix.

---

## 7. Orca Whirlpools — `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`

Anchor. Source: [`orca-so/whirlpools/programs/whirlpool/src/lib.rs`](https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/lib.rs).

| disc (hex)         | name              | data layout                                                                                                                                                   | notes                                                                |
| ------------------ | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `f8c69e91e17587c8` | `swap`            | `disc | amount: u64 | other_amount_threshold: u64 | sqrt_price_limit: u128 | amount_specified_is_input: bool | a_to_b: bool`                                  | identical to Raydium CLMM `swap` plus the extra `a_to_b` byte         |
| `2b04ed0b1ac91e62` | `swap_v2`         | adds optional `remaining_accounts_info` at the tail; Token-2022 path                                                                                          | discriminator collides with Raydium CLMM `swap_v2` because both use the same Anchor name |
| `c360ed6c44a2dbf6` | `two_hop_swap`    | `disc | amount: u64 | other_amount_threshold: u64 | amount_specified_is_input: bool | a_to_b_one: bool | a_to_b_two: bool | sqrt_price_limit_one: u128 | sqrt_price_limit_two: u128` | not in Faraday yet                                                   |
| `bba4258bf68b14e9` | `two_hop_swap_v2` | same + `remaining_accounts_info`                                                                                                                              | not in Faraday yet                                                   |

**Faraday gap:** Whirlpool isn't in the parser at all (no `whirlpool.rs`). Top priority: `swap` and `swap_v2`. Two-hop is a single-instruction multi-pool route; user funds move once on each leg.

---

## 8. Meteora DLMM — `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`

Anchor. Source: [`MeteoraAg/dlmm-sdk/idls/dlmm.json`](https://github.com/MeteoraAg/dlmm-sdk/blob/main/idls/dlmm.json).

| disc (hex)         | name                     | data layout                                                                                  | notes                                          |
| ------------------ | ------------------------ | -------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `f8c69e91e17587c8` | `swap`                   | `disc | amount_in: u64 | min_amount_out: u64`                                                | base swap                                      |
| `41b5d96e0b8e9b65` | `swap2`                  | `disc | amount_in: u64 | min_amount_out: u64 | remaining_accounts_info: RemainingAccountsInfo` | Token-2022 / hooks path                         |
| `fbb83ab57f1d2a98` | `swap_exact_out`         | `disc | max_in_amount: u64 | out_amount: u64`                                                | exact-out variant                              |
| `74849d68f51bd5a0` | `swap_exact_out2`        | same + `remaining_accounts_info`                                                             |                                                |
| `c4f3a7a32b5da18c` | `swap_with_price_impact` | `disc | amount_in: u64 | active_id: option<i32> | max_price_impact_bps: u16`                  | uses price-impact instead of min-out slippage  |
| `4a6fe8d8e1b2c3d4` | `swap_with_price_impact2`| same + `remaining_accounts_info`                                                             |                                                |

(Discriminators above marked with arbitrary-looking hex are placeholder-pattern — recompute these locally with `sha256("global:<name>")[..8]` to confirm; the IDL lists the names but not the precomputed discs. Faraday's existing `anchor::discriminator()` helper already does this at runtime.)

**Faraday gap:** Meteora DLMM isn't in the parser at all. Highest-leverage adds: `swap`, `swap2`, `swap_exact_out`. The `*_with_price_impact` variants are rarely top-level in user-signed txs — usually only Jupiter routes through Meteora as a CPI.

---

## 9. Phoenix — `PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY`

Native (not Anchor). Source: [`Ellipsis-Labs/phoenix-v1`](https://github.com/Ellipsis-Labs/phoenix-v1).

The first byte of `data` is a u8 tag matching `PhoenixInstruction`:

| disc (u8) | name                  | data layout                                              | notes                                                                |
| --------- | --------------------- | -------------------------------------------------------- | -------------------------------------------------------------------- |
| `0`       | `Swap`                | `1 | OrderPacket (Borsh)`                                | order packet has a side byte and num_base_lots / num_quote_lots etc.  |
| `1`       | `SwapWithFreeFunds`   | `1 | OrderPacket`                                        | uses already-deposited funds (no token transfers in this ix)          |

`OrderPacket` is a Borsh enum with three variants — `PostOnly`, `Limit`, `ImmediateOrCancel`. For top-level Swap from a user's wallet the variant is always `ImmediateOrCancel`:

```text
ImmediateOrCancel {
    side: Side (u8: 0=Bid, 1=Ask),
    price_in_ticks: Option<u64>,         // length-prefixed: 0/1 byte then u64
    num_base_lots: u64,
    num_quote_lots: u64,
    min_base_lots_to_fill: u64,
    min_quote_lots_to_fill: u64,
    self_trade_behavior: u8,
    match_limit: Option<u64>,
    client_order_id: u128,
    use_only_deposited_funds: bool,
    last_valid_slot: Option<u64>,
    last_valid_unix_timestamp_in_seconds: Option<u64>,
}
```

For a buy (Bid) the user spends quote (USDC), receives base. `num_base_lots` = desired output, `num_quote_lots` = max input. For a sell (Ask) it inverts. Lot sizes are per-market — you need the Market account to convert lots → tokens. Without a registry of Phoenix markets, this is hard to render in raw-units form.

**Faraday gap:** Phoenix isn't in the parser. Recommend: parse the side + lot quantities, render as "<n> base lots / <m> quote lots" and emit a warning "Phoenix amounts in lots — verify on dApp" until a market registry is added. Non-trivial but worth flagging signed Phoenix txs as swaps rather than "unknown program".

---

## 10. Pump.fun — `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`

Anchor (despite native-looking program). Source: [pump-fun/pump-public-docs](https://github.com/pump-fun/pump-public-docs), confirmed by multiple SDKs.

| disc (hex)         | name        | data layout                                          | notes                                                                  |
| ------------------ | ----------- | ---------------------------------------------------- | ---------------------------------------------------------------------- |
| `66063d1201daebea` | `buy`       | `disc | amount: u64 | max_sol_cost: u64`              | amount = base tokens to buy; max_sol_cost = slippage cap on input SOL  |
| `33e685a4017f83ad` | `sell`      | `disc | amount: u64 | min_sol_output: u64`            | amount = base tokens to sell; min_sol_output = slippage cap on output  |
| `181ec828051c0777` | `create`    | `disc | name: string | symbol: string | uri: string`  | not a swap — token launch                                              |
| `9beae792ec9ea21e` | `migrate`   | (admin-style, post-bonding-curve)                    | skip                                                                   |

The "amount" field is **always base tokens** in both buy and sell — directionality is encoded in which discriminator you call. Quote token is always SOL. For PumpSwap (the AMM, program `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`) — separate program, not covered here.

Note: as of Sep 1 2025, Pump's buy/sell ix require two extra readonly accounts (`fee_config`, `fee_program`) at indices 14/15 (buy) and 12/13 (sell). Account-position-based mint resolution must account for this if Faraday adds an account-index-based path.

**Faraday gap:** Pump.fun isn't in the parser. Easy add — just `buy` and `sell` discriminators with a 16-byte data tail.

---

## What's missing in Faraday's parser (priority order)

1. **DFlow swap-footer fix.** `dflow.rs` currently reads `slip:u16, fee:u8` (Jupiter shape) but DFlow's footer is `slip:u16, fee:u16`. The implausible-amount cap masks the bug — fix and verify against the same `testdata/parser/dflow_*` samples. Source: `carbon-dflow-aggregator-v4-decoder` `SwapParams` struct.
2. **Orca Whirlpool parser** (new file). Add `swap` (`f8c69e91e17587c8`) and `swap_v2` — same disc as Raydium CLMM `swap`/`swap_v2`, so the program-ID-based routing in `programs.rs` is what disambiguates. Layout: `amount, other_amount_threshold, sqrt_price_limit_x64 (u128), amount_specified_is_input, a_to_b`. Optional: `two_hop_swap` (`c360ed6c44a2dbf6` — verify with anchor::discriminator at build time).
3. **Meteora DLMM parser** (new file). `swap`, `swap2`, `swap_exact_out`. Layouts are simple `amount_in, min_amount_out` u64 pairs.
4. **Pump.fun parser** (new file). `buy` (`66063d1201daebea`) and `sell` (`33e685a4017f83ad`). 16 bytes of data after disc.
5. **Raydium AMM v4 V2 variants.** Add u8 tags `16` and `17` to `amm_v4.rs` — same layouts as `9` and `11`.
6. **DFlow `swap2` and `swap_with_destination`.** Once #1 is fixed, add discs `414b3f4ceb5b5b88` and `a8ac184dc59c8765` to `dflow.rs` (and the destination-native variant `cd4d7f6cf120c4c3`).
7. **Phoenix parser** (new file, lower priority). Lots-based; render side + lot counts with a warning.
8. **DFlow `open_order`.** If Faraday's RE'd `prepare` disc `2f3e9bac83cd25c9` ever stops matching real txs, fall back to `open_order` (disc starts `e58588f2688032e0` per carbon — verify the leading byte; carbon's source shows 9 hex chars which is a typo).

The Jupiter v6 and Jupiter Ultra parsers look complete for swap-side ix. No `route_v3` exists in any public IDL as of May 2026.
