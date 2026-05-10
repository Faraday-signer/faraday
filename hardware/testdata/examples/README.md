# Example Transactions

Synthetic demo transactions committed to the repo so the Sign TX flow can be
exercised without an RPC roundtrip or mainnet capture. Regenerate with:

```
cargo run --features simulator --bin gen-test-tx
```

## `self_transfer.{png,bin}`

- **From / To:** `GAthe6Gh8xEuJobQWB3cLUBFjsGtyvsk7Y3BeQMkMsfT` (self-transfer)
- **Mnemonic (account 0, no passphrase):** `warm stage brain flag busy bless
  situate fox push crouch caution direct`
- **Amount:** 10,000,000 lamports (0.01 SOL)
- **Instructions:** 1× System::Transfer
- **Blockhash:** placeholder (`0xAB * 32`) — **not submittable**; for UI demos only.

Load the mnemonic on Faraday (Load Wallet → Enter Words), then Main Menu →
Sign TX → scan `self_transfer.png` from your Mac screen. Device shows the
review screen (`Transfer 0.01 SOL to self`), signs, and emits the signed TX QR.
