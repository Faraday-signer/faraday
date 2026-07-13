# Faraday — Backlog (Kanban)

**This is the single Kanban board for Faraday.** Anyone — human or agent — checks here to see what to work on, in what order, and why. It is owned and kept clean by the **Project Manager agent** (`.claude/agents/project-manager.md`), grounded in [`roadmap.md`](./roadmap.md) and the live [`state.md`](./state.md).

> **The current milestone: grant readiness.** La Familia is supporting a Solana grant application and guiding the process. Before the application can move, the public-facing basics must be alive: an active X account, a real per-device cost basis, and a cared-for website. Product work (Ika approver demo) continues in parallel.

## How to use this board

- **Pick the lowest-ID, highest-priority card in `To Do`/`Backlog` that isn't blocked.** Grant-push cards take precedence while that milestone is open.
- **Read the card fully**, then read [`/CLAUDE.md`](../CLAUDE.md) before writing code.
- **Move the card** as you go: `Backlog → To Do → In Progress → In Review → Done`. Set yourself as owner when you start.
- **Definition of done** (from CLAUDE.md): own branch + conventional commits + focused PR, CI green (`cargo test`, clippy `-D warnings`, JS typechecks), acceptance criteria met, board + `state.md` updated, an entry added to `docs/updates/`.

### Legend

- **ID** — `FA-NN`, unique and never reused. Both sections share one sequence.
- **Priority** — `P0` blocker for the milestone · `P1` important · `P2` nice-to-have · `P3` someday.

---

## Board — Grant push

### 🏗 In Progress
_(none)_

### 📋 To Do
- **FA-01** `P0` — Reactivate the @faradaysigner X account
- **FA-02** `P0` — Per-device cost estimate (hardware, case, assembly, shipping, import)

### 🗂 Backlog
- **FA-03** `P0` — Grant application draft (scope + budget) — depends on FA-02
- **FA-04** `P1` — La Familia branded device batch plan — depends on FA-02
- **FA-05** `P1` — Website + public-surface care: refresh faraday.to, make email capture reliable

## Board — Product / engineering

### 🏗 In Progress
- **FA-06** `P1` — Ika clear-msig approver demo (branch `feat/ika-approver-demo`) — owner: cxalem

### 🗂 Backlog
- **FA-07** `P2` — Decide direction on the verify proposals (`docs/proposals/`)

---

## Grant push — task cards

### FA-01 `P0` — Reactivate the @faradaysigner X account
**Description:** The account has been silent for months. An active public presence is a prerequisite for the grant conversation — reviewers look at it first. Drafts already exist in the marketing pipeline (KB, `marketing/x-strategy/post-drafts/`).
**Acceptance criteria:**
- [ ] First new post published.
- [ ] Posting cadence agreed (e.g. 2/week) and the next 2 weeks of posts queued.
- [ ] Account bio/pinned post reflect the current product (signer suite, Colosseum La Familia track win).
**Owner:** —

### FA-02 `P0` — Per-device cost estimate
**Description:** A real unit-cost basis for the grant budget and for any production batch: BOM (Pi Zero 1.3, Waveshare 1.3" LCD HAT, Pi Camera v1.3, SD card, cabling), case, assembly time, packaging, shipping, import duties into Spain/EU.
**Acceptance criteria:**
- [ ] `docs/cost-estimate.md` with per-unit cost at quantities 1 / 10 / 50, sources linked.
- [ ] Case option chosen (off-the-shelf vs printed) with cost.
- [ ] Import/duty treatment confirmed for the target quantities.
**Owner:** —

### FA-03 `P0` — Grant application draft
**Description:** Draft the Solana grant application: scope, milestones, budget. La Familia is helping size it and guiding the process — iterate the draft with them.
**Depends on:** FA-02 (budget needs the cost basis), FA-01 (account must be alive before submitting).
**Acceptance criteria:**
- [ ] Draft with scope, milestones, and budget shared with La Familia for feedback.
- [ ] Their feedback incorporated; submission-ready version agreed by the team.
**Owner:** —

### FA-04 `P1` — La Familia branded device batch plan
**Description:** A small branded run for La Familia — not a hard requirement, but committed to. Define what "branded" means (case badge, boot splash, packaging), batch size, and cost delta over FA-02's baseline.
**Depends on:** FA-02.
**Acceptance criteria:**
- [ ] Branding scope defined and agreed with La Familia.
- [ ] Batch size + cost + timeline written up.
**Owner:** —

### FA-05 `P1` — Website + public-surface care
**Description:** faraday.to has been neglected. All public surfaces (site, extension store listing) need a care pass, and the email-capture flow must never silently lose signups.
**Acceptance criteria:**
- [ ] Site content/visuals reviewed and refreshed; broken or stale sections fixed.
- [ ] Email capture verified end-to-end; Supabase project confirmed on a plan/config that won't pause; failure path surfaces an error or falls back (no silent drops).
- [ ] Basic uptime/failure alerting for the capture endpoint.
**Owner:** —

## Product / engineering — task cards

### FA-06 `P1` — Ika clear-msig approver demo
**Description:** Faraday as a `clear-msig-ika` approver: firmware classifiers and zoned review shipped on `feat/ika-approver-demo`; manual devnet loop documented in `demos/ika-clear-msig-approver.md`. Remaining: land the branch, close the CLI gap on clear-msig-ika's side, run the full devnet loop, then the EVM (Sepolia) leg that proves the multi-chain payoff.
**Acceptance criteria:**
- [ ] Branch merged via PR.
- [ ] End-to-end devnet loop: proposal → QR → device clear-sign display → signature accepted on-chain.
- [ ] One EVM-target proposal approved the same way.
**Owner:** cxalem

### FA-07 `P2` — Decide direction on the verify proposals
**Description:** Two draft proposals in `docs/proposals/` (verify-protocol: device checks Helius-classified metadata against raw bytes; verify-service: signed verification reports from a Faraday verifier service). Decide which direction (if either) becomes roadmap work; fold the decision into `roadmap.md`.
**Acceptance criteria:**
- [ ] Team decision recorded (adopt / defer / drop, and why).
**Owner:** —
