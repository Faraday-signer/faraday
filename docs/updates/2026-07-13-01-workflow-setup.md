---
date: 2026-07-13
slug: workflow-setup
---

# Board + agents workflow bootstrapped

Mirrored the LaPropia working model into this repo:

- **`.claude/agents/project-manager.md`** — PM/Scrum agent that owns `docs/backlog.md`.
- **`.claude/agents/pr-reviewer.md`** — advisory PR reviewer (ranked findings + manual test guide), tuned to the air-gap threat model.
- **`docs/backlog.md`** — Kanban board seeded with the grant-push cards (FA-01…FA-05, from the 2026-07-13 La Familia debrief) and product cards (FA-06 Ika approver demo, FA-07 verify-proposals decision).
- **`docs/state.md`** — current-state map of the repo.
- **`docs/roadmap.md`** — milestones: M1 grant readiness → M2 Ika demo → M3 branded batch.
- **`docs/README.md`** — index + the workflow loop.
- **`docs/updates/`** — this log (one file per change).

`/CLAUDE.md` stays the single engineering rulebook; the agents reference it rather than duplicating it.

Verified: docs only, no code paths touched.
