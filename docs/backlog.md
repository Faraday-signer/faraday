# Faraday — Backlog (Kanban)

**This is the single Kanban board for Faraday.** Anyone — human or agent — checks here to see what to work on, in what order, and why. It is owned and kept clean by the **Project Manager agent** (`.claude/agents/project-manager.md`), grounded in [`roadmap.md`](./roadmap.md) and the live [`state.md`](./state.md).

> **The current milestone: grant readiness.** La Familia is supporting a Solana grant application and guiding the process. Before the application can move, the public-facing basics must be alive: an active X account, a real per-device cost basis, and a cared-for website. Product work (Ika approver demo) continues in parallel.

## How to use this board

- **Pick the lowest-ID, highest-priority card in `To Do`/`Backlog` that isn't blocked *and isn't claimed*.** Grant-push cards take precedence while that milestone is open.
- **Read the card fully**, then read [`/CLAUDE.md`](../CLAUDE.md) before writing code.
- **Move the card** as you go: `Backlog → To Do → In Progress → In Review → Done`. Set yourself as owner when you start.
- **Definition of done** (from CLAUDE.md): own branch + conventional commits + focused PR, CI green (`cargo test`, clippy `-D warnings`, JS typechecks), acceptance criteria met, board + `state.md` updated, an entry added to `docs/updates/`.

### Claiming a card (how we don't step on each other)

Several people work this repo in parallel, and this file only syncs when PRs merge — so **the board is not the claim. The open PR is the claim.**

1. **Before picking a card:** `git fetch`, then check `gh pr list --state open` and remote branches. A card whose ID appears in an open branch or PR title is taken, whatever the board says.
2. **To claim:** create your branch (`type/short-description`), make your *first* commit the board edit (card → In Progress, owner = you), push, and **open a draft PR immediately** titled with the card ID (e.g. `feat(site): FA-05 email-capture hardening`). From that moment the claim is visible to everyone in real time — no main commits, no race on this file.
3. **Abandoning:** close the draft PR and move the card back in a small PR (or ask the PM agent to). A draft PR with no commits for a week is fair game — ping the owner first.
4. **One card, one branch, one PR.** If your work grows past the card, that's a new card, not a bigger PR.

Merge conflicts on this file are further defused by `.gitattributes` (`merge=union` on the shared docs) — but the protocol above is what actually prevents duplicate effort.

### Legend

- **ID** — `FA-NN`, unique and never reused. Both sections share one sequence.
- **Priority** — `P0` blocker for the milestone · `P1` important · `P2` nice-to-have · `P3` someday.

---

## Board — Grant push

### 🏗 In Progress
_(none)_

### 🔬 In Review
- **FA-16** `P1` — Landing page redesign (faraday.to) — dark instrument-panel rebuild; flasher (FA-13) slot + `/flash` stub and extension-store (FA-08) slot reserved (branch `feat/landing-redesign`, PR #117, **draft — awaiting cxalem design approval on the Vercel preview before merge**) — owner: cxalem

### 📋 To Do
- **FA-01** `P0` — Reactivate the @faradaysigner X account
- **FA-02** `P0` — Per-device cost estimate, dual BOM: Pi Zero **and** ESP32-S3 (hardware, case, assembly, shipping, import)

### 🗂 Backlog
- **FA-03** `P0` — Grant application draft (scope + budget) — depends on FA-02
- **FA-04** `P1` — La Familia branded device batch plan — depends on FA-02
- **FA-05** `P1` — Email-capture reliability on the site (narrowed — the redesign itself is FA-16)
- **FA-11** `P2` — X content plan (pillars, cadence, 4 weeks of drafts) — depends on FA-01

## Board — Product / engineering

### 🏗 In Progress
- **FA-06** `P1` — Ika clear-msig approver demo (branch `feat/ika-approver-demo`, PR #71) — owner: cxalem
- **FA-18** `P1` — Telegram board sync: mirror task claims to the "Faraday Signal" channel (branch `feat/telegram-board`) — owner: cxalem
- **FA-23** `P1` — A sign button you can't miss on the TX review (branch `feat/sign-button`) — owner: cxalem
- **FA-24** `P1` — Lookup-table txs sign normally (branch `feat/sign-button`, PR #127) — owner: cxalem
- **FA-22** `P2` — Camera-entropy capture feedback (branch `feat/camera-entropy-feedback`) — owner: cxalem
- **FA-21** `P2` — Touch UX: press feedback + honest footer (branch `feat/touch-feedback-footer`) — owner: cxalem

### 🔬 In Review
- **FA-09** `P1` — Durable-nonce transactions: signed QR-relayed txs must not expire (branch `feat/durable-nonce`, PR #112) — owner: cxalem
- **FA-20** `P2` — IMU support + shake-to-theme Easter egg on the ESP32-S3 (branch `feat/imu-shake-theme`) — owner: cxalem

### 📋 To Do
- **FA-08** `P1` — Publish the Chrome extension to the Web Store (permissions rework + listing) — owner: Trskel (Javi Lois)

### 🗂 Backlog
- **FA-07** `P2` — Decide direction on the verify proposals (`docs/proposals/`)
- **FA-10** `P2` — QR scan latency: benchmark Pi vs ESP32, then reach parity or better
- **FA-17** `P1` — ESP32-S3 release artifact: versioned firmware image + ESP Web Tools manifest from the release workflow — unblocks FA-13
- **FA-13** `P1` — Web-based firmware flasher (ESP32-S3 / ESP Web Tools) — lands within the FA-16 site; depends on FA-16, FA-17, and ESP32 support (PRs #73/#93)
- **FA-14** `P2` — Optimize CI wall-clock time
- **FA-15** `P2` — Mobile app **epic** — first child card: scoping spike
- **FA-19** `P2` — Durable-nonce for the mobile send flow — follow-up to FA-09 (extension only)
- **FA-12** `P3` — Faraday MCP server *(idea — unshaped; needs a proposal before any build card)*
- **FA-25** `P2` — Verify-backup falsely reports "MISMATCH" on a decode failure, not just a real content mismatch

> **New-work ordering (team direction, 2026-07-20):** FA-16 (landing redesign, with FA-13 inside it; FA-17 unblocks the flasher) → FA-08 (extension publish) → FA-09 (durable nonce) → then FA-10 / FA-14 / FA-11. FA-12 stays iced; FA-15 gets scoped but is not a near-term priority.

---

## Grant push — task cards

### FA-01 `P0` — Reactivate the @faradaysigner X account
**Description:** The account has been silent for months. An active public presence is a prerequisite for the grant conversation — reviewers look at it first. Drafts already exist in the marketing pipeline (KB, `marketing/x-strategy/post-drafts/`).
**Acceptance criteria:**
- [ ] First new post published.
- [ ] Posting cadence agreed (e.g. 2/week) and the next 2 weeks of posts queued.
- [ ] Account bio/pinned post reflect the current product (signer suite, Colosseum La Familia track win).
**Owner:** —

### FA-02 `P0` — Per-device cost estimate (dual BOM: Pi Zero and ESP32-S3)
**Description:** A real unit-cost basis for the grant budget and for any production batch. Scoped to **both device variants** (team decision 2026-07-20, since the ESP32-S3 migration is in flight): the Pi Zero build (Pi Zero 1.3, Waveshare 1.3" LCD HAT, Pi Camera v1.3, SD card, cabling) **and** the ESP32-S3 build (module/dev board, display, camera, cabling). Each variant: BOM, case, assembly time, packaging, shipping, import duties into Spain/EU.
**Acceptance criteria:**
- [ ] `docs/cost-estimate.md` with per-unit cost at quantities 1 / 10 / 50 for **both** the Pi Zero and ESP32-S3 variants, sources linked.
- [ ] Case option chosen (off-the-shelf vs printed) with cost, per variant.
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

### FA-05 `P1` — Email-capture reliability on the site
**Description:** *(Narrowed 2026-07-20: the site content/visual refresh moved into the FA-16 redesign — keeping both would duplicate work.)* The email-capture flow must never silently lose signups, whatever the site looks like.
**Acceptance criteria:**
- [ ] Email capture verified end-to-end; Supabase project confirmed on a plan/config that won't pause; failure path surfaces an error or falls back (no silent drops).
- [ ] Basic uptime/failure alerting for the capture endpoint.
**Owner:** —

### FA-16 `P1` — Landing page redesign (faraday.to)
**Description:** Team direction (2026-07-20): we need a better website — a real redesign of the landing page in `site/` (Next.js, Vercel), not just a care pass. This is the **top of the new-work stack**. The site must tell the current product story (air-gapped signer suite, extension, mobile, Ika approver demo, Colosseum La Familia track win) and it will **include the web flasher** (FA-13) as a page of the new site. Check the `chore/brand-assets` branch for existing visual-identity work before starting from scratch.
**Plan:**
1. Content inventory of the current site: what's stale, missing, or wrong against `state.md`'s product reality.
2. One round of design direction (wireframe/mockup or a deploy preview) approved by **cxalem (Alejandro)** — the named design approver (team decision 2026-07-20) — before the full build; the redesign's look is a human call, not something to derive silently.
3. Rebuild the landing page in `site/`: hero + product story, device and extension sections, space for demo material (the 90-second Ika approver clip), and navigation slots reserved for the flasher page (FA-13) and the extension store listing link (FA-08) so neither needs a second redesign to land.
4. Keep the email-capture flow working untouched (its reliability hardening is FA-05, a separate card).
5. Verify: site builds green in CI, OpenGraph/meta correct (this regressed before — see `fix/site-opengraph-build`), team reviews the deploy preview before merge.
**Acceptance criteria:**
- [ ] Redesigned landing page live at faraday.to with current product content.
- [ ] Design direction approved by cxalem (Alejandro) on a preview before merging.
- [ ] Email capture still works end-to-end (FA-05's scope is not regressed).
- [ ] Navigation accommodates the upcoming flasher page (FA-13) and extension listing link (FA-08).
- [ ] Site build + typecheck green; OpenGraph/meta verified.
**Owner:** cxalem (branch `feat/landing-redesign`; per the claiming protocol the claim becomes visible when the draft PR titled `FA-16` opens)

### FA-11 `P2` — X content plan
**Description:** A sustained content plan for posting about Faraday on X, beyond FA-01's reactivation burst (FA-01 gets the account alive with two weeks queued; this card is the ongoing engine). Define content pillars (air-gapped signing explainers, build-in-public firmware notes, demo clips — the 90-second Ika approver demo doubles as material), formats, and cadence, drawing on the existing drafts in the marketing pipeline (KB, `marketing/x-strategy/post-drafts/`).
**Depends on:** FA-01 (account must be alive; cadence agreed there is the input here).
**Acceptance criteria:**
- [ ] Written plan: 3–5 content pillars, cadence, and formats (text, clips, threads), recorded in the marketing KB or `docs/`.
- [ ] Four further weeks of posts drafted and queued past FA-01's initial two.
- [ ] A stated way to judge traction (what metric matters for the grant narrative — e.g. follower growth, demo-clip views), so the plan can be adjusted rather than abandoned.
**Owner:** —

## Product / engineering — task cards

### FA-06 `P1` — Ika clear-msig approver demo
**Description:** Faraday as a `clear-msig-ika` approver: firmware classifiers and zoned review shipped on `feat/ika-approver-demo`; manual devnet loop documented in `demos/ika-clear-msig-approver.md`. Remaining: land the branch, close the CLI gap on clear-msig-ika's side, run the full devnet loop, then the EVM (Sepolia) leg that proves the multi-chain payoff.
**Acceptance criteria:**
- [ ] Branch merged via PR.
- [ ] End-to-end devnet loop: proposal → QR → device clear-sign display → signature accepted on-chain.
- [ ] One EVM-target proposal approved the same way.
**Owner:** cxalem

### FA-18 `P1` — Telegram board sync: mirror task claims to the "Faraday Signal" channel
**Description:** The team coordinates on Telegram; the board file only syncs on merge. A private read-only channel ("Faraday Signal") now exists with bot **@faraday_board_bot** as its only poster: a pinned "who's working on what" message plus a one-line post per claim/finish, so nobody starts a card someone else is on. This card lands the repo side: the posting script, the shared config template, the doc, and the agent/rulebook wiring so every contributor's agents post automatically.
**Plan:** add `scripts/tg-board.sh` (`post` / `read-pin` / `update-pin`, sourcing a gitignored `.env`, posts stamped with `git config user.name`); commit `.env.example` (chat id + pin message id, token blank — shared privately); write `docs/telegram-board.md` (setup, conventions, token hygiene); wire the conventions into `.claude/agents/project-manager.md` (read pin before recommending, refresh after board changes) and `builder.md` (post on claim in step 2, on review-ready in step 7) plus a `CLAUDE.md` section; un-ignore `.claude/agents/` so the team shares the agent definitions.
**Acceptance criteria:**
- [ ] `scripts/tg-board.sh post|read-pin|update-pin` work against the live channel from a clean checkout + `.env`.
- [ ] Missing `.env` fails with a clear setup message and never blocks repo work (agents continue without it).
- [ ] `docs/telegram-board.md` covers setup, conventions, and token rotation; `.env.example` committed; the real token appears nowhere in the repo or PR.
- [ ] PM and builder agent definitions + `CLAUDE.md` instruct the pin-check before starting and the post+pin-refresh on claim/finish.
**Owner:** cxalem

### FA-07 `P2` — Decide direction on the verify proposals
**Description:** Two draft proposals in `docs/proposals/` (verify-protocol: device checks Helius-classified metadata against raw bytes; verify-service: signed verification reports from a Faraday verifier service). Decide which direction (if either) becomes roadmap work; fold the decision into `roadmap.md`.
**Acceptance criteria:**
- [ ] Team decision recorded (adopt / defer / drop, and why).
**Owner:** —

### FA-08 `P1` — Publish the Chrome extension to the Web Store
**Description:** The extension (WXT, MV3) is MVP-done and store submission is prepped (`extension/PRIVACY_POLICY.md` exists), but the content script matches `<all_urls>` (`extension/wxt.config.ts`), which slows Chrome Web Store review and reads badly on the listing. **The permissions approach is deliberately undecided — UX is the deciding criterion.** Candidate: `activeTab` + `optional_host_permissions` with dynamic content-script registration per site (viable because `inpage.ts` uses wallet-standard `registerWallet()`, which supports late registration). If that matches the `<all_urls>` UX — only the first visit to a site should cost one icon click — ship it; if it degrades UX, ship `<all_urls>` and accept the slower review. This card must not pre-commit either way. Check the existing `chore/drop-host-permissions` branch for partial work before starting.
**Plan:**
1. Reconcile with prior art: review the `chore/drop-host-permissions` branch and PR #70's minimal `host_permissions` change; fold anything usable in rather than redoing it.
2. **UX spike + decision (explicit step, before any submission work):** prototype the optional-permissions flow — remove the `<all_urls>` match, declare `optional_host_permissions`, register the content script dynamically (`chrome.scripting.registerContentScripts`) once the user grants a site (activeTab invocation or a grant from the side panel), persisting granted origins across restarts. Evaluate side-by-side against `<all_urls>` on the playground dapp and at least one real dapp: first-visit cost, reconnect after restart, revocation. **Decision rule: UX parity → optional-permissions approach; UX degraded → ship `<all_urls>`.** Record the decision and rationale in the PR and in `docs/updates/`.
3. Verify wallet discovery under the chosen approach: wallet announces via wallet-standard on playground + a real dapp; manual matrix of fresh install, first connect, restart, revocation.
4. Prepare the listing: copy, screenshots, category, data-use disclosures consistent with `extension/PRIVACY_POLICY.md`, privacy-policy URL (host on faraday.to — coordinate with FA-16/FA-05).
5. Register the developer account (one-time fee), submit, respond to review feedback; on approval, record the listing URL in `state.md` and link it from the site.
**Acceptance criteria:**
- [ ] The spike ran and the permissions decision (optional-permissions vs `<all_urls>`) is recorded with its UX rationale — not defaulted silently.
- [ ] Shipped manifest matches the recorded decision; if optional-permissions won, a previously granted site needs no re-click on later visits.
- [ ] Wallet announces via wallet-standard on the playground dapp under the shipped configuration, including after a browser restart.
- [ ] Extension submitted to the Chrome Web Store; listing approved and public; URL recorded in `state.md` and linked from faraday.to.
- [ ] Listing's privacy-policy link and data-use disclosures match `extension/PRIVACY_POLICY.md`.
**Owner:** Trskel (Javi Lois) — assigned 2026-07-20; per the claiming protocol the claim becomes visible when the draft PR titled `FA-08` opens.

### FA-09 `P1` — Durable-nonce transactions: signed QR-relayed txs must not expire
**Description:** Relaying QR codes between browser and device takes real time, and a transaction built on a recent blockhash goes stale in roughly 60–90 seconds — an expiry problem inherent to air-gapped signing. Use Solana durable nonces: the transaction references a nonce account's stored blockhash and leads with a `SystemProgram::AdvanceNonceAccount` instruction, so the signature stays valid until the nonce advances, however long the QR relay takes.
**Plan:**
1. Fixture-first on the device: add fixtures (canonical public test vectors only — no real seeds/keys) for legacy and v0 transactions whose first instruction is `AdvanceNonceAccount`, then extend the system-program classifier in `hardware/` so review labels the nonce-advance instruction (nonce account, nonce authority) instead of hitting the unknown-instruction warning path. Anything malformed or unrecognized keeps failing safe (warn, never pretty-print a guess).
2. Extension: nonce-account lifecycle — one-time create/initialize of a nonce account funded by the user's wallet, fetch the current nonce value, and build sign-requests using the nonce blockhash plus the leading advance instruction; handle the nonce-authority signer correctly.
3. Decide and implement when nonces apply — proposed default: always, for any transaction relayed to the device (opt-in complicates the UX for no benefit here). State the choice in the PR.
4. Edge cases: nonce advanced between build and submit (stale nonce → detect on submit failure and rebuild), nonce account missing/not yet initialized, insufficient rent balance for the nonce account, v0 versioned transactions.
5. Verify on devnet via the playground: build → QR to device → sign → deliberately wait past normal blockhash expiry (2+ minutes) → relay back → submit successfully.
**Acceptance criteria:**
- [ ] A device-signed devnet transaction submits successfully after waiting well past normal blockhash expiry (2+ minutes between sign and submit).
- [ ] Device review shows the nonce-advance instruction as a labeled system instruction (fixture-backed test); unknown or malformed variants still warn.
- [ ] All fixtures use canonical public test vectors.
- [ ] Nonce-account creation and stale-nonce rebuild paths handled in the extension flow.
- [ ] Follow-up card cut for the mobile send flow if it isn't covered in the same PR (one concern per PR).
**Owner:** cxalem (assigned; per the claiming protocol, the claim becomes real when the draft PR titled `FA-09` opens)

### FA-10 `P2` — QR scan latency: benchmark Pi vs ESP32, then reach parity or better
**Description:** Scan latency is the most-felt friction in the signing loop, and the team is mid-migration from Raspberry Pi Zero to ESP32-S3 (PRs #73/#93) with **no numbers on either device yet**. So this is benchmark-first: measure scan latency on the Pi baseline and on the current ESP32 build under identical conditions, then optimize the ESP32 path until it **matches or beats the Pi**. Pi-era prior art exists — open PR #26 (`perf/crop-and-downsample`) and branches `perf/scan-pipeline`, `perf/luma-direct-decode` — triage it for ideas that port to the ESP32 pipeline.
**Acceptance criteria:**
- [ ] Benchmark recorded (in the PR and `docs/updates/`) for **both** devices, same fixtures and conditions: time-to-decode for a single QR and for a full animated UR transaction set.
- [ ] ESP32-S3 scan latency matches or beats the Pi baseline on both measurements.
- [ ] PR #26 and the perf branches triaged: merged, ported, or closed with the reason written down.
- [ ] No decode-accuracy regression: existing scan/decoder tests and fixtures pass; unknown/garbled frames still fail safe.
- [ ] If optimization needs more than one focused PR, follow-up cards are cut per change rather than growing this one.
**Owner:** —

### FA-17 `P1` — ESP32-S3 release artifact: versioned firmware image + ESP Web Tools manifest
**Description:** The web flasher (FA-13) needs something to flash. The firmware already runs on ESP32-S3 (per Nahem and Javi), so this is **release/CI plumbing, not firmware bring-up**: make the release workflow build a flashable ESP32-S3 image, version it with the release tag, emit the ESP Web Tools JSON manifest that points at it, and publish both as release assets. The team hasn't produced a browser-flashable ESP32 artifact before, so the card starts with a short shaping spike.
**Plan:**
1. **Spike (timeboxed):** establish how to produce a single flashable image — `espflash save-image` / merged-binary format (bootloader + partition table + app at their offsets) — and the ESP Web Tools manifest schema (chip family, parts, offsets). Record findings in the PR and `docs/updates/` so the knowledge isn't tribal.
2. Extend the release workflow (`.github/workflows/release.yml`) to build the merged ESP32-S3 image from the xtensa build (requires the xtensa CI job from PRs #73/#93 to be merged), versioned with the release tag alongside the existing Pi OS image.
3. Emit the ESP Web Tools manifest JSON referencing the versioned artifact URL; publish image + manifest as release assets — nothing checked into the repo or `site/`.
4. Verify by flashing a real ESP32-S3 from those exact published artifacts (esptool or ESP Web Tools locally), then booting the firmware.
**Depends on:** ESP32-S3 support merged (open PR #73 + audit-blocker rollup PR #93 — the xtensa build this packages).
**Acceptance criteria:**
- [ ] Release workflow emits a versioned, flashable ESP32-S3 image plus an ESP Web Tools manifest as release assets.
- [ ] A real ESP32-S3 flashed and booted from the published artifacts, not from a local build.
- [ ] Spike findings (image format, partition offsets, manifest schema) recorded in `docs/updates/`.
- [ ] The released image is built from the same firmware the CI nm radio audit gates — the no-radio-symbols property holds for what users actually flash.
**Owner:** —

### FA-13 `P1` — Web-based firmware flasher on the site
**Description:** A page on faraday.to (`site/`, Next.js) where users flash Faraday firmware from the browser. **This is part of the FA-16 landing-page redesign** — the flasher page lands within the new site, so build it against FA-16's structure, not the old one. It is feasible for the **ESP32-S3 target** via WebSerial (ESP Web Tools); it is *not* feasible for the Pi Zero, which boots a Buildroot SD-card image (`opt/`) that can't be written over WebSerial — so this card is scoped to the ESP32 target only, and the page must say so plainly.
**Depends on:** FA-16 (the flasher ships as a page of the redesigned site), FA-17 (the versioned ESP32 image + ESP Web Tools manifest it flashes), and ESP32-S3 hardware support landing (open PR #73 + audit-blocker rollup PR #93).
**Acceptance criteria:**
- [ ] A flash page on the site using ESP Web Tools / WebSerial, flashing a real ESP32-S3 successfully from Chrome or Edge.
- [ ] Firmware binaries served versioned from GitHub releases via an ESP Web Tools manifest — no binaries checked into `site/`.
- [ ] Unsupported browsers (no WebSerial) get a clear message with the manual-flash alternative, not a broken button.
- [ ] The page states exactly which hardware it targets and that Pi Zero devices use the SD-card image instead.
**Owner:** —

### FA-14 `P2` — Optimize CI wall-clock time
**Description:** `.github/workflows/ci.yml` has grown — Rust host feature-matrix checks, tests, Pi Zero ARM cross-compile, three JS typecheck jobs, and an ESP32-S3 xtensa build + nm radio audit job arriving with the ESP32 branches — and PR feedback time is degrading. Measure first, then cut, without weakening what's enforced.
**Acceptance criteria:**
- [ ] Per-job baseline timings recorded in the PR description before changes.
- [ ] Wall-clock for a typical PR measurably reduced (state the before/after; caching, job consolidation, and path-filtering are the expected levers — e.g. JS-only changes need not build Rust and vice versa).
- [ ] Nothing enforced today is lost: `cargo test`, clippy `-D warnings`, all typechecks, and cross-compile/xtensa builds still gate merges to `main` (path filters, if used, must not let a change class skip a job it can break).
- [ ] The ESP32 xtensa + nm radio-audit job is accounted for in the plan (it must keep running on `hardware/` changes — it enforces the no-radio-symbols property of the air-gap).
**Owner:** —

### FA-15 `P2` — Mobile app epic — scoping spike
**Description:** The mobile app is confirmed as an **epic** (team direction 2026-07-20). **End goal: a real app that does what the extension does** — dapp connections, swaps, transfers, the full signer-relay (QR to device and back) functionality. Today `mobile/` is a React Native + Expo watch-only wallet for the Solana Seeker (pair via QR, balances, send with QR-relay signing, risk-analyzer port; landed in PR #64) — a long way from that goal, which is why the epic's **first child card is this scoping spike**: its output is a roadmap milestone and sized child cards, not code. Decide the gap plan (dapp connection surface on mobile, swap/transfer flows, approver-flow parity noted under "Later" in the roadmap), distribution (Seeker dApp store vs Play/App Store), and the maintenance bar we're committing to.
**Acceptance criteria:**
- [ ] `roadmap.md` gains a mobile-epic milestone stating the end goal above, with explicit in-scope / out-of-scope lists.
- [ ] 3–6 child cards cut on this board, each sized for one focused PR, with honest priorities and dependencies — together they trace a path from watch-only to extension parity.
- [ ] Distribution decision recorded (which store(s), and what that implies — review policies, signing, update cadence).
**Owner:** —

### FA-19 `P2` — Durable-nonce for the mobile send flow
**Description:** Follow-up to FA-09, which landed durable nonces on the **extension** send flow only (device parser support is shared and already done). The mobile app (`mobile/`, watch-only Solana Seeker wallet, PR #64) has its own QR-relay send path that still pins transactions to a recent blockhash and can expire during the relay. Port the FA-09 approach: one-time nonce-account provisioning per wallet, build transfers with a leading `AdvanceNonceAccount`, stale-nonce rebuild on submit failure. Reuse the device-side classification from FA-09 (no firmware change needed). Kept as its own card per "one concern per PR."
**Depends on:** FA-09 (the extension implementation + device parser support it reuses).
**Acceptance criteria:**
- [ ] Mobile send flow builds durable-nonce transfers (leading `AdvanceNonceAccount`), provisioning a nonce account on first send.
- [ ] Stale-nonce rebuild handled (re-fetch nonce, rebuild) on submit failure.
- [ ] A device-signed devnet tx from mobile submits after waiting past normal blockhash expiry (2+ minutes).
**Owner:** —

### FA-21 `P2` — Touch UX: press feedback + honest footer
**Description:** Two touch-feel problems on the ESP32-S3 UI. (1) Tapping a list row gives zero visual feedback — the screen just changes, so taps feel dead. Add a press-down highlight: the row under the finger highlights the instant contact begins (the driver already knows the press point; today it waits ~150ms to confirm the lift with nothing on screen), and the action still commits on lift. (2) The footer's Confirm cell (✓) is redundant on every tap-to-go list screen (menu, pickers, warnings — tapping the row IS the action) yet its tap zone fires even with no icon drawn. Prune it: no ✓ drawn and the cell inert on those screens; Back stays everywhere. On the real commitment gates the glyph becomes a labeled button — SIGN (tx approval), CONFIRM (the #89 no-checksum address gate). Swipe-back was considered and deferred: horizontal swipes already page the backup walkthrough and tx review, and an accidental back mid-ceremony is the worst failure mode.
**Acceptance criteria:**
- [ ] Pressing a list row highlights it while the finger is down; lift commits, drag/swipe cancels the highlight.
- [ ] Tap-to-go screens draw no footer ✓ and their right footer third does nothing.
- [ ] TX approval shows SIGN, the explicit-confirm address gate shows CONFIRM, as labeled footer buttons; keyboard accept keeps ✓.
- [ ] Read-only advance screens (about, QR, word display) keep ✓ as the page-forward affordance.
- [ ] Verified on the device: menu, create flow, settings, sign flow; host tests green on both feature sets.
**Owner:** cxalem

### FA-20 `P2` — IMU support + shake-to-theme Easter egg on the ESP32-S3
**Description:** The Waveshare ESP32-S3-Touch-LCD-2 ships a QMI8658 6-axis IMU that the firmware never touched. This card wires it up (new `BoardImu` board trait; driver shares the touch controller's I2C bus; passive sensor, no radio, no air-gap surface) and uses it for a shake gesture: a firm shake flips the whole UI between the dark palette and a new light palette modeled on the site's light side (paper bg, ink text, brand-ink `#0E7580` accent — bright cyan is unreadable on light). Not persisted: the device has no storage by design, every boot starts dark. QR rendering stays black-on-white in both themes. The original scope — a gyroscope parallax screensaver — was prototyped through 9 on-device iterations and dropped; the design study lives in `docs/updates/2026-08-14-01-fa20-spatial-screensaver-vs-atropos.md` and the screensaver stays the classic bounce, now theme-aware.
**Acceptance criteria:**
- [x] QMI8658 probed and configured on the shared I2C bus; a board without the chip simply never toggles.
- [x] Shake detection (spike counting + cooldown) toggles the theme from any screen; verified on the device.
- [x] Every screen renders correctly in both palettes via `Theme` tokens; QR/backup rendering stays fixed black-on-white.
- [x] Touch, camera, and the rest of the loop unaffected by the shared bus; host tests green (339 + 340 touch-ui).
- [x] Pi/simulator behavior unchanged (no IMU → dark theme, bouncing screensaver).
**Owner:** cxalem

### FA-23 `P1` — A sign button you can't miss on the TX review
**Description:** Reviewing a transaction on the touch build gives no clear way to sign: the sign affordance is a small footer cell that only shows when `can_sign` is true, the detail pages (2..K) draw their ✓ unconditionally so it silently no-ops when signing is blocked, and when `can_sign` is false (wallet not a signer, or unresolved lookup-table accounts — common for swaps) the zoned summary card drops the explanation entirely. Fix: a **sign band** spanning the footer's middle+right cells on every SignReview page — accent-filled "SIGN TRANSACTION" when signing is possible (tap anywhere on it signs; the middle cell's page-forward role is redundant, body taps page), and an explicit danger-toned blocker ("CAN'T SIGN: OTHER WALLET" / "CAN'T SIGN: LOOKUP TABLES") when it isn't. Reject ✗ stays in the left cell. Physical-key builds keep their K1 model but stop drawing the dead ✓ when signing is blocked.
**Acceptance criteria:**
- [ ] Every SignReview page (summary, details, raw) shows the band: SIGN TRANSACTION or the specific blocker.
- [ ] Tapping anywhere on the band signs when allowed; taps on it when blocked do nothing (and the reason is readable).
- [ ] The unconditional dead ✓ on detail pages is gone on both build flavors.
- [ ] Verified on the device with a signable tx and a blocked one; host tests green.
**Owner:** cxalem

### FA-24 `P1` — Lookup-table transactions sign normally
**Description:** Real dapp transactions (Jupiter swaps foremost) reference address lookup tables created dynamically on-chain; no firmware snapshot can keep up, so the old hard-block (`can_sign = false` on any unresolved ALT account) made swaps permanently unsignable — surfaced on-device as "CAN'T SIGN: LOOKUP TABLES" on a plain SOL→USDC swap. New policy: such transactions **review and sign exactly like any other**. A warning interstitial was built and rejected (UX decision 2026-08-15): it would fire on every ordinary swap for a condition the user can neither evaluate nor act on, training people to slam through warnings and devaluing the ones that matter (the high-risk classifier, the address gate). What the user sees is unchanged in the ways that count — amounts, programs, and fees are decoded from the exact signed bytes; a quiet line in the scrollable details notes that some accounts resolve on-chain, and unresolved entries render as a sentinel, never as a fake address. `can_sign` now means only "this wallet is a required signer". Follow-up candidate: the extension resolving table contents and embedding them in the QR payload for full account display.
**Acceptance criteria:**
- [ ] A live Jupiter swap reviews normally, shows SIGN TRANSACTION, and signs end-to-end (QR → extension splice → lands on-chain).
- [ ] Non-signer wallets still hard-block with the OTHER WALLET reason.
- [ ] Parser tests updated; sentinel accounts never display as real addresses.
### FA-22 `P2` — Camera-entropy capture feedback
**Description:** Creating a wallet via camera entropy is the marquee moment of the product and currently gives near-zero feedback: the live preview fills the screen, each tap silently hashes a frame, the only signal is the tiny header counter, and the second capture teleports straight to the backup warning — the tap feels dead and nothing tells a first-time user to tap at all. Add three pieces: (1) a **shutter flash** on every capture (the commit is deferred ~120ms via the existing pending-confirm mechanism so the *final* capture's flash is visible too — otherwise the instant transition eats it); (2) an **entropy meter strip** under the header — segmented blocks filling per capture with a bits counter (256/512), matching the receipt the landing page already shows for this exact screen; (3) a **"TAP TO CAPTURE" hint** over the preview on touch builds. Security constraint: nothing on screen may derive from the entropy pool itself (screen-filming leak channel) — all feedback is driven by frame count only.
**Acceptance criteria:**
- [ ] Every capture (including the final one) shows a visible shutter flash before the screen advances.
- [ ] Meter strip fills per capture with a bits label; matches the actual pool math (256 bits/frame).
- [ ] Touch builds show a capture hint; physical-key builds unchanged apart from the meter.
- [ ] No UI element derives from entropy-pool bytes; verified on the device; host tests green.
**Owner:** cxalem

### FA-26 `P1` — Durable nonce for dapp transactions
**Description:** FA-09 (PR #112) gave durable-nonce lifetimes to transfers Faraday itself builds, but left dapp-built transactions (Jupiter swaps, playground, the Ika demo) on a perishable blockhash — and those are exactly the big animated-QR relays that take longest, and the ones a real user has already hit expiring mid-scan. Rewrite dapp transactions to durable-nonce form in the background sign-session handler when it's safe to do so; byte-exact passthrough on any unsafe case — a dapp sign must never hard-fail because of the rewrite. Wallets with no nonce account yet get a one-time setup interstitial in the sign popup, with graceful fallback to plain signing on skip or any failure. Default ON with a settings kill-switch (escape hatch for dapps that broadcast their own original copy of the transaction instead of the signed bytes Faraday returns).
**Acceptance criteria:**
- [ ] A dapp-signed devnet transaction submits successfully 2+ minutes after signing (past normal blockhash expiry).
- [ ] A v0 transaction referencing an address lookup table round-trips through the rewrite; a fixture test asserts `addressTableLookups` and account indices at the byte level.
- [ ] Every unsafe case (partial signatures, fee payer isn't the wallet, already durable-nonce, oversize, RPC failure/timeout) passes through byte-exact with the reason logged; toggle OFF ⇒ byte-exact passthrough for every dapp tx.
- [ ] Provisioning interstitial: Set up flows through create-nonce-account + the real tx; Skip or any provisioning failure signs the original transaction normally, never blocking.
- [ ] Typecheck, vitest, and the MV3 build are green.
**Owner:** cxalem

### FA-28 `P2` — Token balances update instantly after a swap/receive
**Description:** The sidepanel already holds a live WebSocket on the wallet account, but a push only revalidates the SOL balance — the SPL token list stays on a 60s poll, so a token you just swapped into can show stale for up to a minute. Wire the same push to a debounced token-list refresh (single trailing call, ~2s) so balances feel immediate; SOL path and reconnect logic untouched.
**Acceptance criteria:**
- [x] A live push triggers exactly one debounced token refresh (no React hook-testing infra in this repo — verified by typecheck + code review in PR #135, not a unit test).
- [x] SOL refresh behavior unchanged.
- [x] No extra DAS calls when no push fires (idle behavior identical).
- [x] Typecheck + vitest + MV3 build green.
**Owner:** cxalem

### FA-25 `P2` — Verify-backup falsely reports "MISMATCH" on a decode failure, not just a real content mismatch
**Description:** Caught on camera during the marketing demo shoot (`2026-08-16 00-58-01.mov`, ~20:00–20:03): right after finishing a hand-drawn paper backup and entering Verify Backup, the very first scan showed "MISMATCH — Paper doesn't match / The scanned QR is not this wallet's seed" — while the paper was still being brought into frame (visibly in motion in the footage). The same physical paper then scanned correctly on every later attempt (4–5 reloads since), which a genuine content mismatch can't explain — a wrong mnemonic would keep mismatching. `core/src/gui/flows/verify.rs::handle` (`Screen::VerifyBackupScan`) explains it: `decode_qr::detect_and_decode` returning `None` (couldn't read a QR at all — blur, motion, partial frame) and returning `Some(mnemonic)` that doesn't match the loaded wallet both route to the identical `Screen::VerifyBackupSeedMismatch` screen. A plain "couldn't read it, try again" is being shown to the user as "these are the wrong seeds," which is a much scarier and more specific claim than the evidence supports.
**Acceptance criteria:**
- [ ] Decode failure (`None`) and content mismatch (`Some(m)` where `m != wallet.mnemonic`) route to distinct screens/messages — the no-read case reads as a scan retry prompt, not a seed-mismatch accusation.
- [ ] Regression/unit test covering both branches of `VerifyBackupScan`'s `Confirm` handling.
- [ ] Manual check: a deliberately blurred/partial scan shows the new "couldn't read it" screen, not MISMATCH.
**Owner:** —

### FA-12 `P3` — Faraday MCP server *(idea — unshaped)*
**Description:** Early-stage idea: an MCP server exposing Faraday's host-side capabilities to LLM agents/tools — e.g. building sign-requests, encoding/decoding QR (UR) payloads, running the tx risk analyzer. Unshaped: the value proposition and the surface are undefined. One boundary is already non-negotiable and must anchor any proposal: an MCP server is host-side software (extension/companion territory); `hardware/` gains no network dependencies and the device's review-what-you-sign flow is unchanged — an agent can prepare transactions, it can never approve them.
**Acceptance criteria:** *(shaping only — no build work from this card)*
- [ ] A one-page proposal in `docs/proposals/` covering: concrete use cases, the tool surface, and an explicit off-limits list (no key material access, no `hardware/` involvement, device approval stays human).
- [ ] Team decision recorded (adopt as roadmap work / defer / drop), folded into `roadmap.md` like FA-07.
**Owner:** —

### FA-27 `P2` — Ika heroes survive a leading nonce-advance
**Description:** FA-09 taught `extract_send` to skip a leading `AdvanceNonceAccount` but `extract_ika` still rejects any non-Ika instruction, so durable-nonce-rewritten Ika transactions (FA-26) lose their hero screen and fall to the generic instruction list — still signable, just unlabeled. Add the same disc-4 skip to `extract_ika`, back it with an Ika+nonce fixture, and fix the stale `just nonce-fixtures` recipe (says `cd hardware`, crate lives in `raspberry-pi/`).
**Acceptance criteria:**
- [x] An Ika tx with a leading `AdvanceNonceAccount` still produces its zoned hero (fixture-backed test).
- [x] `just nonce-fixtures` runs from repo root.
- [x] `cargo test --features simulator` green (verified via `cd core && cargo test` / `--features touch-ui`, which is where the parser tests live — `raspberry-pi/`'s own `cargo test --features simulator` has 0 unit tests of its own).
**Owner:** cxalem
