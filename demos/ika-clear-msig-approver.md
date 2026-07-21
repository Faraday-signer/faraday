# Ika clear-msig approver demo

This is a draft demo flow for using Faraday as a `clear-msig-ika` approver.

## What this proves

- Faraday signs the exact Ed25519 message bytes that `clear-msig-ika` verifies on chain.
- The browser popup and Pi both display the Solana off-chain message body.
- The Pi still signs the full wrapped message bytes, including the `\xffsolana offchain` header.
- The returned QR is the current Faraday message-signing response: a 64-byte signature encoded as 128 hex characters.

## Byte shape

`clear-msig-ika` signs Solana off-chain messages:

```text
\xffsolana offchain || version(0) || format(0) || len_le_u16 || body
```

The body is the human-readable approval text:

```text
expires 2030-01-01 00:00:00: approve transfer 1000000000 lamports to 9abc... | wallet: treasury proposal: 42
```

Faraday's QR transport adds its own routing byte before those bytes:

```text
0xff || clear_msig_message_bytes
```

So an Ika approval QR starts with two `0xff` bytes after base64 decoding. The first is Faraday's sign-message routing byte. The second is part of the Solana off-chain signing domain and is included in the Ed25519 signature.

## Manual devnet loop

1. Create a `clear-msig-ika` proposal on devnet.
2. Build or fetch the canonical approval message bytes from the CLI/SDK.
3. Ask Faraday to sign those bytes through the existing `signMessage` wallet path.
4. Scan the QR with the Pi.
5. Confirm that the Pi displays the body, not `"(binary data)"`.
6. Approve on the Pi.
7. Scan the 64-byte signature hex QR back into the extension.
8. Submit that signature to `clear-msig-ika`'s `approve` instruction with the correct `expiry` and `approver_index`.

## Current integration gap

`clear-msig-ika` currently signs from its CLI keypair/Ledger path. For a complete demo, add or mock two CLI affordances there:

```bash
clear-msig proposal approval-message --wallet treasury --proposal <PROPOSAL>
clear-msig proposal submit-approval --wallet treasury --proposal <PROPOSAL> --approver-index <N> --signature <HEX>
```

Until that lands, this Faraday branch proves the wallet/device side: message preview, QR transport, exact byte signing, signature validation, and signed-response scanning.
