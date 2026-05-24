# Ika clear-msig fixture QRs

Synthetic QRs that exercise every code path Faraday gained for the
`clear-msig-ika` integration. Regenerate with:

```bash
just ika-fixtures   # or: cargo run --features simulator --bin gen-ika-fixtures
```

The signer is the canonical BIP39 test vector (`abandon × 11 + about`),
account 0, no passphrase → `HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk`.
Load that mnemonic on the simulator first (Load Wallet → Enter Words).
**All signatures and blockhashes are placeholders. These are demo-only.**

## How to use

The `.png` QRs aren't checked in (they're generated artifacts). Run
`just ika-fixtures` first to materialize them next to the `.bin` files.
Then display any `.png` on your Mac screen (Preview at 100% works) and
scan it from the simulator's Sign flow. The iPhone Continuity Camera
works well at index 2 (`FARADAY_CAMERA_INDEX=2 cargo run --features simulator`).

## Message-signing fixtures (Sign Message flow)

These exercise the off-chain message classifier in `gui/screens.rs`.

| File | Expected device review (top of screen) |
|---|---|
| `msg_approve_transfer.png` | **APPROVE TRANSFER** — IKA PROPOSAL 42, AMOUNT 1 SOL, TO HAgk…Kpqk, WALLET treasury |
| `msg_propose_transfer.png` | **PROPOSE TRANSFER** — IKA PROPOSAL 43, AMOUNT 0.5 SOL |
| `msg_cancel_transfer.png` | **CANCEL TRANSFER** — IKA PROPOSAL 42 |
| `msg_approve_spl.png` | **APPROVE SPL TRANSFER** — AMOUNT 1500000, MINT EPjF…Dt1v |
| `msg_approve_add_intent.png` | **APPROVE ADD INTENT** — HASH 012345…abcdef |
| `msg_approve_remove_intent.png` | **APPROVE REMOVE INTENT** — INDEX 3 |
| `msg_approve_update_intent.png` | **APPROVE UPDATE INTENT** — INDEX 2, HASH deadbe…abcdef |
| `msg_btc_fallback.png` | **APPROVE ACTION** — TEXT "send 12345 sats to bc1q-pkh:0xdeadbeef from utxo 0xabcd:0" (fallback path for unknown content shapes) |

## Transaction-signing fixtures (Sign TX flow)

These exercise `parser/ika.rs`. Each tx contains exactly one Ika program
instruction with synthetic account slots (`0x21..0x2c`) so the device
shows distinct shortened pubkeys for each role.

| File | Disc | Expected device review header |
|---|---|---|
| `tx_create_wallet.png` | 0 | **Ika create wallet** — Approval thr. 2, Cancel thr. 1, Timelock 1h |
| `tx_propose.png` | 1 | **Ika propose** — Proposal # 7, Params 24 bytes |
| `tx_approve.png` | 2 | **Ika approve** — Approver #3, Expires (unix) 1893456000 |
| `tx_cancel.png` | 3 | **Ika cancel** — Canceller #5 |
| `tx_execute.png` | 4 | **Ika execute** — Wallet, Vault, Intent, Proposal |
| `tx_cleanup.png` | 5 | **Ika cleanup proposal** — Proposal, Rent refund |
| `tx_bind_dwallet.png` | 6 | **Ika bind dWallet** — Chain Solana, dWallet AAAA…AAAA |
| `tx_ika_sign.png` | 7 | **Ika sign (MPC)** — dWallet, Hash 1 a1a1a1…a1a1a1, Hash 2 b2b2…, Hash 3 c3c3… |

## What's in the .bin files

Each PNG has a sibling `.bin` with the underlying raw bytes:

- `msg_*.bin` — the wrapped Solana off-chain message (the bytes the device
  signs over; **without** the outer 0xFF transport byte).
- `tx_*.bin` — the unsigned legacy Solana tx bytes (suitable for unit tests).
