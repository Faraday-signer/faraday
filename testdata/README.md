# Test Transactions

Real Solana transactions captured from mainnet for testing the parser.

## How it works

1. **`signatures.tsv`** — list of transaction signatures to capture, one per line, organized by program. Each entry has a name and a signature separated by a tab.
2. **`capture.sh`** — fetches transactions from a Solana RPC endpoint and saves them as:
   - `test_txs_bin/*.bin` — raw transaction bytes (input to `parser::parse()`)
   - `test_txs_qr/*.png` — labeled QR images encoding the base64 transaction (for testing with the device camera)
   - `test_txs_ur/<name>/` — UR-encoded animated QR sequences (for testing with low-res cameras):
     - `frame_001.png`, `frame_002.png`, ... — individual QR frames (for unit tests and `simulator_no_cam`)
     - `<name>.gif` — animated GIF with infinite loop (for testing with the physical device)
     - `<name>.txt` — raw UR strings, one per line (for testing the UR decoder directly without camera)
3. **`cargo test`** — the test `test_real_transactions_parse_without_panic` reads every `.bin` file and runs it through the full parser pipeline, printing the decoded output.
4. **Optional `<name>.expected`** sidecar — if a file with this name sits next to `<name>.bin`, the test reads it as a `key=value` file and asserts the listed predicates against the parsed transaction. Useful for locking in classifier behavior on real fixtures so regressions surface as test failures.

   Supported keys:
   - `primary_program=Jupiter` — `primary_instruction()` must select an ix with this program label.
   - `not_primary_program=AssocToken` — primary must **not** be this program (catches the classic "Create Token Account hijacks the hero" bug).
   - `hero_title=SWAP` — the `@H1` hero row built by `build_review_lines` must equal this value (`SWAP` / `TRANSFER` / `ACTION` / etc).
   - `hero_contains=SOL` — any hero row (`@H1` / `@H2` / `@HM` / `@SWAPPAIR`) must contain this substring. Less brittle than `hero_title` for asserting "the dest mint resolved".

   Lines starting with `#` are comments. Unknown keys are skipped (with a log line) so future expectations don't break old fixtures.

   Example: `test_txs_bin/jupiter_swap_with_create_ata.expected`
   ```
   # Regression: ATA-create side-effect must not eclipse the swap
   primary_program=Jupiter
   not_primary_program=AssocToken
   hero_title=SWAP
   hero_contains=SOL
   ```

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
- **Optional (for UR sequences):** `cargo` (Rust toolchain)

If optional dependencies are missing, `capture.sh` still generates `.bin` files and skips the corresponding step.

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

Both `test_txs_bin/`, `test_txs_qr/`, and `test_txs_ur/` are in `.gitignore`. Each developer runs `./capture.sh all` to populate them.

## UR generation details

The `generate_ur/` directory contains a standalone Rust project that encodes transactions as [Uniform Resource (UR)](https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-005-ur.md) fountain-coded QR sequences. This is built and run automatically by `capture.sh`.

- Fragment size: 100 bytes (produces QR codes of ~49×49 modules, readable by low-res cameras)
- Animation speed: 200ms per frame
- Fountain codes allow the decoder to reconstruct the full payload without needing every frame in sequence
