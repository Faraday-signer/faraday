# Telegram board — "Faraday Signal"

The team mirrors task claims to a private Telegram **channel** called **Faraday Signal**:
read-only for humans, posted to only by the bot **@faraday_board_bot**. Its **pinned
message** is the live "who's working on what" board; a short post announces every claim
and finish. The authoritative claim is still the draft PR (see `backlog.md`) — the
channel is the human-visible mirror.

## Join (humans)

Ask cxalem for the invite link. The channel is private; the link is the only way in.

## Setup (per machine — required for agents to post)

1. `cp .env.example .env`
2. Ask cxalem for `TG_BOT_TOKEN` **over a private DM** and paste it in. Never commit
   `.env`, never paste the token into the channel or a PR.

That's it — posts are stamped with your `git config user.name`.

## Usage

```sh
scripts/tg-board.sh post "🔨 started FA-09 — durable-nonce transactions"
scripts/tg-board.sh read-pin
scripts/tg-board.sh update-pin "…new full board text…"
```

## Conventions (what agents do automatically)

- **Before recommending or starting work:** `read-pin` alongside `gh pr list --state open`.
- **On claiming a card** (branch + draft PR created): `post "🔨 started FA-NN — <title>"`,
  then regenerate the pin from `docs/backlog.md` (In Progress + top unclaimed To Do cards)
  and `update-pin`.
- **On finishing** (PR ready for review / merged): `post "✅ FA-NN in review — <PR url>"`
  and refresh the pin the same way.
- Pin updates are read-modify-write, last-write-wins — fine, because the draft PR, not
  the pin, is the claim.
- If `.env` is missing, say so and continue — Telegram is a mirror, never a blocker.

## Bot administrivia

- Bot: `@faraday_board_bot`, owned by cxalem via @BotFather. Group invitations are
  disabled (`/setjoingroups` → Disabled) — the bot lives only in Faraday Signal.
- If the token leaks or a teammate leaves: `/revoke` in BotFather, update your `.env`,
  re-share privately.
- `TG_PIN_MSG_ID` is the pinned board message — always **edit it in place**
  (`update-pin`), never delete and repost (a new message id would break everyone's `.env`).
