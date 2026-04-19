# Test Transactions

Real Solana transactions captured from mainnet for testing the parser.

## How it works

1. **`signatures.tsv`** — list of transaction signatures to capture, one per line, organized by program. Each entry has a name and a signature separated by a tab.
2. **`capture.sh`** — fetches transactions from a Solana RPC endpoint and saves them as:
   - `test_txs_bin/*.bin` — raw transaction bytes (input to `parser::parse()`)
   - `test_txs_qr/*.png` — labeled QR images encoding the base64 transaction (for testing with the device camera)
3. **`cargo test`** — the test `test_real_transactions_parse_without_panic` reads every `.bin` file and runs it through the full parser pipeline, printing the decoded output.

## Quick start

```bash
cd testdata

# 1. Find signatures for a program
./capture.sh search JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4

# 2. Grab one by hand
./capture.sh grab jupiter_swap 4xYk...signature...

# 3. Or add it to signatures.tsv (uncomment + paste) and fetch all at once
./capture.sh all

# 4. Run tests
cd ..
cargo test test_real_transactions -- --nocapture
```

## Dependencies

- **Required:** `curl`, `jq`, `base64`
- **Optional (for QR images):** `qrencode`, `imagemagick`

If QR dependencies are missing, `capture.sh` still generates `.bin` files and prints the install command for your OS.

```bash
# Linux
sudo apt-get install -y qrencode imagemagick

# macOS
brew install qrencode imagemagick
```

## capture.sh commands

| Command | Description |
|---------|-------------|
| `grab <name> <signature>` | Fetch a single transaction and save as `.bin` + `.png` |
| `search <program_id> [n]` | List recent transaction signatures for a program (default: 5) |
| `all` | Fetch all uncommented entries from `signatures.tsv` |
| `programs` | List known program IDs (supported and not yet supported) |

## Custom RPC endpoint

The public mainnet RPC has rate limits. Set `SOLANA_RPC` to use a different endpoint:

```bash
export SOLANA_RPC=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
./capture.sh all
```

## Adding a new test case

1. Find a transaction signature on Solana Explorer or with `./capture.sh search`
2. Add a line to `signatures.tsv`: `name<TAB>signature`
3. Run `./capture.sh all`
4. Run `cargo test test_real_transactions -- --nocapture` to verify it parses correctly

## What the test checks

- The transaction deserializes without panic
- The message structure is valid (not an "Error" result)
- At least one instruction is parsed
- The decoded output is printed for visual inspection

Transactions from unsupported programs (Orca, Marinade, etc.) are expected to parse as "Unknown" — that's correct behavior. The test verifies that the parser handles them gracefully rather than crashing.

## Generated files are not committed

Both `test_txs_bin/` and `test_txs_qr/` are in `.gitignore`. Each developer runs `./capture.sh all` to populate them.
