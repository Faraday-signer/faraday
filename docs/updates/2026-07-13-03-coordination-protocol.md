---
date: 2026-07-13
slug: coordination-protocol
---

# Multi-contributor coordination: claims via draft PRs; PM gets direction inputs

Several people now work this repo in parallel; a markdown board alone races (it only syncs on merge). Changes:

- **Claim protocol** (in `docs/backlog.md`): the open **draft PR titled with the card ID** is the claim, not the board. Claim = branch → board edit as first commit → push → draft PR. Check `gh pr list` before picking a card.
- **`builder` agent**: checks claims before branching; opens the draft PR as its claim (step 2); finishes by filling the PR body and marking ready-for-review.
- **`project-manager` agent**: now reads `docs/roadmap.md` as its direction input and the newest `docs/updates/` entries, cross-checks open branches/PRs before recommending work, excludes claimed cards from "what's next", and records new direction into the roadmap before cutting cards from it.
- **`.gitattributes`**: `merge=union` on `backlog.md` / `state.md` / `roadmap.md` so board-edit merges don't conflict.

Verified: docs + agent definitions only.
