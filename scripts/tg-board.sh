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
    api_call sendMessage \
      --data-urlencode chat_id="$TG_CHAT_ID" \
      --data-urlencode text="$* — $AUTHOR"
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
      --data-urlencode text="$TEXT"
    echo "pin updated"
    ;;
  *)
    echo "usage: tg-board.sh post \"msg\" | read-pin | update-pin [\"text\"]" >&2
    exit 2
    ;;
esac
