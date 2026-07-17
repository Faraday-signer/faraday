<div align="center">
  <img src="raspberry-pi/assets/brand/faraday-logo.svg" alt="Faraday" width="320">
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

- **The signer** (`raspberry-pi/`) — Rust firmware for the air-gapped device, also runnable as a desktop simulator
- **The browser extension** (`extension/`) — Wallet Standard companion that relays dapp signing requests over QR
- **The mobile app** (`mobile/`) — Watch-only wallet for the Solana Seeker phone with QR-relay signing

## How it works

The whole life of a key on Faraday — from birth to use to death — happens in a single diagram. **No part of it touches a network.**

```
 [Phone / Laptop]                      [Faraday (air-gapped)]
       |                                        |
       |                                        |  1. POWER ON
       |                                        |
       |                                        |  2. CREATE or LOAD wallet
       |                                        |     Keys live ONLY in RAM
       |                                        |
       |  3. Build unsigned transaction         |
       |  4. Display as QR code          -----> |  5. Scan QR with camera
       |                                        |  6. Display transaction details
       |                                        |  7. User reviews & approves
       |  9. Scan signed QR back         <----- |  8. Sign & display signed QR
       | 10. Submit to Solana network           |
       |                                        |
       |                                        | 11. POWER OFF
       |                                        |     ☠ seed wiped from RAM
       |                                        |
       |  Private key NEVER crosses this gap    |
```

Every transaction is **decoded and shown in human terms** before signing — Jupiter swaps, Raydium swaps, SPL transfers, stake operations, Anchor program calls, all parsed offline without touching an RPC. See [Transaction parser](#transaction-parser).

### A note on importing existing seeds

Faraday's **LOAD** flow exists so you can restore a Faraday-created seed onto a new device (or after a power cycle). It is **not** a path for migrating a Phantom / Solflare / Backpack wallet onto Faraday.

If a seed phrase was ever generated on, displayed on, or typed into an internet-connected device, treat it as already known to attackers — current or future. The whole point of Faraday is that your seed was *born* offline and stays offline. Loading an online-born seed onto Faraday gives you the air-gapped *signing* protection but leaves the *seed itself* compromised; the moment that wallet holds anything worth taking, an attacker who's been sitting on the seed can drain it from anywhere.

If you have a wallet that's already been online and you want to move to Faraday: create a fresh wallet on Faraday, transfer your assets to its address, and retire the old seed.

## What's in this repo

| Path | What it is |
|------|------------|
| [`core/`](core) | Shared platform-agnostic library (`faraday-core`). Crypto, parser, signer, QR, GUI state machine, and UI widgets — used by all hardware targets. |
| [`raspberry-pi/`](raspberry-pi) | Rust firmware for the Pi Zero — also runs as a desktop simulator (`cargo run --features simulator`) for development. Self-contained crate with its own `Cargo.toml`, assets, and test data. |
| [`esp32-common/`](esp32-common) | Shared firmware for the ESP32-S3 board family (`esp32-common`). Camera, QR decode, BOOT power button, and the main event loop — used by every ESP32-S3 board. |
| [`esp32-touch2/`](esp32-touch2) | Board firmware for the Waveshare ESP32-S3-Touch-LCD-2. Touch-driven UI on a 240×320 display. |
| [`opt/`](opt) | Buildroot recipe that produces the Pi OS image. See [`opt/README.md`](opt/README.md) |
| [`extension/`](extension) | Chromium browser extension — Wallet Standard companion that relays dapp signing requests to Faraday over QR. See [`extension/README.md`](extension/README.md) |
| [`mobile/`](mobile) | React Native + Expo wallet for the Solana Seeker phone (work in progress). See [`mobile/README.md`](mobile/README.md) |
| [`playground/`](playground) | Vite devnet dapp for exercising the extension end-to-end. See [`playground/README.md`](playground/README.md) |
| [`site/`](site) | Next.js marketing site (separate deploy target) |
| [`scripts/`](scripts) | Helper scripts (e.g. `fetch_alt.py` to capture frozen Address Lookup Tables) |

## Hardware

Faraday supports two hardware platforms. The Pi Zero 1.3 has no radio silicon at all — a physical air gap. The ESP32-S3 has WiFi/BT on the die; Faraday links no radio drivers into the firmware and a CI `nm` symbol audit verifies none are present. For a physical air gap, choose the Pi.

### Raspberry Pi Zero 1.3

| Component | Model | Purpose |
|-----------|-------|---------|
| Computer | Raspberry Pi Zero 1.3 | No WiFi/BT chip on this revision (v1.3, **not** Zero W) |
| Display | Waveshare 1.3" LCD HAT | 240×240, ST7789, 3 buttons + joystick |
| Camera | Pi Camera v1.3 (OV5647) | QR code scanning + entropy capture |

Total cost: **~$35**. Any Pi works — including ones with WiFi, if you already own one. The original Zero v1.3 is recommended because the network chip simply isn't there, so there's nothing to misconfigure or trust to "off".

### Waveshare ESP32-S3-Touch-LCD-2

| Component | Model | Purpose |
|-----------|-------|---------|
| MCU | ESP32-S3R8 | Dual-core Xtensa, 8MB PSRAM, no WiFi enabled |
| Display | 2.0" IPS LCD | 240×320, ST7789T3 via SPI |
| Input | CST816D capacitive touch | Tap zones + swipe gestures |
| Camera | OV2640 | QR code scanning + entropy capture |
| Power | Single-cell Li-ion (JST) | Charge-level gauge via ADC; BOOT button for on/off |

Total cost: **~$30**. A single self-contained board with display, touch, and camera. WiFi/BT radios exist on the chip but are never initialized in Faraday firmware — the binary simply doesn't call the WiFi driver. For maximum paranoia, the Pi Zero 1.3 remains the gold standard (no radio hardware at all).

## Quick start (desktop simulator)

You don't need any hardware to try Faraday — the same code that runs on the Pi runs on macOS/Linux/Windows with a webcam.

```bash
cd raspberry-pi
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

### Windows (WSL)

For Windows, working in WSL is recommended, but the Windows camera isn't available inside it. To work around this:

- **File camera:** From `raspberry-pi/`, use `cargo run --features simulator_no_cam` — opens a JPEG, PNG, or animated GIF each time the camera is triggered.
- **USB/IP:** Attach the camera to WSL with [usbipd-win](https://github.com/dorssel/usbipd-win), then use `--features simulator` normally.
- **Native Windows build:** Compile on Windows pointing to the WSL repo path (`\\wsl$\Ubuntu\home\…\faraday`) — the camera works directly.

## End-to-end demo (simulator + extension + playground)

Run all three locally — no Pi needed — to see the full sign flow.

**1. Simulator:**
```bash
cd raspberry-pi
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

## Getting a Pi OS image

The OS is a minimal Buildroot Linux that boots straight into the Faraday binary, with no networking, no shell on the framebuffer, and a read-only root.

### Option A: download a pre-built image

Each tagged release publishes a `faraday_os.<version>.pi0.img.zip` plus a SHA256SUMS file under [GitHub Releases](https://github.com/Faraday-signer/faraday/releases). Verify the hash before flashing — the `Build Pi Zero image` workflow run linked from each release shows exactly how the artifact was produced.

```bash
unzip faraday_os.<version>.pi0.img.zip
shasum -a 256 -c faraday_os.<version>.sha256.txt
just flash DEVICE=/dev/diskN
```

On Windows, unzip and write the `.img` with [Raspberry Pi Imager](https://www.raspberrypi.com/software/) → "Use custom image".

### Option B: build it yourself

You should do this if you care about supply-chain integrity at all. The pre-built image is a convenience; the source is the source of truth.

```bash
# 1. Cross-compile the ARM binary
cargo install cargo-zigbuild
cd raspberry-pi && cargo zigbuild --release --target arm-unknown-linux-gnueabihf && cd ..

# 2. Build the OS image (uses Docker — first build takes ~30 min, cached rebuilds are fast)
docker compose up

# 3. Flash to SD card (find your device with `diskutil list` first)
just flash DEVICE=/dev/diskN
```

Image lands at `images/faraday_os.pi0.img`. See [`opt/README.md`](opt/README.md) for what's inside the OS, what's stripped out, and how to customize it.

## Building the ESP32-S3 firmware

### Prerequisites (one-time setup)

```bash
# 1. Install the Espressif Rust toolchain
cargo install espup
espup install

# 2. Source the environment (add to your .bashrc to make it permanent)
. ~/export-esp.sh

# 3. Install the linker proxy and flash tool
cargo install ldproxy
cargo install espflash

# 4. On Debian/Ubuntu, ensure python3-venv is available (needed by ESP-IDF)
sudo apt install python3-venv
```

### Build

```bash
just esp-touch2
```

The first build takes several minutes — it downloads ESP-IDF v5.3.2 and compiles the standard library for Xtensa. Subsequent builds are fast.

### Flash and monitor

Connect the board via USB, then:

```bash
just esp-touch2-flash
```

This flashes the firmware and opens a serial monitor. The board boots into the same UI as the Pi — splash screen, then main menu.

### Flashing from Windows (when building in WSL)

USB serial devices aren't accessible from WSL, so flash from the Windows side:

```powershell
# Install espflash on Windows (one-time, requires native Windows Rust)
cargo install espflash

# Flash directly from the WSL build output (adjust your WSL distro/username)
espflash flash --monitor \\wsl$\Ubuntu\home\<user>\development\faraday\target\xtensa-esp32s3-espidf\release\faraday-esp32-touch2
```

Alternatively, copy the binary first:
```powershell
copy \\wsl$\Ubuntu\home\<user>\development\faraday\target\xtensa-esp32s3-espidf\release\faraday-esp32-touch2 C:\temp\faraday-esp32-touch2.bin
espflash flash --monitor C:\temp\faraday-esp32-touch2.bin
```

### Touch controls

| Gesture | Action |
|---------|--------|
| Swipe up/down/left/right | Navigate (Up/Down/Left/Right) |
| Tap body area | Confirm |
| Tap bottom bar — left third | Back |
| Tap bottom bar — center | Secondary action |
| Tap bottom bar — right third | Confirm |
| Long press (>1.5s) | Wipe wallet (kill switch) |

### Power

The ESP32-S3-Touch-LCD-2 has no battery hardware (it runs off USB), so there is no charge gauge or battery icon. Battery support lives in the shared `esp32-common` crate (the `BoardBattery` trait) for other ESP32-S3 boards that do have a pack.

**Power button (on/off).** The BOOT button (GPIO0) doubles as a soft power button:

| Action | Result |
|--------|--------|
| Long-press BOOT (≥1.5s) while on | Wipes the in-memory wallet, then deep-sleeps — "off" |
| Press BOOT while asleep | Wakes via a full reset, boots fresh to the first screen — "on" |

Power-off wipes the seed/keys from RAM *before* sleeping, so nothing sensitive is retained while the device is off. Wake is a deep-sleep power-cycle reset, so the firmware reboots cleanly (RAM cleared) and the USB-Serial/JTAG re-initializes — the board stays flashable over USB after an off→on cycle without needing the physical RESET button. See [`esp32-common/src/power.rs`](esp32-common/src/power.rs).

## `just` commands

```
just sim          # Run the desktop simulator
just arm          # Cross-compile the ARM binary for Pi Zero
just image        # Build the full Pi OS image (cold Buildroot — slow)
just image-fast   # Rebuild reusing warm Buildroot state
just flash DEVICE # Flash to SD card (DEVICE=/dev/diskN)
just esp-touch2       # Build the ESP32-S3-Touch-LCD-2 firmware
just esp-touch2-flash # Flash ESP32-S3-Touch-LCD-2 and open serial monitor
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
1. Create `core/src/parser/<program>.rs` with `pub fn parse(data, accounts) -> ParsedInstruction`
2. Register the program ID in `core/src/parser/programs.rs`
3. Add a match arm in `dispatch()` in `core/src/parser/mod.rs`

## QR payload format

QR codes carry base64-encoded payloads. A single prefix byte determines the type:

| First byte | Type | Payload |
|------------|------|---------|
| `0x00`–`0xFE` | Transaction | Standard Solana serialized transaction (legacy or v0) |
| `0xFF` | Sign Message | Arbitrary message bytes (remaining bytes after the prefix) |

Transactions use no prefix — the first byte is `num_signatures` (typically `0x01`), which is always a valid transaction header. The `0xFF` prefix is reserved for messages because no valid transaction can have 255 signatures.

For payloads that exceed a single QR's capacity, Faraday uses [UR](https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-005-ur.md) (Uniform Resource) animated QR streams.

## Security model

1. **No network hardware (Pi).** The Pi Zero 1.3 has no WiFi/Bluetooth chip — not "disabled", physically absent. The ESP32-S3 has radios on-die but Faraday links no radio drivers, verified by a CI `nm` symbol audit — a firmware-enforced air gap, not a physical one. For a physical air gap, choose the Pi.
2. **RAM-only keys.** Seeds never touch persistent storage. Power off = keys gone. The OS rootfs is read-only.
3. **Verifiable transactions.** Full decoded details shown on-screen before signing. The user is the final approval.
4. **Open source, reproducible.** All firmware, OS recipe, and companion apps are auditable. Cross-compile + Buildroot make the build deterministic given the same toolchain.
5. **Minimal surface.** No web server, no daemon, no SSH, no shell on the framebuffer.
6. **Pre-sign risk analysis** (in the browser extension) catches drainer patterns — unlimited approvals, ownership changes, token impersonators, simulated balance drops — before the signing QR is even displayed.

## BIP39 wordlist

The wordlist is **not bundled** in the repo. At build time, `core/build.rs` fetches it directly from the canonical [bitcoin/bips](https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt) repository and verifies the SHA256 checksum. If the hash doesn't match, the build fails. No trust required — verify the constant in `core/build.rs` yourself.

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
