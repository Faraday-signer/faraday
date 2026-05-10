# Faraday Mobile

> ⚠️ **Work in progress.** Pairing, balance display, and the QR-relay sign flow work end-to-end on Android. Several pieces are deferred — see [Status](#status) below.

React Native + Expo wallet for the **Solana Seeker** phone (and any modern Android). Watch-only on the device, signs through QR relay to the air-gapped Faraday Pi — same model as the [browser extension](../extension), repackaged for mobile.

The phone never holds a private key. Everything signing-related happens on the Pi.

## Stack

Expo SDK 54 · React Native 0.81 · React 19 · `@solana/kit` · `@react-navigation/*` · AsyncStorage · `react-native-svg` · `react-native-qrcode-svg` · UR animated QR (`@ngraveio/bc-ur`)

Dark theme, Departure Mono font, Faraday brand mark/wordmark ported as `react-native-svg`.

## Status

**Works:**
- Pair via QR scan or paste (handles `faraday:pair:`, `solana:`, and bare base58 envelopes)
- View SOL + SPL balances (Helius DAS primary path, Jupiter verified-meta fallback for public RPC)
- Send SOL or SPL tokens via QR-relay to a Faraday Pi:
  - Build unsigned tx with `@solana/kit` (`getTransferToATAInstructionPlanAsync`)
  - Pre-sign risk analysis (verbatim port of the extension's analyzer)
  - UR-encoded animated QR for large txs
  - Splice `faraday:sig:` envelope into the unsigned tx, validate signature, broadcast
- Recipient history, broadcast error explanations
- Settings tab (device info, network, about)

**Deferred / known limitations:**
- **Requires `EXPO_PUBLIC_RPC_URL`** pointing to a Helius (or similar) RPC. Public mainnet blockhash is too stale for the QR-relay round-trip; broadcasts will simulate-fail. `.env` is gitignored.
- **WebSocket live-balance subscriptions don't work on RN.** `@solana/kit`'s subscriptions package has a structural error on RN; falls back to polling silently after 2 retries.
- **Mobile Wallet Adapter (MWA)** wallet registration not yet wired up — apps can't pick Faraday from the system wallet picker yet.
- **Receive screen** is a placeholder.
- **No tests** in this package yet.

## Run

```bash
cd mobile
pnpm install   # or npm install
```

Create `.env` with your RPC URL:

```bash
EXPO_PUBLIC_RPC_URL=https://mainnet.helius-rpc.com/?api-key=...
```

Then either:

```bash
pnpm run android      # Android device or emulator
pnpm run ios          # iOS simulator (untested as a sign target)
pnpm start            # Expo dev server, choose target from the menu
pnpm run typecheck    # tsc --noEmit
```

## End-to-end sign flow

1. Pair to a Faraday Pi: scan the address QR shown on the Pi's `MAIN MENU → SETTINGS → ADDRESS` screen, or paste it
2. Wallet screen loads SOL + SPL balances
3. Tap **Send** → choose recipient + amount → review risk report
4. App displays the unsigned-tx QR (animated UR if it doesn't fit a single QR)
5. Scan that QR on the Pi → review → approve → Pi shows signed-tx QR
6. Scan the signed QR back in the app → app validates, broadcasts, shows the explorer URL

## Project structure

```
src/
├── components/        # Shared UI (error banner, QR display, risk report, tokens section, etc.)
├── lib/
│   ├── solana.ts      # RPC client + balance fetch (DAS + Jupiter fallback)
│   ├── tokens.ts      # Token list + metadata (~300 lines, mirrors extension)
│   ├── tx-risk.ts     # Pre-sign risk analyzer (port of the extension's)
│   ├── sol-transfer.ts / spl-transfer.ts   # Tx builders via @solana/kit
│   ├── ur-encode.ts   # Animated QR encoding for large txs
│   ├── pair-parser.ts # Accepts faraday:pair:, solana:, bare base58
│   ├── app-state.tsx  # AsyncStorage-backed state (paired pubkey, approved origins)
│   └── ...            # Hooks: use-wallet, use-tokens, use-live-balance
├── navigation/        # Bottom-tab + nested stacks
└── screens/
    ├── pair-scan.tsx / pair-paste.tsx
    ├── wallet.tsx
    ├── send-{compose,review,sign}.tsx
    └── settings/
```
