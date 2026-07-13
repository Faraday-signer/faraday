---
date: 2026-07-13
slug: chrome-extension-expert
---

# Chrome extension expert agent

Added `.claude/agents/chrome-extension-expert.md` (Opus) after the Web Store rejected the extension for the unused `tabs` permission (violation ref "Purple Potassium"; fixed in PR #75) — the second permission-related friction after PR #70's `host_permissions` drop.

The agent's job is to find the rejection before Google does:

- **7-gate pre-submission audit** — permissions vs actual API usage (on the *built* manifest, `.output/chrome-mv3/`), MV3 remote-code ban, privacy/data disclosures, packed-build functionality, single purpose + metadata, obfuscation, deception.
- **Rejection decoder** — Google's color+element violation reference IDs, with the Faraday-relevant subset inlined and an instruction to verify against the live troubleshooting page (policies drift).
- **Faraday facts baked in** — intended permission set `["storage","sidePanel"]`, the `<all_urls>` content-script justification (Wallet Standard provider shim), camera-not-mic invariant (`audio: false`), the keys-never-touch-the-extension trust story as review posture.

Process rule going forward: run this agent before every store submission and on any PR that touches `wxt.config.ts` or the manifest.

Verified: agent definition only; taxonomy cross-checked against developer.chrome.com/docs/webstore/troubleshooting on 2026-07-13.
