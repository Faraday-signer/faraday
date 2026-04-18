# Proposal: typed QR envelopes

**Status:** draft, awaiting firmware review
**Scope:** protocol between the Faraday device firmware and the Faraday browser
extension. No user-facing behaviour specified here.
**Owner (extension side):** @cxalem
**Owner (firmware side):** _unassigned_

## Problem

The Faraday device firmware and the Faraday extension exchange state over
QR codes. Multiple distinct payloads flow across that channel — a pairing
pubkey, an unsigned transaction, a signed transaction — and today none of
them carry a type tag. The payloads are distinguished only by their shape
(length, base58 vs base64, whether `bs58.decode()` returns 32 bytes, etc.).

Shape-based discrimination is a protocol smell:

- **Parsing is heuristic.** The extension runs `isValidSolanaAddress` on
  whatever it scans, falls back to a base64 decode, then a Solana-tx
  envelope parse. A payload that happens to be 32 decodable base58 bytes
  but is not actually an address passes the address check.
- **Wrong-mode scans are silently accepted.** If the extension's pairing
  scanner is pointed at a QR emitted from the sign flow (or an unrelated
  QR with the right shape), the only thing stopping it is content
  inspection. There is no envelope-level check that says "this QR was
  emitted for a different purpose."
- **Every new flow broadens the parser.** Receive, multi-account switch,
  seed-QR flows will each add another shape to the heuristic stack. The
  parser's blast radius grows with every feature.

These are not UX bugs — they are protocol-design gaps. The surface happens
to also be visible in the UI (scanner shows a generic error) but the fix
belongs at the protocol layer.

## Proposal

Introduce a URI-scheme-based envelope for every QR exchanged between the
device and the extension. Each envelope starts with `faraday:<kind>:` and
is followed by the typed payload. One exception: receive addresses
continue to use the ecosystem standard `solana:<address>` because they are
scanned by third-party wallets that we do not control.

### Envelopes

| Direction | Flow | URI shape |
|---|---|---|
| device → extension | Pair device | `faraday:pair:<base58 pubkey>` |
| device → third-party wallet | Receive | `solana:<base58 pubkey>` |
| extension → device | Sign transaction (unsigned payload) | `faraday:unsigned:<base64url>` |
| device → extension | Signed transaction (response) | `faraday:signed:<base64url>` |

Payload encodings:

- **base58** for public keys — standard 32-byte Solana address encoding.
- **base64url** (RFC 4648 §5) for binary payloads — URL- and filename-safe,
  no padding. Preferred over vanilla base64 because `-` and `_` don't
  conflict with URI delimiters.

### Required behaviour

Both ends:

1. **Emit only tagged envelopes** for the flows above. Bare pubkey and bare
   base64 forms are deprecated on the wire.
2. **Refuse, at the envelope level, any QR whose `kind` does not match the
   current scan context.** Pair scanner accepts `faraday:pair:*`; sign
   scanner on the device accepts `faraday:unsigned:*`; signed-response
   scanner in the extension accepts `faraday:signed:*`. Rejection happens
   before any content parsing.
3. **Do not fall back to untagged shapes in new builds.** A legacy
   fall-through is permitted for a defined migration window (see Migration)
   and must be explicitly documented, not implicit.

Versioning (optional for v0, recommended for v1):

- Single-byte version prefix inside the payload, e.g. `faraday:unsigned:v0.<base64url>`.
- Absent version prefix in v0 is treated as implicit v0.
- v1+ must include the prefix.

## Why this matters

- **Envelope-level type safety.** A sign-flow scanner can reject a pair QR
  at prefix match time, without ever invoking the Solana transaction
  parser. The parser's input domain shrinks from "any QR-decodable string"
  to "any string prefixed with the exact kind we expect."
- **Defense in depth against adversarial QRs.** A malicious QR carefully
  crafted to match an address shape but behave differently when later
  used as something else cannot cross flow boundaries.
- **No heuristic stacking as flows are added.** Each new flow declares its
  own envelope kind. Parsers gain exactly one match arm, not a new shape
  to disambiguate against all existing shapes.
- **Trivially cheap.** The emit side is a string prefix. The receive side
  is a literal-string match followed by the existing payload parse. No
  cryptography, no new dependencies, no schema registry.
- **Debuggable out of band.** A captured QR string tells you what it is
  without running a parser. Logs, screenshots, and support workflows stay
  readable.

## Non-goals

- **No cryptographic signing or encryption of the envelope itself.** The
  existing cryptographic guarantees (transaction signatures, on-device
  visual review) are unchanged. The envelope is a type tag, not a
  capability token.
- **No binary framing.** Stays text so `cat`, logs, screenshots, and
  `tail -f` remain useful. Multi-frame binary formats (UR, BBQr) are a
  separate concern that can layer on top if QR capacity ever becomes a
  bottleneck.
- **No change to the Receive QR.** `solana:<addr>` is the ecosystem
  standard; interop with third-party wallets is a feature.
- **No user-facing UI specification.** Whatever copy the extension or the
  device shows when a kind mismatch is detected is the surface owner's
  choice. This proposal defines only the wire shape and the rejection
  rule.

## Migration

1. **Firmware v0.N** ships the tagged envelope on all flows under its
   control. The extension is already forward-compatible (see "Extension
   readiness" below), so firmware v0.N can ship without a coordinated
   extension release.
2. **Optional deprecation window.** For one release, each scanner MAY
   accept the legacy untagged shape with a runtime log noting the
   fallback path. This is a strict temporary migration affordance, not a
   permanent behaviour.
3. **Firmware v0.N+1** removes legacy fall-throughs where applicable.
   Signed-tx scanners on the device side stop accepting bare base64.

No flag day, no lockstep release.

## Open questions for firmware owner

1. **Version prefix at v0 or v1?** Carrying the byte now costs nothing;
   deferring it means one breaking change later if any envelope needs to
   evolve.
2. **base64url vs base64 for binary payloads.** QR alphanumeric mode is
   more density-efficient on `[A-Z0-9 $%*+\-./:]`. Worth benchmarking
   scan density on-device before locking the choice.
3. **Should the envelope carry a device fingerprint** (e.g. a 4-byte hash
   of the device's boot-time public key) so the extension can detect
   "this is a different Faraday than the one you paired"? Not required
   for v0, but worth reserving space in the envelope syntax.

## Acceptance criteria

Firmware ships tagged envelopes when:

- [ ] Pair screen renders `faraday:pair:<base58 pubkey>`.
- [ ] Signed-tx screen renders `faraday:signed:<base64url>`.
- [ ] Sign-tx scanner rejects anything that is not `faraday:unsigned:<base64url>`.
- [ ] Receive screen (if implemented) renders `solana:<addr>`.
- [ ] Envelope format is documented in the firmware repo (`docs/qr-format.md`
      or equivalent), including any version prefix choice.

## Extension readiness

The extension-side parser at
`extension/entrypoints/sidepanel/screens/pair-scan.tsx` → `parsePairScan()`
already:

- Accepts `faraday:pair:<addr>` as the preferred form.
- Rejects `faraday:sig:*`, `faraday:signed:*`, `faraday:unsigned:*`,
  `faraday:tx:*` at the envelope level (these are not valid pair input,
  regardless of inner content).
- Accepts `solana:<addr>` and bare base58 as migration fallbacks.

The signed-response scanner in the sign flow will gain the equivalent
prefix check once this proposal is accepted.
