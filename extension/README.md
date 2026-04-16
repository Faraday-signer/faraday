# Faraday Extension (MVP)

Browser extension companion for the air-gapped Faraday device.

MVP behavior:
- Wallet Standard `connect`
- Wallet Standard `signTransaction` (single tx)
- QR relay only (unsigned tx QR -> Pi signs -> signed tx QR back)

## Security Model

- Extension stores only:
  - paired Solana pubkey (watch-only)
  - approved dapp origins
- Extension does **not** store seeds or private keys.
- Signing still happens only on the offline Faraday device.

## Development

```bash
cd extension
npm install
npm run dev
```

Load the unpacked extension from WXT output in Chrome.

## Playground (Recommended Test Loop)

Use the included devnet playground page to test connect + sign flow.

```bash
cd extension/playground
python3 -m http.server 4173
```

Then open `http://localhost:4173` and:

1. Open extension popup and pair your Faraday pubkey.
2. Click **Refresh Wallets** in the playground.
3. Click **Connect** and approve origin access.
4. (Optional) Click **Airdrop 1 SOL** on devnet.
5. Click **Sign + Send Transfer**:
   - unsigned tx QR appears in Faraday sign window,
   - scan on Pi/simulator and approve,
   - scan signed QR back in the sign window,
   - playground broadcasts to devnet and logs explorer URL.

## MVP Limitations

- Single-account pairing (one pubkey)
- Single transaction per `signTransaction` request
- Single QR payload only (no animated/chunked QR yet)
- Chrome/Chromium first

## Flow

1. Pair a Solana pubkey in the extension popup.
2. Dapp calls `connect` -> origin approval prompt appears.
3. Dapp calls `signTransaction`.
4. Sign window opens and displays unsigned tx as QR.
5. Scan on Faraday device, approve, scan signed QR back in the sign window.
6. Extension returns signed tx bytes to the dapp.
