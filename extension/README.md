# Faraday Extension (MVP)

Browser extension companion for the air-gapped Faraday device.

MVP behavior:
- Wallet Standard `connect`
- Wallet Standard `signTransaction` (single tx)
- QR relay only (unsigned tx QR -> Pi signs -> signed tx QR back)
- Pre-sign transaction risk analysis (fraud detection before the QR is shown)

## Security Model

- Extension stores only:
  - paired Solana pubkey (watch-only)
  - approved dapp origins
  - the wallet's durable-nonce account **address** (public data; the ephemeral keypair that creates it signs once and is never stored)
- Extension does **not** store seeds or private keys.
- Signing still happens only on the offline Faraday device.
- Every transaction is simulated and analyzed for fraud signals before the signing QR is generated.

## Transaction Risk Analysis

Before displaying the signing QR, the extension simulates the transaction on-chain and runs a suite of fraud detectors. The user sees a risk report (SAFE / WARNING / DANGER) with plain-English warnings and a breakdown of expected balance changes.

The fraud detector is based on [SolDecode](https://github.com/jvr0x/soldecode-extension), an open-source Solana transaction security tool.

### Fraud signals detected

**Structural (instruction-level)**

| Signal | Severity | Description |
|--------|----------|-------------|
| Unlimited Token Approval | DANGER | `Approve` / `ApproveChecked` with amount = `u64::MAX` — grants a program unlimited permission to spend the token forever |
| Token Account Ownership Change | DANGER | `SetAuthority` with `AccountOwner` type — transfers control of a token account to a foreign address |
| Mint Authority Change | DANGER | `SetAuthority` changing the mint authority of a token — enables unlimited supply inflation |
| Freeze Authority Change | DANGER | `SetAuthority` changing the freeze authority — enables locking victims out of their token accounts |
| Account Close to Foreign Address | WARNING | `CloseAccount` sending rent SOL to an address that is not the signing wallet |
| Oversized Priority Fee | WARNING | Compute budget price that would result in ≥ 0.05 SOL in priority fees — drainers sometimes siphon SOL this way |

**Simulation-based (pre/post balance diff)**

| Signal | Severity | Description |
|--------|----------|-------------|
| Possible Token Drain | DANGER | Any token balance drops ≥ 95% of the pre-transaction amount |
| Possible SOL Drain | DANGER | SOL balance drops ≥ 95% when the account holds ≥ 0.1 SOL |
| Multiple Tokens Leaving Wallet | DANGER | 3 or more distinct tokens exit the wallet in a single transaction — classic drainer pattern |
| High Value Transfer | WARNING | Net SOL outflow ≥ 10 SOL |
| Transaction Would Fail | DANGER | Simulation returns an error — the transaction cannot succeed on-chain in its current state |

**Token metadata (impersonator detection)**

| Signal | Severity | Description |
|--------|----------|-------------|
| Impersonator Token | DANGER | An incoming token's symbol resolves to a canonical ticker (USDC, USDT, SOL, JUP…) but its mint address does not match the official one. Catches homoglyph spoofing: Cyrillic `С` instead of Latin `C`, fullwidth `ＵＳＤＣ`, zero-width characters, Greek confusables, and exact symbol clones |

### Balance change display

Token symbols are resolved via the [Jupiter token API](https://tokens.jup.ag) and displayed alongside the net balance change per asset (e.g. `-100.00 USDC`, `+0.50 SOL`). This lets users verify that what the simulation shows matches what they expect to send.

### Behavior on analysis failure

If the RPC is unreachable or the simulation times out (15 s), the extension shows a WARNING instead of blocking the user. A failed analysis never prevents signing — the hardware device remains the final safety check.

## MVP Limitations

- Single-account pairing (one pubkey)
- Single transaction per `signTransaction` request
- Single QR payload only (no animated/chunked QR yet)
- Chrome/Chromium first

## Development

```bash
cd extension
npm install
npm run dev
```

Load the unpacked extension from WXT output in Chrome.

## Playground (Recommended Test Loop)

A Vite + React devnet playground lives at the repo root in `playground/`.

```bash
cd ../playground
npm install
npm run dev
```

Then open <http://localhost:4173> and:

1. Open extension popup and pair your Faraday pubkey.
2. Click **Connect** and approve origin access.
3. (Optional) Click **Airdrop 1 SOL** on devnet.
4. Click **Sign + send transfer**:
   - unsigned tx QR appears in Faraday sign window,
   - scan on Pi/simulator and approve,
   - scan signed QR back in the sign window,
   - playground broadcasts to devnet and logs the explorer URL.

## Flow

1. Pair a Solana pubkey in the extension popup.
2. Dapp calls `connect` -> origin approval prompt appears.
3. Dapp calls `signTransaction`.
4. Sign window opens and displays unsigned tx as QR.
5. Scan on Faraday device, approve, scan signed QR back in the sign window.
6. Extension returns signed tx bytes to the dapp.
