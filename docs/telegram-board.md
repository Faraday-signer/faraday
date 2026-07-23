# Telegram board — "Faraday Signal"

The team coordinates through a private Telegram **channel** called **Faraday Signal**:
read-only for humans, posted to only by the bot **@faraday_board_bot**. Its **pinned
message** renders the full board — every card, one line — and a short post announces
every claim and finish.

**Precedence rule — who is the source of truth for what:**

- **The pin wins for status.** Who's working on what, right now. It updates in real
  time; the board file only syncs on merge. Humans never need to open the file.
- **`backlog.md` wins for card content.** Descriptions, plans, acceptance criteria —
  what agents actually build from. Durable, diffable, PR-reviewed.
- **The draft PR is the claim** (ties the two together): pin says it first, git proves
  it, the file converges on merge.

## Join (humans)

Ask cxalem for the invite link. The channel is private; the link is the only way in.

## Setup (per machine — required for agents to post)

1. `cp .env.example .env`
2. Ask cxalem for `TG_BOT_TOKEN` **over a private DM** and paste it in. Never commit
   `.env`, never paste the token into the channel or a PR.

That's it — posts are stamped with your `git config user.name`.

## Usage

```sh
scripts/tg-board.sh post "…message…"        # also accepts stdin
scripts/tg-board.sh read-pin
scripts/tg-board.sh update-pin "…full board…"   # also accepts stdin
```

## Message types

Multiline posts, one shape each. **Line 1 is the headline** (rendered bold):
`<emoji> FA-NN <verb> — <card title>`. Lines after it carry the useful detail —
one fact per line, no prose paragraphs. The author stamp is appended automatically.
**Any post that references a PR carries the full PR URL** on its own detail line
(bare URL — the script escapes HTML tags, Telegram auto-links plain URLs). Never
post a single unformatted run-on line. These shapes apply to *every* poster —
human-driven or agent-driven, on any machine.

```
🔨 FA-09 claimed — durable-nonce transactions
branch feat/durable-nonce · draft PR: https://github.com/Faraday-signer/faraday/pull/114
plan: nonce account per relay hop, fixture-first
```

```
✅ FA-09 in review — durable-nonce transactions
PR ready: https://github.com/Faraday-signer/faraday/pull/114
verified: cargo test + simulator relay loop
```

```
🏁 FA-09 done — durable-nonce transactions
merged: https://github.com/Faraday-signer/faraday/pull/114
```

```
📋 board update
• FA-19 cut — <title> (P1, To Do)
• FA-02 rescoped — dual BOM (Pi + ESP32)
```

```
⚠️ FA-16 blocked — waiting on design assets
unblocks when: <what> · needed from: <who>
```

Emoji vocabulary: 🔨 claimed · ✅ in review · 🏁 done/merged · 📋 board change ·
⚠️ blocked/flag. One post per event — don't batch unrelated events into one message.

**Write plain text — the script does the formatting.** It bolds `FA-NN` ids and
section headers, italicizes the `Updated …` line and the 📖 footer, stamps posts
with an italic author line, and HTML-escapes everything. Don't send HTML tags
(they'd be escaped, not rendered). This is also why read-modify-write is safe:
`read-pin` returns plain text and `update-pin` deterministically re-applies the
styling, so pin updates never degrade formatting. Pin structure the formatter
expects: title on line 1, `Updated YYYY-MM-DD` on line 2, `🔨`/`🎯`/`📋` section
headers, `• FA-NN — title — owner` bullets, `📖` footer.

## Conventions (what agents do automatically)

- **Before recommending or starting work:** `read-pin` first — it is the status
  authority — then `gh pr list --state open` to confirm.
- **On claiming a card** (branch + draft PR created): post the 🔨 shape above
  (headline + branch/PR-URL line), then refresh the pin.
- **On finishing** (PR ready for review / merged): post the ✅ (or 🏁) shape above —
  headline, `PR ready: <full URL>` line, verified line — and refresh the pin.
- **Refreshing the pin = full-board render**, every card as one line under
  🔨 In progress / 🎯 To Do / 📋 Backlog, read-modify-write: start from `read-pin`
  (it may hold claims newer than your checkout), fold in your change and anything
  `backlog.md` adds, `update-pin`. Keep one line per card — the pin caps at 4096 chars;
  specs stay in `backlog.md`.
- If `.env` is missing, say so and continue — Telegram is a mirror, never a blocker.

## Bot administrivia

- Bot: `@faraday_board_bot`, owned by cxalem via @BotFather. Group invitations are
  disabled (`/setjoingroups` → Disabled) — the bot lives only in Faraday Signal.
- If the token leaks or a teammate leaves: `/revoke` in BotFather, update your `.env`,
  re-share privately.
- `TG_PIN_MSG_ID` is the pinned board message — always **edit it in place**
  (`update-pin`), never delete and repost (a new message id would break everyone's `.env`).
