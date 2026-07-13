# Faraday — Docs & Knowledge Base

Everything a contributor (human or agent) needs to work on Faraday, in one place. The product pitch lives in the [root README](../README.md); the engineering rulebook is [`/CLAUDE.md`](../CLAUDE.md). This folder is the working knowledge base.

## The workflow

| File | What it is | Owner |
|------|------------|-------|
| [`backlog.md`](./backlog.md) | The single Kanban board — what to work on, in what order, and why | PM agent (`.claude/agents/project-manager.md`) |
| [`state.md`](./state.md) | What actually exists in the repo today | Whoever ships |
| [`roadmap.md`](./roadmap.md) | Milestones (current: grant readiness) + locked-in decisions | Team |
| [`updates/`](./updates/) | The change log — one file per change, no merge conflicts | Everyone |
| [`proposals/`](./proposals/) | Design proposals under discussion | Authors |

**The loop:** pick a card from `backlog.md` → read `/CLAUDE.md` → build it on its own branch → PR (the `pr-reviewer` agent gives a first pass + a manual test guide) → merge → update the board + `state.md` + add an `updates/` entry.

## Agents

- **`@project-manager`** (Fable) — owns the board and writes the detailed implementation plan on every card. Plans, never code: its edits stay in `docs/**`.
- **`@builder`** (Opus) — implements one backlog card end-to-end from its plan: branch, tests, verified change, board + docs updated, PR-ready.
- **`@pr-reviewer`** (Opus) — reviews a PR: ranked findings + a manual/local test plan. Advisory, read-only.

**Model policy:** Fable plans (detailed, prose-only — it never writes code); Opus builds and reviews.

## Related, outside this repo

- `demos/` — runnable demo walkthroughs (currently the Ika clear-msig approver loop).
- Business/partner context (meeting notes, grant conversation, marketing pipeline) lives in the team's private knowledge base, not here. Cards reference it when needed.
