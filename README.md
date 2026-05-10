<div align="center">
  <img src="hardware/assets/brand/faraday-logo.svg" alt="Faraday" width="320">
  <p><strong>Open-source, air-gapped Solana signing suite.</strong></p>
  <p>
    <a href="https://github.com/Faraday-signer/faraday/actions/workflows/ci.yml"><img src="https://github.com/Faraday-signer/faraday/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  </p>
</div>

---

## Why Faraday exists

AI is getting smarter every day, and **anything online will eventually be hacked**. Hot wallets get drained. Browser extensions ship malicious updates. The hardware wallets people fall back on are mostly **closed source** — users end up trusting a vendor's firmware, the vendor's update channel, the vendor's RNG, and the vendor's promise that the secure element really is secure. You can't verify any of it.

Faraday flips that. The whole stack is open: signer firmware, OS image recipe, browser extension, mobile companion. You can read the code, build it yourself, and verify that the binary you run is the one you compiled.

The signer device itself has **no antennas**. No WiFi, no Bluetooth, no NFC, no cellular — the Pi Zero 1.3 board doesn't physically contain a network chip. The only channel into or out of Faraday is the camera reading QR codes and the screen displaying them. There is no remote attack surface because there is no remote.

Faraday isn't just a device. It's a suite:

- **The signer** (`hardware/`) — Rust firmware for the air-gapped device, also runnable as a desktop simulator
- **The browser extension** (`extension/`) — Wallet Standard companion that relays dapp signing requests over QR
- **The mobile app** (`mobile/`) — Watch-only wallet for the Solana Seeker phone with QR-relay signing

## How it works

The whole life of a key on Faraday — from birth to use to death — happens in a single diagram. **No part of it touches a network.**

```
                            ┌──────────────────────────────┐
                            │   Faraday (air-gapped Pi)    │
                            │                              │
   CREATE  or  LOAD ──────► │   Wallet                     │
                            │                              │
                            │   ↓                          │
                            │   Keys live ONLY in RAM      │
                            │                              │
   [Phone / Laptop]         │                              │
        |                   │                              │
        | unsigned QR ─►    │   SIGN                       │
        |                   │   ├─ scan QR via camera      │
        |                   │   ├─ decode tx, show details │
        |                   │   ├─ user reviews & approves │
        | signed QR ◄──     │   └─ display signed QR       │
        |                   │                              │
        | broadcast         │                              │
        |                   │                              │
   POWER OFF ────────────►  │   ☠ seed wiped from RAM     │
                            │     no disk, no journal,     │
                            │     no recovery on next boot │
                            └──────────────────────────────┘

   Private key NEVER crosses the air gap. The only durable copy
   of your seed is the one you wrote down on paper.
```

Every transaction is **decoded and shown in human terms** before signing — Jupiter swaps, Raydium swaps, SPL transfers, stake operations, Anchor program calls, all parsed offline without touching an RPC. See [Transaction parser](#transaction-parser).

### A note on importing existing seeds

Faraday's **LOAD** flow exists so you can restore a Faraday-created seed onto a new device (or after a power cycle). It is **not** a path for migrating a Phantom / Solflare / Backpack wallet onto Faraday.

If a seed phrase was ever generated on, displayed on, or typed into an internet-connected device, treat it as already known to attackers — current or future. The whole point of Faraday is that your seed was *born* offline and stays offline. Loading an online-born seed onto Faraday gives you the air-gapped *signing* protection but leaves the *seed itself* compromised; the moment that wallet holds anything worth taking, an attacker who's been sitting on the seed can drain it from anywhere.

If you have a wallet that's already been online and you want to move to Faraday: create a fresh wallet on Faraday, transfer your assets to its address, and retire the old seed.

## What's in this repo

| Path | What it is |
|------|------------|
| [`hardware/`](hardware) | Rust firmware for the Pi Zero — also runs as a desktop simulator (`cargo run --features simulator`) for development. Self-contained crate with its own `Cargo.toml`, assets, and test data. |
| [`opt/`](opt) | Buildroot recipe that produces the Pi OS image. See [`opt/README.md`](opt/README.md) |
| [`extension/`](extension) | Chromium browser extension — Wallet Standard companion that relays dapp signing requests to Faraday over QR. See [`extension/README.md`](extension/README.md) |
| [`mobile/`](mobile) | React Native + Expo wallet for the Solana Seeker phone (work in progress). See [`mobile/README.md`](mobile/README.md) |
| [`playground/`](playground) | Vite devnet dapp for exercising the extension end-to-end. See [`playground/README.md`](playground/README.md) |
| [`site/`](site) | Next.js marketing site (separate deploy target) |
| [`scripts/`](scripts) | Helper scripts (e.g. `fetch_alt.py` to capture frozen Address Lookup Tables) |

## Hardware

| Component | Model | Purpose |
|-----------|-------|---------|
| Computer | Raspberry Pi Zero 1.3 | No WiFi/BT chip on this revision (v1.3, **not** Zero W) |
| Display | Waveshare 1.3" LCD HAT | 240×240, ST7789, 3 buttons + joystick |
| Camera | Pi Camera v1.3 (OV5647) | QR code scanning + entropy capture |

Total cost: **~$35**. Any Pi works — including ones with WiFi, if you already own one. The original Zero v1.3 is recommended because the network chip simply isn't there, so there's nothing to misconfigure or trust to "off".

## Quick start (desktop simulator)

You don't need any hardware to try Faraday — the same code that runs on the Pi runs on macOS/Linux/Windows with a webcam.

```bash
cd hardware
cargo run --features simulator
```

Or from the repo root: `just sim`.

A 240×240 window opens. Pick `CREATE → 12 or 24 WORDS → RANDOM` to generate a wallet with on-screen entropy.

| Key | Hardware button | Action |
|-----|-----------------|--------|
| Arrow keys | Joystick | Navigate |
| Enter / Z | Key1 / JoyPress | Confirm |
| X | Key2 | Secondary action |
| Escape | Key3 | Back / Cancel |

## End-to-end demo (simulator + extension + playground)

Run all three locally — no Pi needed — to see the full sign flow.

**1. Simulator:**
```bash
cd hardware
cargo run --features simulator
```
Create a wallet (`CREATE → 12 or 24 WORDS → RANDOM`) or load an existing one. Leave it running.

**2. Extension** (in a second terminal):
```bash
cd extension
npm install
npm run dev
```
WXT builds to `extension/.output/chrome-mv3/`. Load it in Chrome:
1. Open `chrome://extensions`, toggle **Developer mode** on
2. Click **Load unpacked** → pick `extension/.output/chrome-mv3/`

**3. Playground** (in a third terminal):
```bash
cd playground
npm install
npm run dev
```
Opens at <http://localhost:4173>.

**4. Drive the loop:**
1. Click the Faraday extension icon → **Pair** to your simulator's pubkey (shown on `MAIN MENU → SETTINGS → ADDRESS QR` — scan or copy)
2. In the playground, click **Connect** → approve in the extension
3. Click **Airdrop 1 SOL** (devnet)
4. Click **Sign + send transfer** → unsigned tx QR appears in the extension's sign window
5. Point the simulator's camera at the QR → review → approve → it shows the signed QR
6. Scan the signed QR back in the extension → playground broadcasts → check the explorer link

[`extension/README.md`](extension/README.md) and [`playground/README.md`](playground/README.md) have details and troubleshooting.

## Building the Pi OS image

The OS is a minimal Buildroot Linux that boots straight into the Faraday binary, with no networking, no shell on the framebuffer, and a read-only root.

```bash
# 1. Cross-compile the ARM binary
cargo install cargo-zigbuild
cd hardware && cargo zigbuild --release --target arm-unknown-linux-gnueabihf && cd ..

# 2. Build the OS image (uses Docker — first build takes ~30 min, cached rebuilds are fast)
docker compose up

# 3. Flash to SD card (find your device with `diskutil list` first)
just flash DEVICE=/dev/diskN
```

Image lands at `images/faraday_os.pi0.img`. See [`opt/README.md`](opt/README.md) for what's inside the OS, what's stripped out, and how to customize it.

## `just` commands

```
just sim          # Run the desktop simulator
just arm          # Cross-compile the ARM binary for Pi Zero
just image        # Build the full Pi OS image (cold Buildroot — slow)
just image-fast   # Rebuild reusing warm Buildroot state
just flash DEVICE # Flash to SD card (DEVICE=/dev/diskN)
just ext          # Build the browser extension
just test         # cargo test
just check        # Type-check both simulator and Pi targets
```

## Transaction parser

Faraday decodes Solana transactions before signing so users see human-readable details instead of raw bytes. Both legacy and v0 (versioned) transaction formats are supported. **All decoding happens offline — no RPC.**

| Program | Instructions decoded |
|---------|---------------------|
| System | Transfer, CreateAccount, CreateAccountWithSeed, Allocate, TransferWithSeed |
| SPL Token / Token-2022 | Transfer, TransferChecked, Approve, ApproveChecked, Revoke, MintTo, MintToChecked, Burn, BurnChecked, CloseAccount |
| Stake | Initialize, DelegateStake, Split, Withdraw, Deactivate, Merge |
| Jupiter v6 | Route, RouteV2, SharedAccountsRoute, ExactOutRoute, RouteWithTokenLedger (10 variants) |
| Jupiter Ultra / RFQ | Swap variants with RFQ pricing |
| DFlow | Swap (heuristic decode of trailing footer) |
| Raydium AMM v4 | SwapBaseIn, SwapBaseOut |
| Raydium CLMM | Swap, SwapV2 |
| Raydium CPMM | SwapBaseInput, SwapBaseOutput |
| Associated Token | CreateAccount |
| ComputeBudget | SetComputeUnitLimit, SetComputeUnitPrice |
| Memo | Inline memo text |
| Unknown | Program ID + raw data shown with a warning |

For Jupiter swaps, mints are resolved offline through a hardcoded registry of ~30 well-known tokens (SOL, USDC, JUP, etc.) plus deterministic ATA derivation — no network call. When a mint can't be resolved, a warning is shown instead of a misleading symbol.

To add a new program:
1. Create `hardware/src/parser/<program>.rs` with `pub fn parse(data, accounts) -> ParsedInstruction`
2. Register the program ID in `hardware/src/parser/programs.rs`
3. Add a match arm in `dispatch()` in `hardware/src/parser/mod.rs`

## QR payload format

QR codes carry base64-encoded payloads. A single prefix byte determines the type:

| First byte | Type | Payload |
|------------|------|---------|
| `0x00`–`0xFE` | Transaction | Standard Solana serialized transaction (legacy or v0) |
| `0xFF` | Sign Message | Arbitrary message bytes (remaining bytes after the prefix) |

Transactions use no prefix — the first byte is `num_signatures` (typically `0x01`), which is always a valid transaction header. The `0xFF` prefix is reserved for messages because no valid transaction can have 255 signatures.

For payloads that exceed a single QR's capacity, Faraday uses [UR](https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-005-ur.md) (Uniform Resource) animated QR streams.

## Security model

1. **No network hardware.** Pi Zero 1.3 has no WiFi/Bluetooth chip — not "disabled", physically absent.
2. **RAM-only keys.** Seeds never touch persistent storage. Power off = keys gone. The OS rootfs is read-only.
3. **Verifiable transactions.** Full decoded details shown on-screen before signing. The user is the final approval.
4. **Open source, reproducible.** All firmware, OS recipe, and companion apps are auditable. Cross-compile + Buildroot make the build deterministic given the same toolchain.
5. **Minimal surface.** No web server, no daemon, no SSH, no shell on the framebuffer.
6. **Pre-sign risk analysis** (in the browser extension) catches drainer patterns — unlimited approvals, ownership changes, token impersonators, simulated balance drops — before the signing QR is even displayed.

## BIP39 wordlist

The wordlist is **not bundled** in the repo. At build time, `hardware/build.rs` fetches it directly from the canonical [bitcoin/bips](https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt) repository and verifies the SHA256 checksum. If the hash doesn't match, the build fails. No trust required — verify the constant in `hardware/build.rs` yourself.

## Derivation path

Solana standard: `m/44'/501'/0'/0'` (all hardened, Ed25519/SLIP-0010)

- `44'` — BIP-44 purpose (multi-account HD wallets)
- `501'` — Solana coin type
- `0'` — account index
- `0'` — address index (all hardened because Ed25519/SLIP-0010 doesn't support non-hardened derivation)

Compatible with Phantom, Solflare, and other Solana wallets — the seed you create on Faraday can be restored in any standards-compliant Solana wallet, and vice versa.

## License

Business Source License 1.1 — see [LICENSE](LICENSE).

You may use, copy, and modify the code for non-production purposes (learning, testing, personal use, contributions). Production and commercial use requires a license from the author. The code converts to Apache 2.0 on 2030-04-16.
