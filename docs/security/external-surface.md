# External surface map — what could compromise the seeds, QRs, and wallets we produce

Everything outside this repo that participates in producing a seed, a
backup QR, or a signature — with how a compromise would manifest, what
already mitigates it, and what hardening is still open. Companion to the
QR predictability audit (`core/tests/qr_predictability_audit.rs`, PR #129)
and to [`failure-modes.md`](./failure-modes.md), which maps *what could
fail* in depth — this doc answers *who do we trust*, that one answers
*what breaks and how would we notice*.

Threat question, precisely: *which external party, if compromised, could
cause a Faraday device to produce a predictable seed, a wrong backup, or
a signature over something the user didn't approve?*

Last reviewed: 2026-08-16.

## 1. The seed path (firmware, `core` + `esp32-*`)

| dependency | role in the path | compromise manifests as | standing mitigations |
|---|---|---|---|
| `getrandom` → OS RNG / `esp_random` | base entropy for every wallet | predictable seeds (the Coldcard shape) | camera-frame hash XORed in (seed never weaker than the stronger source); boot RNG health checks (PR #119); refuses to run without RNG |
| ESP-IDF v5.3.2 (`esp-idf-sys`, C SDK) | provides `esp_random`, mbedtls SHA-512 for BIP39 seed derivation | subverted RNG conditioning; wrong seed derivation | version-pinned; BIP39 seed-derivation known-answer self-test at every boot (fail-closed); health checks. **Residual: the SDK is a huge C codebase we cannot audit; Espressif release integrity is trusted** |
| Xtensa Rust toolchain (`espup`, esp-rs GitHub releases) | compiles the firmware | arbitrary code in the binary | **trusted on download — no independent verification. Reproducible builds (below) are the systemic answer** |
| RustCrypto crates (`sha2`, `hmac`, `pbkdf2`), `ed25519-dalek`, `zeroize` | hashing, key derivation, signing, secret wiping | wrong keys, leaked secrets, forged signatures | pure Rust, pinned via `Cargo.lock`, high-scrutiny upstream orgs; BIP39 self-test crosses `sha2` against mbedtls at boot |
| BIP39 English wordlist | entropy → words mapping | wrong words → backups restore to a different wallet | **downloaded from `raw.githubusercontent.com` at build time**, verified against a SHA-256 pinned in `core/build.rs` (tamper = build failure). Hardening: vendor the 13 KB file in-repo and delete the network step |
| `bs58`, `base64`, `hex` | address / payload encoding | wrong addresses or payloads displayed | small, pinned; addresses independently checkable against the extension's rendering |
| Cargo registry (crates.io) — 458 locked crates total | everything above, transitively | malicious crate version | `Cargo.lock` pins exact versions + checksums; **no advisory scanning in CI yet** |

## 2. The QR path

| dependency | role | compromise manifests as | standing mitigations |
|---|---|---|---|
| `qrcode` (encode) | renders seed backups, signature envelopes, addresses | a QR that decodes to different bytes than intended | audit suite (PR #129): backup QR must equal exactly the whitened entropy the words encode; decode side is an independent implementation, so encode/decode collusion would need two compromised crates |
| `rxing` / decode stack, `ur`, `minicbor` | reads incoming tx QRs / animated UR streams | parser confusion: device displays a different tx than it signs | review is decoded **from the exact raw bytes that get signed** — display and signature share one input; parser fuzz/test vectors in `core` |
| ALT snapshots (`lookup_tables.rs`, fetched from mainnet RPC) | naming accounts behind address lookup tables | display shows stale/wrong account identities (execution unaffected — the signature covers table *references*, not contents) | mutable-table caveat documented at the const; `fetch_alt.py` refuses non-frozen tables without an explicit override; unresolved entries render as sentinels, never fake addresses (FA-24) |

## 3. The host side (extension, dapp, npm)

The design assumption is that **the host is compromised** — that's why the
device exists. A malicious extension dependency (`@solana/kit` builds
transactions; `tweetnacl`, `@ngraveio/bc-ur`, `@zxing/browser` handle
crypto/QR I/O; ~4,800-line pnpm lockfile transitively) cannot steal keys
(it never sees them) and cannot make the device lie (the device decodes
the raw bytes itself). What it *can* do:

- Build a different transaction than the dapp UI promised — **this is the
  attack the on-device review exists for**; amounts/programs/fees on the
  device screen come from the signed bytes, nothing else.
- Lie in its own UI before/after signing — out of scope by design.

Residual risk concentrates where the device display can't verify:
accounts behind lookup tables (see §2) and any future feature where the
extension supplies display data (the "extension resolves ALTs" follow-up
must label such data as host-supplied).

## 4. Build & release infrastructure

| dependency | role | compromise manifests as | hardening |
|---|---|---|---|
| GitHub Actions (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `esp-rs/xtensa-toolchain@v1.5.3`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `mlugg/setup-zig@v2`, `softprops/action-gh-release@v2`) | CI + release artifacts | poisoned release binaries; neutered radio audit | **actions are pinned by tag, not commit SHA — tags are mutable.** Pin by SHA |
| GitHub itself (repo, releases) | source of truth; future web-flasher manifest (FA-17) | tampered source or artifacts | branch protection; the landing page's "build it and check the hash" claim requires **reproducible builds** to mean anything — fold into FA-17 |

## 5. Hardening backlog (proposed, ranked)

1. **Vendor the BIP39 wordlist** — delete the build-time download; keep the hash check against the vendored file. (Trivial, removes a network trust point.)
2. **Pin every GitHub Action by commit SHA.** (Small; closes tag-mutation.)
3. **`cargo-deny` in CI** — RUSTSEC advisories, source pinning, license gate for all 458 crates; `pnpm audit` equivalent for the extension.
4. **Reproducible firmware builds** — bit-for-bit rebuildable release images, folded into FA-17; this is what makes the site's hash-verification claim real and neutralizes toolchain/CI trust in one move.
5. **ALT snapshot freshness check** — CI warns when a baked mutable table drifts from chain (staleness surfaced in #128).
6. **Golden-vector regression growth** (see §6).

## 6. The regression program (extends PR #129)

What exists: statistical decorrelation, avalanche, determinism, exact
entropy round-trip, mixing uniformity. What to add, in order:

1. **Official BIP39 test vectors** (Trezor's canonical set): entropy →
   words → seed, cross-checked against the spec's published answers — the
   strongest possible guard on the words themselves.
2. **Full-loop QR round trip**: encode with the real encoder, decode with
   the real *decoder* (independent implementation), assert identical
   seed — closes the loop the current suite only walks one way.
3. **Address derivation vectors**: seed → SLIP-0010 → ed25519 → base58
   against externally computed answers, so a derivation regression can't
   ship wallets whose backups restore to different addresses.
