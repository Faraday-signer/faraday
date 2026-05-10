# Example Transactions

Synthetic demo transactions committed to the repo so the Sign TX flow can be
exercised without an RPC roundtrip or mainnet capture. Regenerate with:

```
cargo run --features simulator --bin gen-test-tx
```

## `self_transfer.{png,bin}`

- **From / To:** `HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk` (self-transfer)
- **Mnemonic (account 0, no passphrase):** `abandon abandon abandon abandon
  abandon abandon abandon abandon abandon abandon abandon about` — the canonical
  BIP39 test vector. **Never put a real seed in this file.**
- **Amount:** 10,000,000 lamports (0.01 SOL)
- **Instructions:** 1× System::Transfer
- **Blockhash:** placeholder (`0xAB * 32`) — **not submittable**; for UI demos only.

Load the mnemonic on Faraday (Load Wallet → Enter Words), then Main Menu →
Sign TX → scan `self_transfer.png` from your Mac screen. Device shows the
review screen (`Transfer 0.01 SOL to self`), signs, and emits the signed TX QR.
