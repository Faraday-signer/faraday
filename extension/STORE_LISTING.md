# Chrome Web Store submission — Faraday

Paste-ready copy for the Developer Dashboard. Each heading names the dashboard field it fills.
Source of truth for the privacy text is `extension/PRIVACY_POLICY.md`.

Last reviewed: 2026-07-27

## Item name

Faraday

## Summary (Store listing → Summary, 132 char max)

Air-gapped Solana QR relay wallet for browser dapps.

> Identical to the `description` field in `wxt.config.ts`. Keep the two in sync — a mismatch between the manifest and the listing is an avoidable review flag.

## Category

**Productivity.** Chrome has no Finance category for extensions. Tools was considered and rejected: this is an end-user wallet, not developer tooling.

## Language

English (United States)

## Single purpose (Privacy → Single purpose)

Faraday's single purpose is to let a user connect to Solana dapps in their browser and approve transactions with an air-gapped hardware signer, relaying every transaction to and from that device as a QR code.

## Detailed description (Store listing → Description)

Faraday is the browser half of an air-gapped Solana wallet. Your private keys live on a separate Faraday device that has no Wi-Fi, no Bluetooth, no NFC, and no USB data path — they never touch this extension, your browser, or your computer. When a dapp asks you to sign something, the extension renders the transaction as a QR code, you review it on the device's own screen, and you scan the signed result back in with your camera. The extension only ever holds your public address, which means a compromised browser can show you a wrong balance but it cannot move your funds.

Faraday connects to dapps through the Solana Wallet Standard, so sites that already support Phantom, Backpack, or Solflare work without any changes. There are no Faraday servers: for all balance, transaction, and token-metadata requests the extension talks directly to a public Solana RPC endpoint and to Jupiter's public token APIs. Token logo images are loaded from the URLs published in each token's on-chain metadata (see the privacy policy). No account, no email address, no analytics, no telemetry, no tracking. Every transaction is decoded and shown to you twice — once in the extension, once on the device — before anything is signed.

Requires the Faraday hardware signer. The extension is a companion to that device and cannot sign on its own.

## Permission justifications (Privacy → Permissions justification)

### `storage`

Faraday keeps a small amount of wallet state on the user's own machine. `chrome.storage.local` holds the watch-only Solana public key the user paired from their hardware device, the list of dapp origins the user has explicitly approved for connection, recent recipient addresses used to warn about mistyped destinations, and display preferences such as whether to show unverified tokens. `chrome.storage.session` holds pending signing sessions, which expire after five minutes and are discarded when the browser closes. No private key or seed material is ever written. Approved origins, recipient history, preferences, and signing sessions are never transmitted anywhere; the stored public wallet address is used in the RPC and Jupiter requests disclosed in the data-usage section, since reading balances from the blockchain requires it. The storage exists only to avoid asking the user to re-pair and re-approve on every page load.

### `sidePanel`

The extension's own wallet interface (balances, the send flow, transaction review, the signing QR, settings, and device pairing) renders in a side panel via `chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true })`, so the user can keep the dapp visible while reviewing and approving. Dapp-initiated connection and signature approvals open in a separate popup window. The sidePanel permission allows the first behavior; without it the user cannot access the extension's own wallet interface.

### `content_scripts` — `<all_urls>` at `document_start`

Solana dapps discover wallets through the Wallet Standard, which requires a provider object to be registered on `window` before the page's own wallet-detection code runs. The content script therefore runs at `document_start` and matches all URLs — the same mechanism used by Phantom, Backpack, and Solflare. It is a passive shim that does exactly two things: it injects `inpage.js`, and it relays connect and sign messages between that script and the extension's background service worker. It does not read page content, the DOM, form fields, cookies, or browsing activity, and it sends nothing to us. The match pattern cannot be narrowed because a Solana dapp can be hosted on any origin, and the user still has to approve every origin and every individual signature.

### `web_accessible_resources` — `inpage.js` for `<all_urls>`

`inpage.js` is the Wallet Standard provider that the content script injects into the page's own JavaScript context; that injection is the only way a dapp is able to see the wallet at all. It has to be web-accessible from any origin for exactly the same reason the content script matches any origin — Solana dapps are hosted on arbitrary domains, so this match pattern cannot be narrower than the content script's. The injected script exposes only wallet methods (connect, disconnect, signTransaction, signMessage, signIn) and forwards each one to the extension for explicit user approval. It also registers the standard:events feed to notify the dapp of wallet-state changes. It does not read the page.

### Not requested

The extension requests no `host_permissions` and no `tabs` permission. Camera access is not a manifest permission: extension pages request it per-origin through the browser's standard `getUserMedia` prompt, always with `audio: false`, and only on the extension's own pages.

## Data usage (Privacy → Data usage)

### What leaves the device

- The user's **public** Solana wallet address and transaction data are sent to a Solana RPC endpoint in order to read balances, simulate transactions before signing, and broadcast signed transactions. The default endpoint is the public `https://api.mainnet-beta.solana.com`; it is configurable at build time via `VITE_RPC_URL` (see `extension/.env.example`).
- Token mint addresses are sent to Jupiter's public APIs (`lite-api.jup.ag`, `tokens.jup.ag`) to resolve token symbols, USD prices, and the verified-token list used to flag airdrop spam.
- Token logo images are loaded from whatever URL the token's own metadata points to, which may be any third-party host.
- Nothing is sent to any server operated by Faraday. There are no Faraday servers.

### Data type disclosures

| Category | Collected | Note |
| --- | --- | --- |
| Personally identifiable information | No | No name, email, address, phone, or ID is ever requested or stored. |
| Health information | No | — |
| Financial and payment information | **Yes** | A public on-chain wallet address and public transaction data are transmitted to a third-party Solana RPC endpoint to read balances and broadcast transactions. No card numbers, bank details, credit data, or identity data are involved, and none of it reaches Faraday. |
| Authentication information | No | No accounts, passwords, or credentials exist. Signing keys never leave the hardware device. |
| Personal communications | No | — |
| Location | No | — |
| Web history | No | Approved dapp origins are stored in `chrome.storage.local` and never transmitted off the device. |
| User activity | No | No analytics, telemetry, clicks, mouse tracking, or usage metrics of any kind. |
| Website content | No | The content script injects a provider and relays wallet messages; it never reads page text, the DOM, or form fields. |

> Chrome defines "collect" as transmitting data off the user's device. Data written to local or session storage and never sent anywhere is disclosed above as not collected, and the reasoning is given per row.

### Certifications

- [x] I do not sell or transfer user data to third parties, outside of the approved use cases.
- [x] I do not use or transfer user data for purposes that are unrelated to my item's single purpose.
- [x] I do not use or transfer user data to determine creditworthiness or for lending purposes.
- [x] My item complies with the Chrome Web Store **Limited Use** policy. Data leaves the device only for the single disclosed purpose of reading balances and simulating and broadcasting the user's own transactions.

## Privacy policy URL

https://faraday.to/privacy

The published text is generated from `extension/PRIVACY_POLICY.md`, which is the source of truth. The page is served by `site/app/privacy/page.tsx`.

## Screenshot shot list

Required: 1–5 screenshots, **1280×800** PNG or JPEG (24-bit, no alpha). Capture the real UI — no mockups, no rendered comps.

1. **Onboarding / pairing** — `entrypoints/sidepanel/screens/onboarding.tsx` into `pair-scan.tsx`. Show the side panel asking the user to scan the device's pairing QR, with the camera frame live. `pair-paste.tsx` is the fallback if a camera capture is awkward.
2. **Side-panel home** — `entrypoints/sidepanel/screens/home.tsx`, captured next to a real dapp so the side-panel context reads. SOL balance plus a few SPL tokens with symbols and USD values.
3. **Send review** — `entrypoints/sidepanel/screens/send-review.tsx`. Recipient, amount, fee, and the risk report (SAFE / WARNING / DANGER) with the expected balance changes.
4. **QR sign screen** — `entrypoints/sidepanel/screens/send-sign.tsx` (or `entrypoints/sign/` for the dapp-initiated flow). The animated unsigned-transaction QR being handed to the device, with the "scan the signature back" step visible.

Capture rules:

- Exactly 1280×800. Do not upscale a smaller capture — the store rejects blurry assets.
- Use a demo wallet. No real balances, no real recipient addresses, nothing that identifies a person.
- No marketing text overlaid on the UI beyond a short caption band, and no fake UI elements.
- Keep one theme across all four.

### TODO — promo assets

- [ ] **440×280 small promo tile** (PNG or JPEG). Required for any Chrome Web Store featuring or category placement. Suggested treatment: the Faraday wordmark on the site's background, matching `site/app/page.tsx`.
- [ ] 1400×560 marquee tile — optional, only needed for a featured-carousel pitch.
- [x] 128×128 store icon — already present at `extension/public/icon/128.png`.

## Support and contact

Contact email: javilois@gmail.com (matches `extension/PRIVACY_POLICY.md`).
