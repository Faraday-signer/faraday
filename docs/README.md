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

**The loop:** pick an unclaimed card from `backlog.md` (check open PRs first — **the open draft PR, not the board, is the claim**) → claim it: branch, board edit as first commit, draft PR titled with the card ID → read `/CLAUDE.md` → build → mark ready for review (`pr-reviewer` gives a first pass + manual test guide) → merge → board + `state.md` + `updates/` entry land with it. Full claiming protocol in [`backlog.md`](./backlog.md#claiming-a-card-how-we-dont-step-on-each-other).

## Agents

- **`@project-manager`** (Fable) — owns the board and writes the detailed implementation plan on every card. Plans, never code: its edits stay in `docs/**`.
- **`@builder`** (Opus) — implements one backlog card end-to-end from its plan: branch, tests, verified change, board + docs updated, PR-ready.
- **`@pr-reviewer`** (Opus) — reviews a PR: ranked findings + a manual/local test plan. Advisory, read-only.

**Model policy:** Fable plans (detailed, prose-only — it never writes code); Opus builds and reviews.

## Related, outside this repo

- `demos/` — runnable demo walkthroughs (currently the Ika clear-msig approver loop).
- Business/partner context (meeting notes, grant conversation, marketing pipeline) lives in the team's private knowledge base, not here. Cards reference it when needed.
