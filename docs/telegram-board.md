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

## Audience & voice — write for mortals

Faraday Signal doubles as a public dev-log: assume the reader is a curious
non-developer following the project, not a teammate. Every post must pass the
**mortal test** — someone who has never opened the repo understands what
happened and why it matters.

- **Line 2 is the plain-English line.** Right after the headline, one sentence
  on what this means for someone using Faraday ("Transactions you sign will now
  go through even if the QR handoff takes a few minutes."). Tech facts (branch,
  PR, test counts) come after it, one per line.
- **No unexplained jargon.** Say "the browser extension", not "MV3"; "the
  signing device", not "the ESP32 target". If a term is unavoidable, gloss it in
  a few words on first use ("durable nonce — a trick that stops transactions
  from expiring").
- **Lead with the outcome, not the activity.** "The device now shows exactly
  what a multisig transaction does before you sign" beats "implemented
  clear-msig classifiers".
- **Card ids stay** — they're the paper trail — but as a suffix in parentheses
  (`(FA-09, PR #114)`), never as the subject of the sentence.
- **The pin follows the same rule:** every bullet is a plain description a
  stranger can parse, with the id in parentheses at the end.

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

Multiline posts, one shape each. **Line 1 is the headline** (rendered bold), in
plain words with the card id as a suffix: `<emoji> <plain outcome> (FA-NN)`.
**Line 2 is the plain-English line** (see Audience & voice). Lines after it
carry the tech detail — one fact per line, no prose paragraphs. The author
stamp is appended automatically.

```
🔨 Started: transactions that can't expire mid-handoff (FA-09)
Today a signed transaction goes stale if you take too long scanning QR codes — this fix removes that time limit.
branch feat/durable-nonce · draft PR #114
```

```
✅ Ready for review: transactions that can't expire (FA-09)
You'll be able to take as long as you like between signing on the device and sending — the transaction still lands.
PR #114 · verified: full test suite + simulator relay loop
```

```
🏁 Shipped: transactions that can't expire (FA-09, PR #114 merged)
```

```
📋 Plans updated
• New task: the mobile app gets the same no-expiry fix (FA-19)
• The device cost estimate now covers both hardware versions (FA-02)
```

```
⚠️ Paused: the new website design (FA-16)
Waiting on the design assets before building can continue.
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
- **On claiming a card** (branch + draft PR created): `post "🔨 Started: <plain outcome> (FA-NN)"`
  with a plain-English line 2, then refresh the pin.
- **On finishing** (PR ready for review / merged): `post "✅ Ready for review: <plain outcome> (FA-NN, PR #N)"`
  with a plain-English line 2, and refresh the pin.
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
