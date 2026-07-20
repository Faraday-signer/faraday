#!/usr/bin/env bash
# Faraday Telegram board — post task updates and maintain the pinned status board
# in the private "Faraday Signal" channel. See docs/telegram-board.md.
#
# Usage:
#   scripts/tg-board.sh post "🔨 started FA-09 — durable-nonce transactions"
#   scripts/tg-board.sh read-pin
#   scripts/tg-board.sh update-pin "new full text of the pinned board"
#   echo "new full text" | scripts/tg-board.sh update-pin
#
# Write PLAIN TEXT — the script formats it (bold FA-NN ids, bold headers,
# italic footer, HTML-escaped). Don't send HTML tags; they'll be escaped.
# Posts are stamped with your name (git config user.name, or TG_AUTHOR in .env).
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
ENV_FILE="$REPO_ROOT/.env"
if [[ ! -f "$ENV_FILE" ]]; then
  echo "error: $ENV_FILE not found." >&2
  echo "Copy .env.example to .env and ask cxalem (privately) for TG_BOT_TOKEN." >&2
  exit 1
fi
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a
: "${TG_BOT_TOKEN:?TG_BOT_TOKEN missing from .env}"
: "${TG_CHAT_ID:?TG_CHAT_ID missing from .env}"

API="https://api.telegram.org/bot${TG_BOT_TOKEN}"
AUTHOR="${TG_AUTHOR:-$(git config user.name 2>/dev/null || echo unknown)}"

# fmt <post|pin> — stdin plain text → stdout Telegram HTML.
fmt() {
  python3 -c '
import html, re, sys

mode = sys.argv[1]
bold_ids = lambda s: re.sub(r"\bFA-(\d+)\b", r"<b>FA-\1</b>", s)
out = []
for i, raw in enumerate(sys.stdin.read().splitlines()):
    line = html.escape(raw, quote=False)
    if not raw.strip():
        out.append("")
    elif mode == "pin" and i == 0:
        out.append(f"<b>{line}</b>")                 # title
    elif mode == "pin" and raw.startswith(("Updated", "\U0001F4D6")):
        out.append(f"<i>{line}</i>")                 # timestamp + 📖 footer
    elif "FA-" in raw or mode == "post":
        out.append(bold_ids(line))                   # bullets / body text
    else:
        out.append(f"<b>{line}</b>")                 # section headers
print("\n".join(out), end="")
' "$1"
}

api_call() { # api_call <method> [curl --data-urlencode args...]
  local method="$1" response
  shift
  response="$(curl -s "$API/$method" "$@")"
  if [[ "$response" != '{"ok":true'* ]]; then
    echo "error: Telegram $method failed: $response" >&2
    return 1
  fi
}

case "${1:-}" in
  post)
    shift
    [[ $# -gt 0 ]] || { echo "usage: tg-board.sh post \"message\"" >&2; exit 2; }
    TEXT="$(printf '%s' "$*" | fmt post)"
    api_call sendMessage \
      --data-urlencode chat_id="$TG_CHAT_ID" \
      --data-urlencode parse_mode=HTML \
      --data-urlencode text="$TEXT
<i>— $AUTHOR</i>"
    echo "posted"
    ;;
  read-pin)
    curl -s "$API/getChat?chat_id=$TG_CHAT_ID" | python3 -c '
import json, sys
pinned = json.load(sys.stdin).get("result", {}).get("pinned_message", {})
print(pinned.get("text", ""))'
    ;;
  update-pin)
    shift
    if [[ $# -gt 0 ]]; then TEXT="$*"; else TEXT="$(cat)"; fi
    [[ -n "$TEXT" ]] || { echo "error: empty pin text" >&2; exit 2; }
    : "${TG_PIN_MSG_ID:?TG_PIN_MSG_ID missing from .env}"
    api_call editMessageText \
      --data-urlencode chat_id="$TG_CHAT_ID" \
      --data-urlencode message_id="$TG_PIN_MSG_ID" \
      --data-urlencode parse_mode=HTML \
      --data-urlencode link_preview_options='{"is_disabled":true}' \
      --data-urlencode text="$(printf '%s' "$TEXT" | fmt pin)"
    echo "pin updated"
    ;;
  *)
    echo "usage: tg-board.sh post \"msg\" | read-pin | update-pin [\"text\"]" >&2
    exit 2
    ;;
esac
