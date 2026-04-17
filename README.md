# Faraday

Air-gapped Solana transaction signer for Raspberry Pi Zero. Pure Rust.

**Your private keys never touch the internet. Ever.**

## How It Works

```
 [Phone / Laptop]                      [Faraday (air-gapped)]
       |                                        |
       |  1. Build unsigned transaction         |
       |  2. Display as QR code          -----> |  3. Scan QR with camera
       |                                        |  4. Display transaction details
       |                                        |  5. User reviews & approves
       |  7. Scan signed QR back         <----- |  6. Sign & display signed QR
       |  8. Submit to Solana network           |
       |                                        |
       |  Private key NEVER crosses this gap    |
```

No WiFi. No Bluetooth. No network. Communication happens exclusively through QR codes.

## Hardware

| Component | Model | Purpose |
|-----------|-------|---------|
| Computer | Raspberry Pi Zero 1.3 | No WiFi/BT chip |
| Display | Waveshare 1.3" LCD HAT | 240x240, ST7789, 3 buttons + joystick |
| Camera | Pi Camera (OV5647) | QR code scanning + entropy capture |

Total cost: ~$35

## Features

- **Wallet Creation**: Generate seeds from dice rolls, coin flips, camera entropy, or device random
- **Passphrase Support**: Optional BIP39 passphrase with confirmation
- **Transaction Signing**: Scan unsigned tx QR, review decoded details per instruction type, approve, display signed QR
- **SeedQR**: Backup/restore seeds as compact QR codes
- **Manual Import**: Enter 12 or 24 BIP39 words via on-screen keyboard
- **Air-gapped**: Keys exist only in RAM, wiped on power off

## Security Model

1. **No network hardware** — Pi Zero 1.3 has no WiFi/Bluetooth chip
2. **RAM-only keys** — Seeds never written to disk. Power off = keys gone
3. **Verifiable transactions** — Full tx details shown before signing
4. **Open source** — All code is auditable
5. **Minimal surface** — No web server, no database, no unnecessary services

## Project Structure

```
src/
├── main.rs               # Entry point (simulator + Pi modes)
├── crypto/               # BIP39 mnemonics, SLIP-0010 derivation, Ed25519
├── gui/
│   ├── app.rs            # App struct, Screen enum, input types, transition dispatcher
│   ├── flows/
│   │   ├── create.rs     # Create wallet flow (word count, entropy, verify, passphrase)
│   │   ├── load.rs       # Load wallet flow (scan QR, enter words, passphrase)
│   │   ├── sign.rs       # Sign TX flow (scan, review, approve, display signed QR)
│   │   └── settings.rs   # Settings flow (address, accounts, export, power off)
│   ├── screens.rs        # Screen rendering (all draw functions)
│   ├── components.rs     # Reusable UI components
│   ├── colors.rs         # Color palette
│   ├── icons.rs          # Icon bitmaps
│   └── framebuffer.rs    # In-memory framebuffer (simulator)
├── hardware/             # ST7789 display driver, GPIO buttons
├── qr/
│   ├── encode_qr.rs      # QR encoding (SeedQR, CompactSeedQR, address, signed tx)
│   └── decode_qr.rs      # QR decoding and type detection
├── parser/
│   ├── mod.rs            # Entry point: parse(tx_bytes) → ParsedTransaction, to_lines()
│   ├── message.rs        # Solana wire format deserializer (legacy + v0 versioned)
│   ├── programs.rs       # Known program ID registry (System, Token, Stake, …)
│   ├── system.rs         # System Program instruction parser
│   ├── token.rs          # SPL Token / Token-2022 instruction parser
│   ├── stake.rs          # Stake Program instruction parser
│   └── unknown.rs        # Fallback parser for unrecognised programs
└── signer/               # Ed25519 transaction and message signing

build.rs                  # Downloads BIP39 wordlist from bitcoin/bips, verifies SHA256
opt/                      # Buildroot OS build system for Pi Zero
```

## Transaction Parser

Faraday decodes Solana transactions before signing so users see human-readable details instead of raw bytes. Both legacy and v0 (versioned) transaction formats are supported.

Recognised programs:

| Program | Instructions decoded |
|---------|---------------------|
| System | Transfer, CreateAccount, CreateAccountWithSeed, Allocate, TransferWithSeed |
| SPL Token / Token-2022 | Transfer, TransferChecked, Approve, ApproveChecked, Revoke, MintTo, MintToChecked, Burn, BurnChecked, CloseAccount |
| Stake | Initialize, DelegateStake, Split, Withdraw, Deactivate, Merge |
| Associated Token | CreateAccount |
| ComputeBudget | SetComputeUnitLimit, SetComputeUnitPrice |
| Memo | Inline memo text |
| Unknown | Program ID + raw data shown with a warning |

To add support for a new program:
1. Create `src/parser/<program>.rs` with `pub fn parse(data, accounts) -> ParsedInstruction`
2. Register the program ID in `src/parser/programs.rs`
3. Add a match arm in the `dispatch()` function in `src/parser/mod.rs`

## Quick Start (Desktop Simulator)

```bash
cargo run --features simulator
```

### Simulator Controls

| Key | Hardware Button | Action |
|-----|----------------|--------|
| Arrow keys | Joystick | Navigate |
| Enter / Z | Key1 / JoyPress | Confirm |
| X | Key2 | Secondary action |
| Escape | Key3 | Back / Cancel |

## Cross-Compile for Pi Zero

```bash
# Install cross-compilation toolchain
cargo install cargo-zigbuild

# Build ARM binary
cargo zigbuild --release --target arm-unknown-linux-gnueabihf
```

## Build OS Image

```bash
docker compose up
```

Image output: `images/faraday_os.pi0.img`

## Flash to SD Card

```bash
diskutil unmountDisk /dev/diskN
sudo dd if=images/faraday_os.pi0.img of=/dev/rdiskN bs=4m status=progress
diskutil eject /dev/diskN
```

## BIP39 Wordlist

The wordlist is **not bundled** in the repo. At build time, `build.rs` fetches it directly from the canonical [bitcoin/bips](https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt) repository and verifies the SHA256 checksum (`2f5eed53...`). If the hash doesn't match, the build fails. No trust required — verify it yourself.

## Derivation Path

Solana standard: `m/44'/501'/0'/0'` (all hardened, Ed25519/SLIP-0010)

- 44' — BIP-44 purpose (multi-account HD wallets)                                                                                                                                                                                       
- 501' — Solana coin type
- 0' — account index                                                                                                                                                                                                                    
- 0' — address index (all hardened because Ed25519/SLIP-0010 doesn't support non-hardened derivation)

Compatible with Phantom, Solflare, and other Solana wallets.

## License

Business Source License 1.1 — see [LICENSE](LICENSE)

You may use, copy, and modify the code for non-production purposes (learning, testing, personal use, contributions). Production and commercial use requires a license from the author. The code converts to Apache 2.0 on 2030-04-16.
