# Faraday — Current State Map

**Date:** 2026-07-13 · **Repo:** https://github.com/Faraday-signer/faraday · **Default branch:** `main`

What actually exists today, so backlog cards and reviews are grounded in reality — not a fantasy. Update this file when something meaningful ships (and add an entry to [`updates/`](./updates/)).

## What it is

Open-source, air-gapped Solana signing suite. Winner of the La Familia Colosseum track (2026). The signer (Pi Zero 1.3, ~$35 all-in) has no network chip; the only I/O is light — camera scans QR in, screen displays QR out. Keys come from real-world entropy (dice, coins, camera noise), live only in RAM, wiped on power-off.

## Repo structure

| Path | What it is | Maturity |
|------|------------|----------|
| `hardware/` | Rust firmware; also runs as a desktop simulator (`just sim`). Offline tx parser (Jupiter v6/Ultra/RFQ, Raydium, DFlow, SPL/Token-2022, stake, Anchor), clear-msig-ika message/tx classifiers, risk display, BIP39 entropy flows. | Solid; the core product |
| `extension/` | Chromium Wallet Standard companion (WXT). Relays dapp signing over QR (UR animated), side panel, tx risk analyzer (`src/lib/tx-risk.ts`). Minimal `host_permissions` (PR #70). Privacy policy in place — Chrome Web Store submission prepped. | MVP done, store review pending |
| `mobile/` | React Native + Expo watch-only wallet for the Solana Seeker: pair via QR, balances, send flow with QR-relay signing, risk analyzer port. | WIP, merged (PR #64) |
| `opt/` | Buildroot recipe for the Pi OS image; release workflow publishes a pre-built image. | Working |
| `playground/` | Vite devnet dapp for exercising the extension end-to-end. | Working |
| `site/` | Next.js marketing site → faraday.to. Email capture backed by Supabase. | **Neglected — FA-05** |
| `demos/` | `ika-clear-msig-approver.md` — manual devnet loop for the Ika approver flow. | Current |
| `docs/proposals/` | Draft proposals: `verify-protocol.md`, `verify-service.md` (pre-sign verification architecture). | Drafts — FA-07 |

## In flight

- **`feat/ika-approver-demo`** (FA-06): clear-msig classifiers, hero-style zoned review, SPL symbol/decimals for known mints, bind_dwallet fixtures for 5 chains. Pushed, not yet PR'd/merged.
- **Grant push** (FA-01…FA-05): X account reactivation, cost estimate, grant draft, La Familia branded batch, website care. See [`backlog.md`](./backlog.md).

## CI

GitHub Actions: Rust (`cargo test`, clippy under `-D warnings`) + JS typechecks; release workflow builds the OS image.

## External surfaces

- **X:** [@faradaysigner](https://x.com/faradaysigner) — dormant, reactivation is FA-01.
- **Site:** [faraday.to](https://faraday.to) (Vercel) — care pass is FA-05.
- **Partners:** Ika (clear-msig approver integration, FA-06); La Familia (grant support + branded batch).
