# 2026-07-20 — Board recreated on `feat/ika-approver-demo`; new cards FA-08…FA-16; direction update

**Direction recorded (from the team, 2026-07-20, relayed via cxalem):**

- **We need a better website** — a landing-page redesign, not just a care pass; it is the top of the new-work stack and will include the web flasher. → new card **FA-16**; FA-05 narrowed to email-capture reliability only.
- **New-work priority order:** FA-16 (redesign, with FA-13 inside it) → FA-08 (extension publish) → FA-09 (durable nonce) → then FA-10 / FA-14 / FA-11. FA-12 stays iced; FA-15 gets scoped but is not near-term.
- **Extension permissions are undecided; UX decides.** Spike activeTab + optional_host_permissions + dynamic registration vs `<all_urls>`; parity → optional permissions, degradation → `<all_urls>`. Baked into FA-08 as an explicit decision step.
- **Device hardware is migrating Pi Zero → ESP32-S3** and there are no scan-latency numbers on either. FA-10 is benchmark-first with a match-or-beat-the-Pi criterion. *(This migration also needs folding into `roadmap.md`/`state.md` on PR #74 — flagged, not done here.)*
- **Mobile is an epic**: end goal is a real app doing what the extension does (dapp connections, swaps, transfers, full signer relay). FA-15 is the epic's first child: a scoping spike.

**What changed on the board**

- `docs/backlog.md` did not exist on this branch (or on `main` — the docs knowledge base lives on the unmerged `chore/agent-workflow` branch, draft PR #74). Recreated it here from the canonical version at commit `cd56c81`, then extended. `roadmap.md` / `state.md` were *not* copied over — they stay on PR #74 to avoid divergence; the board's links resolve once it merges. `docs/updates/README.md` restored verbatim.
- New cards (sequence continues from FA-07):
  - **FA-08** `P1` — Publish the Chrome extension to the Web Store (UX spike decides the permissions model) — Product/eng, **To Do** (plan written).
  - **FA-09** `P1` — Durable-nonce transactions so QR-relayed signatures don't expire — Product/eng, **To Do** (plan written), owner cxalem (Alejandro Mena).
  - **FA-10** `P2` — QR scan latency: benchmark Pi vs ESP32, then parity or better; triage open PR #26 + perf branches — Product/eng, Backlog.
  - **FA-11** `P2` — X content plan (pillars, cadence, 4 weeks drafted); depends on FA-01 — Grant push, Backlog.
  - **FA-12** `P3` — Faraday MCP server, *idea — unshaped*; shaping-only card (board has no icebox column) — Product/eng, Backlog.
  - **FA-13** `P1` — Web-based firmware flasher, ESP32-S3 via ESP Web Tools; lands within the FA-16 site; depends on FA-16 + PRs #73/#93 + a release `.bin` — Product/eng, Backlog.
  - **FA-14** `P2` — Optimize CI wall-clock time; must keep the xtensa + nm radio-audit job gating `hardware/` changes — Product/eng, Backlog.
  - **FA-15** `P2` — Mobile app **epic** — scoping spike (end goal: extension parity incl. dapp connections, swaps, transfers) — Product/eng, Backlog.
  - **FA-16** `P1` — Landing page redesign (faraday.to), includes the flasher page slot — Grant push, **To Do** (plan written).
- Card edits: FA-05 narrowed (site refresh moved to FA-16); FA-11 cut to P2 per the new ordering; FA-06's board line now references its open PR (#71).

**Verification:** cross-checked live claims (`gh pr list --state open`, remote branches). Open PRs folded into card text: #26 → FA-10, #70 + `chore/drop-host-permissions` → FA-08, #73/#93 → FA-13. No new card ID collides with an open branch/PR title.

**Not committed** — board edits left as working-tree changes per the requester's instruction.
