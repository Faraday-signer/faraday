# Privacy Policy — Faraday Browser Extension

Last updated: July 27, 2026

## Summary

Faraday collects no personal data and has no servers of its own. Your keys and settings stay on your device; the only data that leaves it is public blockchain data — your public wallet address, transactions, and token mint addresses — sent to the Solana RPC endpoint, Jupiter APIs, and token-logo image hosts described under "Network requests" below.

## What data the extension stores

The extension stores the following data locally in your browser using Chrome's storage API:

- Your Solana public key (a watch-only address used to display balances and connect to dapps)
- A list of dapp origins you have approved for wallet connection
- Recent recipient addresses, used to warn you about mistyped destinations
- Display preferences, such as whether to show unverified tokens
- Pending signing sessions, held in session storage only — they expire after five minutes and are discarded when you close the browser

None of this data is ever sent to Faraday — there are no Faraday servers. Approved origins, recipient history, preferences, and signing sessions never leave your browser at all. The one exception is your public wallet address: it is included in the requests to the Solana RPC endpoint and Jupiter APIs described under "Network requests" below (that is how balances and token holdings are read from the blockchain), and it is shared with a dapp when — and only when — you explicitly approve that dapp's connection request.

## What data the extension does not store

- Private keys or seed phrases — signing happens exclusively on the Faraday hardware signer, which never connects to a network (the Pi Zero 1.3 has no radio hardware at all; the ESP32-S3 links no radio drivers)
- Browsing history, page content, or URLs
- Personal information such as name, email, or location
- Analytics, telemetry, or usage metrics

## Network requests

The extension makes the following network requests:

- Solana RPC nodes — to fetch account balances, simulate transactions, and broadcast signed transactions
- Jupiter APIs (lite-api.jup.ag, tokens.jup.ag) — to resolve token symbols, USD prices, and the verified-token list used to flag airdrop spam
- Token logo images — loaded from whatever URL a token's own metadata points to, which may be any third-party host

No data is sent to servers owned or operated by Faraday. No cookies, identifiers, or tracking parameters are included in any request.

## Camera access

The extension requests access to your camera for exactly one purpose: scanning QR codes displayed by the Faraday device (pairing your public address, and reading signed transactions back from the device). Camera access is requested through the browser's standard permission prompt the first time a scanner opens, and only on the extension's own pages — never on websites you visit.

- Video frames are processed locally in your browser to detect QR codes and are discarded immediately; nothing is recorded, stored, or transmitted
- The microphone is never requested
- You can revoke camera access at any time from the browser's extension settings; the extension offers paste-based alternatives where possible

## Third-party services

The extension does not integrate any analytics, advertising, or tracking services.

## Data sharing

Faraday does not sell, rent, or share any user data with third parties. There is no user data to share.

## Data retention

Stored data persists in Chrome's local storage until you remove it by unpairing your wallet or uninstalling the extension. Pending signing sessions live in session storage and are gone when you close the browser. No data is retained elsewhere.

## Children's privacy

The extension does not knowingly collect any information from anyone, including children under 13.

## Changes to this policy

If this policy is updated, the revised version will be posted at the same URL with an updated date. Since we collect no data, meaningful changes are unlikely.

## Contact

If you have questions about this privacy policy, contact us at javilois@gmail.com.
