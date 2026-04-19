#!/usr/bin/env bash
# Captures Solana transactions as raw bytes for parser testing.
#
# Usage:
#   ./capture.sh grab <name> <tx_signature>    Save a specific transaction
#   ./capture.sh search <program_id> [limit]   Find recent tx signatures for a program
#   ./capture.sh all                           Fetch all curated transactions
#
# Environment:
#   SOLANA_RPC   RPC endpoint (default: mainnet public)

set -euo pipefail

RPC="${SOLANA_RPC:-https://api.mainnet-beta.solana.com}"
OUTDIR="$(cd "$(dirname "$0")" && pwd)/test_txs"

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
            echo "$b64" | base64 -d > "$OUTDIR/$name.bin"
            local size
            size=$(wc -c < "$OUTDIR/$name.bin")
            echo "OK: $name → test_txs/$name.bin ($size bytes)"
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
