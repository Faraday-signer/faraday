---
date: 2026-07-13
slug: builder-agent
---

# Builder agent added; model policy set; reviewer commands corrected

**Model policy:** `project-manager` runs on **Fable** — it writes detailed, executable plans on cards (new required **Plan** field before To Do) but never writes code; `builder` and `pr-reviewer` run on **Opus**.

- **`.claude/agents/builder.md`** — implements one `docs/backlog.md` card per invocation: branch-first (`type/short-description`), test-first where it applies, verification on the real surface via the repo's actual toolchain (justfile + CI commands), air-gap hard rules (no network deps in `hardware/`, RAM-only keys, fail-safe parsers, byte-exact QR envelopes), closes the loop on board/state/updates.
- **`.claude/agents/pr-reviewer.md`** — fixed the manual-guide commands: extension/mobile/site use **npm** (`npm ci`, `npm run typecheck`) and `just` recipes, matching CI; the earlier draft wrongly said pnpm.
- **`docs/README.md`** — agents list now shows the full loop: PM → builder → pr-reviewer.

Verified: docs only; commands cross-checked against `justfile` and `.github/workflows/ci.yml`.
