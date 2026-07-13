# Faraday — Roadmap

Milestones and locked-in decisions. The backlog ([`backlog.md`](./backlog.md)) is the executable view; this is the why and the order.

## M1 — Grant readiness (now)

La Familia is supporting a Solana grant application and guiding the process. Everything a reviewer touches must be alive first:

1. **X account reactivated** with a steady cadence (FA-01).
2. **Per-device cost basis** — hardware, case, assembly, shipping, import (FA-02). Feeds the grant budget and any batch.
3. **Grant application drafted and iterated with La Familia** (FA-03).
4. **Website + email capture cared for** — no dead surfaces, no lost signups (FA-05).

Exit criteria: application submitted, public surfaces presentable.

## M2 — Ika approver demo shipped

Faraday as a `clear-msig-ika` approver — the multi-chain co-signer story (FA-06). Firmware classifiers and review UI are on `feat/ika-approver-demo`; remaining work is landing the branch, the full devnet loop, and one EVM-target approval. A 90-second demo ("approve an ETH transfer on a $35 Pi with no antenna") doubles as grant/marketing material.

## M3 — La Familia branded batch

A small branded production run (FA-04): validates the FA-02 cost model, gives La Familia devices to show, and is the first real manufacturing exercise.

## Later

- Verify protocol/service — pre-sign verification architecture (proposals in [`proposals/`](./proposals/), decision is FA-07).
- Ikavery roster membership (integration map A2) and the inkrypto-on-Pi spike (C1) — after the approver demo ships.
- Mobile parity for the approver flow; firmware hardening; signing policies.

## Decisions

- **Rulebook:** `/CLAUDE.md` — think-first, simplicity, surgical changes, conventional commits, branch-per-PR.
- **Board process:** LaPropia-style — PM agent owns `docs/backlog.md`; every change logs one file in `docs/updates/`; `state.md` tracks reality.
- **Threat-model language:** never "secure" in the abstract — name the property. The LOAD flow is restore-only, never seed migration.
- **Air-gap invariant:** `hardware/` never gains network dependencies; the device displays only what it derives from the raw bytes it signs.
