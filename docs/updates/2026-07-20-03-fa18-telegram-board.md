# 2026-07-20 — FA-18 cut and claimed: Telegram board sync

- Created private Telegram channel **Faraday Signal** (read-only; bot **@faraday_board_bot** is the only poster) with a pinned "who's working on what" board. Bot created via BotFather, group invitations disabled.
- Cut **FA-18** (Telegram board sync — script, `.env.example`, doc, agent wiring) directly into **In Progress**, owner cxalem, branch `feat/telegram-board`.
- This PR also starts tracking `docs/` (the board itself) and `.claude/agents/` (shared agent definitions) — both were local-only until now, which contradicted the "any contributor gets the same behavior" premise.
- Token distribution is manual and private (DM from cxalem); `.env` stays gitignored.
