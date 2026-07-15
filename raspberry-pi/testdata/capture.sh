#!/usr/bin/env bash
# Captures Solana transactions as raw bytes and QR images for parser testing.
#
# Usage:
#   ./capture.sh grab <name> <tx_signature>    Save a specific transaction
#   ./capture.sh search <program_id> [limit]   Find recent tx signatures for a program
#   ./capture.sh all                           Fetch all curated transactions
#
# Environment:
#   SOLANA_RPC   RPC endpoint (default: mainnet public)
#
# Dependencies (for QR generation):
#   qrencode, imagemagick
#
# Dependencies (for UR generation):
#   cargo (Rust toolchain)

set -euo pipefail

RPC="${SOLANA_RPC:-https://api.mainnet-beta.solana.com}"
BASEDIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$BASEDIR/test_txs_bin"
QR_DIR="$BASEDIR/test_txs_qr"

check_qr_deps() {
    local missing=()
    command -v qrencode >/dev/null 2>&1 || missing+=(qrencode)
    command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1 || missing+=(imagemagick)
    if [ ${#missing[@]} -gt 0 ]; then
        echo "Missing dependencies for QR generation: ${missing[*]}" >&2
        if [[ "${OSTYPE:-}" == linux* ]]; then
            echo "  sudo apt-get install -y ${missing[*]}" >&2
        elif [[ "${OSTYPE:-}" == darwin* ]]; then
            echo "  brew install ${missing[*]}" >&2
        fi
        return 1
    fi
    return 0
}

convert_cmd() {
    if command -v magick >/dev/null 2>&1; then
        echo "magick"
    else
        echo "convert"
    fi
}

grab() {
    local name=$1 sig=$2
    local retries=5 delay=5

    for attempt in $(seq 1 $retries); do
        local response
        response=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" -d "{
            \"jsonrpc\": \"2.0\", \"id\": 1,
            \"method\": \"getTransaction\",
            \"params\": [\"$sig\", {\"encoding\": \"base64\", \"maxSupportedTransactionVersion\": 0}]
        }")

        local b64
        b64=$(echo "$response" | jq -r '.result.transaction[0] // empty' 2>/dev/null)
        if [ -n "$b64" ]; then
            mkdir -p "$BIN_DIR" "$QR_DIR"

            echo "$b64" | base64 -d > "$BIN_DIR/$name.bin"
            local size
            size=$(wc -c < "$BIN_DIR/$name.bin")

            # Generate labeled QR image from the base64 payload
            if check_qr_deps 2>/dev/null; then
                generate_qr "$name" "$b64"
                echo "OK: $name → .bin ($size bytes) + .png"
            else
                echo "OK: $name → .bin ($size bytes) [no QR — missing deps]"
            fi
            return 0
        fi

        local err
        err=$(echo "$response" | jq -r '.error.message // empty' 2>/dev/null)
        if [[ "$err" == *"Too many requests"* ]] && [ "$attempt" -lt "$retries" ]; then
            sleep "$delay"
            delay=$((delay * 2))
            continue
        fi

        echo "FAIL: $name ($sig)" >&2
        echo "      ${err:-$response}" >&2
        return 1
    done
}

generate_qr() {
    local name=$1 b64=$2
    local qr_tmp
    qr_tmp=$(mktemp /tmp/qr_XXXXXX.png)

    echo -n "$b64" | qrencode -o "$qr_tmp" -s 10 -m 2 -l L 2>/dev/null || {
        echo "  WARN: QR too large for $name, skipping" >&2
        rm -f "$qr_tmp"
        return 0
    }

    local conv
    conv=$(convert_cmd)
    "$conv" "$qr_tmp" \
        -gravity South -background white -splice 0x36 \
        -gravity South -font Courier -pointsize 24 -annotate +0+8 "$name" \
        "$QR_DIR/$name.png" 2>/dev/null

    rm -f "$qr_tmp"
}

search() {
    local program=$1 limit=${2:-5}

    echo "Recent transactions for $program:"
    curl -s "$RPC" -X POST -H "Content-Type: application/json" -d "{
        \"jsonrpc\": \"2.0\", \"id\": 1,
        \"method\": \"getSignaturesForAddress\",
        \"params\": [\"$program\", {\"limit\": $limit}]
    }" | jq -r '.result[] | "  \(.signature)  \(.err // "ok")"'
}

all() {
    echo "Fetching curated transactions..."
    echo "RPC: $RPC"
    if check_qr_deps; then
        echo "QR generation: enabled"
    else
        echo ""
        echo "QR generation: disabled (continuing without QR images)"
    fi
    echo ""

    local ok=0 fail=0

    while IFS=$'\t' read -r name sig; do
        [[ "$name" =~ ^#.*$ || -z "$name" ]] && continue
        if grab "$name" "$sig"; then
            ok=$((ok + 1))
        else
            fail=$((fail + 1))
        fi
        sleep 3
    done < "$(dirname "$0")/signatures.tsv"

    echo ""
    echo "Done: $ok ok, $fail failed"

    generate_ur
}

generate_ur() {
    local ur_dir="$BASEDIR/generate_ur"
    if ! command -v cargo >/dev/null 2>&1; then
        echo "UR generation: disabled (cargo not found)" >&2
        return 0
    fi
    echo ""
    echo "Generating UR sequences..."
    cargo build --release --manifest-path "$ur_dir/Cargo.toml" --quiet 2>&1 || {
        echo "  WARN: failed to build generate-ur" >&2
        return 0
    }
    "$ur_dir/target/release/generate-ur"
}

# ── Program IDs for search ───────────────────────────────────────────────────

programs() {
    cat <<'EOF'
# ── Supported ────────────────────────────────────────────────────────────────
System               11111111111111111111111111111111
Token                TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
Token-2022           TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
Stake                Stake11111111111111111111111111111111111112
AssocToken           ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bVU
ComputeBudget        ComputeBudget111111111111111111111111111111
Memo                 MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
Jupiter              JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4
Raydium AMM          675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
Raydium CLMM         CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK
Raydium CPMM         CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C
# ── Not yet supported ───────────────────────────────────────────────────────
Orca Whirlpool       whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc
Marinade             MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD
Jito Staking         Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Posu
Metaplex Metadata    metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s
Tensor               TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN
Magic Eden           M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K
EOF
}

# ── CLI ──────────────────────────────────────────────────────────────────────

case "${1:-}" in
    grab)
        [[ $# -lt 3 ]] && { echo "Usage: $0 grab <name> <signature>" >&2; exit 1; }
        grab "$2" "$3"
        generate_ur
        ;;
    search)
        [[ $# -lt 2 ]] && { echo "Usage: $0 search <program_id> [limit]" >&2; exit 1; }
        search "$2" "${3:-5}"
        ;;
    all)
        all
        ;;
    programs)
        programs
        ;;
    *)
        echo "Usage:"
        echo "  $0 grab <name> <signature>    Save a transaction by signature"
        echo "  $0 search <program_id> [n]    Find recent tx signatures"
        echo "  $0 all                        Fetch all from signatures.tsv"
        echo "  $0 programs                   List known program IDs"
        ;;
esac
