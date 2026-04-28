# Faraday — Pitch Deck Design Brief

> Use this as the briefing doc for any design help (Claude design-taste skill, a designer, another AI session). Everything decided so far, plus open items.

---

## 1. The product

**Faraday** is the open-source signing system for Solana. Three taglines we use:

- **Tagline (title slide)**: *"Offline. Unhackable. Open source."*
- **Sub-tagline**: *"The most secure signer in the world. Built for Solana."*
- **Positioning**: *"We don't sell hardware. We sell the whole system."*

It's a **signer**, not a wallet. The distinction matters: a wallet stores keys; a signer signs and forgets. The device holds nothing when off.

### The stack (4 pieces)

| Piece | What it does | Status |
|---|---|---|
| **Device** | Pi Zero + screen + camera, runs Faraday firmware in Rust. Always offline. Reads QR codes, shows the real transaction in plain English, signs offline. | ✅ Working |
| **Browser extension** | Wallet-Standard compatible. Plug-and-play with Phantom, Squads, Jupiter, Drift, every Solana app. | ✅ Working |
| **Team dashboard** | Multisig payroll, batch payments, role permissions, per-team audit log. Each approver signs on their own Faraday. | ✅ Working |
| **Reverse indexer** | Cloudflare Worker mapping `member_pubkey → multisigs`. Sub-100ms. Solves a gap Squads itself doesn't expose publicly. | ✅ Working |

### The signing flow (end-to-end)

```
1. User initiates a tx (in any Solana app or in the dashboard for team payments)
2. Browser extension renders the unsigned tx as a QR code
3. User scans the QR with their Faraday device
4. Device decodes the tx — shows recipient, amount, program, in plain English
5. User reviews on the device screen, presses approve
6. Device produces a signed-response QR (never goes online)
7. User scans the response QR back into the extension
8. Extension submits the signed tx to Solana mainnet
```

For shared accounts (multisig):
- Same flow per approver
- Dashboard shows proposal status (X of Y approvals)
- Once threshold met, anyone can execute on-chain

### Real proof point

A 2-of-3 multisig was created and a real payment was executed end-to-end through the Faraday stack on Solana mainnet. Every step was rendered in plain English on the device.

---

## 2. Audience & goal

- **Event**: Solana Frontier Hackathon 2026 (Colosseum)
- **Format**: 3-minute pitch
- **Audience**: hackathon judges — non-tech-savvy expected, mixed crypto familiarity
- **Goal**: Grand Champion ($30K) or top 20 ($10K each)
- **Date**: ~today (April 27, 2026), event runs April 6 – May 11

### Tone rules

- Plain English, short sentences
- Declarative, punchy
- Replace jargon: "air-gapped" → "offline"; "stateless" → "unhackable"; "multisig" → "shared account" or "team account"; "hot wallet" → "online wallet"
- Avoid: "BoM", "SMB", "PMF", "TAM/SAM/SOM" acronyms
- Bold claims OK when defensible (open source = anyone can verify)

---

## 3. The brand

### Color palette

| Token | Hex | Use |
|---|---|---|
| `--bg` | `#0a0e14` | Slide background (very dark, near-black) |
| `--surface` | `#11161d` | Cards, elevated panels |
| `--elevated` | `#161c25` | Modals, hover states |
| `--border` | `#21262d` | Subtle dividers |
| `--border-strong` | `#2d343d` | Stronger dividers |
| `--fg` | `#e6edf3` | Body text (off-white) |
| `--muted` | `#8b949e` | Secondary text |
| `--dim` | `#6e7681` | Tertiary, captions |
| `--accent` | `#1af8ff` | **Faraday cyan — brand color**, used sparingly for emphasis |
| `--accent-soft` | `rgba(26,248,255,0.10)` | Background tint |
| `--success` | `#3fb950` | Positive states |
| `--warning` | `#d29922` | Pending |
| `--danger` | `#f85149` | Errors / strikethrough red |

### Typography

- **Sans (body)**: Inter (400, 500, 600, 700, 800)
- **Mono (eyebrows, numbers, addresses)**: JetBrains Mono (400, 500, 600, 700)
  - We tried Departure Mono (the brand's pixel-art font from extension/playground). Dropped it for the deck — too retro for VC audiences. JetBrains Mono is the deck font.
- **Inter** for everything else: titles, body, labels.

### Brand mark

A simple bracket-and-dot icon — corner brackets framing a square in the center. Cyan stroke + cyan fill. Matches the Faraday hardware mark (extracted from SVG bitmap, 60×10 grid).

### Visual signature moves

- Mono uppercase eyebrows as slide titles (e.g., **THE PROBLEM**, **MARKET**)
- Big sans-serif declarative headlines
- Numbers in mono (the $285M figure on the Problem slide is the hero)
- Strikethrough as rhetorical move (e.g., "Online wallets ~~can~~ **WILL** be hacked.")
- Cards with subtle `accent-soft` left border or top border for emphasis

---

## 4. The deck (6 slides, 3 minutes)

### Slide 1 — Title (~12s)
- Brand mark + "Faraday"
- Tagline: **Offline. Unhackable. Open source.**
- Sub-tagline: *The most secure signer in the world. Built for Solana.*

**Notes**: Open quietly. Pause two beats after the tagline. Sub-tagline lands the whole pitch in one sentence.

### Slide 2 — The Problem (~45s)
- Eyebrow: **THE PROBLEM**
- Lead headline: **Online wallets ~~can~~ WILL be hacked.** (with red strikethrough on "can", cyan uppercase "WILL")
- Body: *"Anything connected gets hacked eventually. AI keeps getting smarter — fast, and it never stops trying. Even hardware wallets aren't safer — most of the time, you're still signing blind."*
- Hero card: **$285,000,000 — Drift Protocol drained, April 2026, 26 days ago. Two of five team members tricked into approving what they could not read. Second-largest hack in Solana history.** (Sources: Bloomberg, Chainalysis, TRM Labs, BlockSec)
- Closing: **Every signer today is vulnerable.**

### Slide 3 — The Solution (~45s)
- Eyebrow: **THE SOLUTION**
- H2: *We don't sell hardware. We sell the whole system.*
- Pitch line: *"A small device that never connects to the internet, a browser app that talks to every Solana platform, a dashboard for company accounts, and an indexer that finds them in milliseconds. Together they show you, in plain English, exactly what you are about to approve — before you approve it."*
- Four-card grid: The signer · Browser app · Team dashboard · Account finder
- Four principles: Always offline · Forgets when off · Open source · Built in Rust

### Slide 4 — Market (~30s)
- Eyebrow: **MARKET · BOTTOM-UP**
- H2: *Where the money lives — and where it's exposed.*
- Funnel:
  - **560M** people hold crypto worldwide (820M active wallets)
  - **$565M** hardware wallet sales in 2025 (~6M units, growing 18%/yr)
  - **$10B** *(beachhead, highlighted)* already in shared accounts on Solana — 300+ companies, DAOs, treasuries
- Closer: *"The category is empty. Ledger and Trezor were built for Bitcoin. None of them speak Solana."*

### Slide 5 — Business Model (~35s)
- Eyebrow: **BUSINESS MODEL**
- H2: *Three streams. One company.*
- Pitch: *"We sell the **whole system** — devices for the signers, software for the team. Hardware brings them in. Recurring software is where the business compounds."*
- Streams card: Device $129/unit · Team subscription $49–$199/mo · Custom integrations $5K–$15K/project
- Projection card: Year 1 $200K → Year 2 $1.2M → Year 3 $5M
- Note: *"By year 3: ~1,500 paying teams, ~20,000 devices sold. Capturing a fraction of Solana's 300+ shared-account teams."*

### Slide 6 — Team & Ask (~25s)
- Eyebrow: **TEAM & ASK**
- H2: *The people building it. What we want.*
- Three team cards: **Nahem** (founder · hardware & firmware) · **Ale** (engineering · TBD) · **Javi** (engineering · TBD)
- Ask band: **Frontier Grand Champion + 5 design partners** — looking for Solana-native teams to run real signing volume.
- Contact: GitHub, email, demo URL

---

## 5. Verified facts (with sources)

| Claim | Source |
|---|---|
| Drift Protocol drained $285M, April 1, 2026, 2-of-5 multisig | [Bloomberg](https://www.bloomberg.com/news/articles/2026-04-01/solana-based-defi-project-drift-hit-by-285-million-exploit) · [Chainalysis](https://www.chainalysis.com/blog/lessons-from-the-drift-hack/) · [TRM Labs](https://www.trmlabs.com/resources/blog/north-korean-hackers-attack-drift-protocol-in-285-million-heist) · [BlockSec](https://blocksec.com/blog/drift-protocol-incident-multisig-governance-compromise-via-durable-nonce-exploitation) |
| Solana Foundation security overhaul, April 7, 2026 | [CoinDesk](https://www.coindesk.com/tech/2026/04/07/solana-foundation-unveils-security-overhaul-days-after-usd270-million-drift-exploit) |
| Address poisoning: $3.4B in 2025, 80,000 victims | [MEXC News](https://www.mexc.com/en-GB/news/318254) |
| Squads: $10B+ secured, 300+ teams (incl. Helium, Jito, Pyth) | [Squads.xyz](https://squads.xyz/multisig) · [Fystack research](https://fystack.io/blog/squads-from-zero-to-the-multisig-protocol-securing-10b-on-solana) |
| Hardware wallet market: $565M in 2025, 6M units/yr | [CoinLaw market stats](https://coinlaw.io/hardware-wallet-market-statistics/) |
| 560M crypto holders globally, 820M active wallets | [CoinLaw adoption stats](https://coinlaw.io/cryptocurrency-wallet-adoption-statistics/) |

---

## 6. What still needs to be filled

1. **Team bios** — slide 6 has placeholders. Need:
   - Nahem: 1 line of credibility
   - Ale: which area (extension? dashboard? security?) + 1 line
   - Javi: which area + 1 line
2. **Real URLs** on slide 6 — currently placeholders:
   - GitHub (we used `github.com/nseguias/faraday`)
   - Email
   - Demo URL
3. **Revenue projection sanity-check** — happy with $200K → $1.2M → $5M as the public claim?
4. **Demo video** (strong recommendation) — 30-60s of the device on screen would massively boost the pitch. Currently no demo slide; can add one if a video is recorded.

---

## 7. Notes on what works / what designers should preserve

- **The strikethrough on the Problem slide** — *"~~can~~ **WILL**"* is rhetorically strong. Keep it.
- **The $285M hero card** — let it dominate. It's the slide's center of gravity.
- **The bottom-up funnel** on the Market slide reads well — three clean tiers with the beachhead highlighted in cyan.
- **One idea per slide.** Resist the temptation to add more text. The deck is currently tight at 6 slides for 3 minutes.
- **Keep the brand mark monochrome cyan** on dark — don't introduce additional accent colors.
- **Mono is for numbers and labels, sans-serif is for thoughts.** Don't mix.

---

## 8. What the deck looks like in code

The deck is a single self-contained HTML file at `pitch-deck-20260427-134212.html` in the repo root. Renders in any browser, prints to PDF cleanly, has keyboard navigation (`←` `→`), presenter notes (`N`), fullscreen (`F`), print (`P`).

Design system is inline (CSS custom properties for the palette, typographic scale via `clamp()`). Each slide is a `<section class="slide" data-slide="N">`. Iteration is fast — just edit a section.

Font assets:
- Inter via Google Fonts (CDN)
- JetBrains Mono via Google Fonts (CDN)
- Departure Mono (currently unused but @font-face declared, file at `assets/DepartureMono-Regular.woff2`)
