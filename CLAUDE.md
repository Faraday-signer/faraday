1./ Think before coding
Don't assume. Don't hide confusion. State ambiguity explicitly. Present multiple interpretations rather than silently picking one. Push back if a simpler approach exists. Stop and ask rather than guess.

2./ Simplicity first
No features beyond what was asked. No abstractions for single-use code. No "flexibility" that wasn't requested. No error handling for impossible scenarios. The test: would a senior engineer say this is overcomplicated? If yes, rewrite it.

3./ Surgical changes
Don't "improve" adjacent code. Don't refactor things that aren't broken. Match the existing style even if you'd do it differently. If you notice unrelated dead code, mention it, don't delete it. Every changed line should trace directly to the request.

4./ Goal-driven execution
Transform "fix the bug" into "write a test that reproduces it, then make it pass." Transform "add validation" into "write tests for invalid inputs, then make them pass." Give it success criteria and watch it loop until done.

5./ Git workflow
Follow Conventional Commits (https://www.conventionalcommits.org/en/v1.0.0/). Commit messages must use the format: `type(scope): description` — e.g. `feat(signing): wire sign transaction flow`, `fix(bip39): use raw entropy for coin flips`, `chore(build): cfg-gate GUI modules`.

Valid types: feat, fix, refactor, chore, docs, test, style, perf, ci, build.

Every feature, bug fix, or chore gets its own branch off `main` and a PR. Branch naming: `type/short-description` — e.g. `feat/sign-message`, `fix/entropy-handling`, `chore/cleanup-warnings`. Keep PRs small and focused — one concern per PR. Never commit directly to `main`.

6./ Team board sync
Task claims are mirrored to the private "Faraday Signal" Telegram channel via `scripts/tg-board.sh` (see docs/telegram-board.md). Check the pinned board (`scripts/tg-board.sh read-pin`) alongside `gh pr list` before starting work on a backlog card; post when you claim or finish one and refresh the pin. Requires `.env` (copy `.env.example`); if it's missing, say so and continue — Telegram is a mirror, never a blocker.
