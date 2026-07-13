# Faraday Verify Protocol — Implementation Plan

**Status:** Draft for review · **Author:** —
**Last updated:** 2026-05-05 (revised after pulling main; reflects post-PR-#61 firmware structure)
**Related tasks:** #10 (security alerts), #11 (sign-message redesign), #13 (durable nonces)
**Triggering event:** April 2026 Drift Council exploit ($285M, durable-nonce + social engineering attack vector)

---

## TL;DR

Helius's Enhanced Transactions API classifies transactions into 138+ types (`SWAP`, `SET_AUTHORITY`, `TRANSFER`, etc.) with structured metadata. The companion fetches this classification and bundles it into the QR alongside the raw transaction bytes. The device verifies that the raw tx **structurally matches** every claim in the metadata — same programs, same accounts, same ATAs (re-derived on-device), same transfer amounts, no extra instructions. If verification passes, the device displays Helius's enhanced view directly (we trust their classifiers when the structure checks out). If anything fails to match, verification fails and the user cannot sign from that surface.

Crucially, **anything in the raw tx that the metadata didn't account for is a verification failure** — that's the Drift-class hidden-authorization catch. The device never shows raw bytes as the main screen; it shows clean human-readable summaries (verified or failed-with-reason).

The device never touches the internet. The companion fetches Helius classification and packs it into the QR. The device verifies independently.

---

## Goal

Make Faraday detect three categories of attack at signing time, on the device, with no internet access:

1. **Compromised companion lying about tx contents** — the wallet UI claims "swap 0.5 SOL for USDC" but the raw tx actually transfers 5 SOL to the attacker. Device displays the truth, not what the UI claims.
2. **Drift-class hidden authorizations** — a transaction looks routine on the surface (durable nonce, normal-looking transfers) but smuggles in a `SetAuthority` / `UpgradeProgram` / admin instruction. Surface the unexplained instruction as a warning.
3. **Schema-level surprises** — durable nonces that pre-sign now and execute weeks later, transactions that span Address Lookup Tables the user didn't expect, etc. Force explicit acknowledgment of risk class before signing.

Non-goal: Faraday does NOT defend against contract bugs, oracle manipulation, governance attacks, or attacks where the user knowingly signs a malicious tx that's correctly classified.

---

## Architecture (one paragraph)

```
┌──────────────┐   1. Build raw tx     ┌──────────────────┐
│  Dapp / dex  │ ─────────────────────▶│   Companion      │
└──────────────┘                       │  (extension /    │
                                       │   phone app)     │
                ┌────────────────────  │                  │
                │ 2. Fetch Helius      │                  │
                │    classification    │                  │
                ▼                      │                  │
        ┌───────────────┐              │                  │
        │  Helius API   │  ───────────▶│                  │
        └───────────────┘              │                  │
                                       │                  │
                                       │ 3. Bundle:       │
                                       │   raw_tx + meta  │
                                       │                  │
                                       │ 4. Encode UR QR  │
                                       └────────┬─────────┘
                                                │
                          QR (animated UR)      │
                                                ▼
                                       ┌──────────────────┐
                                       │  Faraday device  │
                                       │  (air-gapped)    │
                                       │                  │
                                       │ 5. Decode QR     │
                                       │ 6. Parse raw tx  │
                                       │    (own parser)  │
                                       │ 7. Cross-verify  │
                                       │    against meta  │
                                       │ 8. Display       │
                                       │    3-state UI    │
                                       │ 9. User signs    │
                                       │    or rejects    │
                                       └──────────────────┘
```

The device parser is the source of truth. Helius metadata is an *additional input* that lets the device check "does what the companion claims match what's actually in the bytes I see?" — and "is there anything in the bytes the companion didn't claim?"

---

## Design principles

These rules are explicit so the UX work in Phase 5 doesn't drift.

### 1. Never show non-human-readable transactions to the user

The device's main screen is always a clean, human-readable summary — never raw bytes, never instruction dumps. This applies to both verified and failed states.

| State | Main screen | Drill-down view (button) | Why |
|---|---|---|---|
| **Verified** | Helius's enhanced view — "SWAP via Jupiter v6 · −0.5 SOL · +12.3 USDC · Fee 0.000005 SOL" with a verification badge | "Verifiable data": program list, accounts, derived ATAs, all the structural assertions the device confirmed | Match the existing UX — clean main, technical drill-down |
| **Verification failed** | Plain-language failure message — "Verification failed. Do not sign." with the specific reason ("Companion claimed SWAP but transaction transfers 5 SOL to an unverified address") | "Show details": programs, mismatched fields, side-by-side comparison | User knows immediately: don't sign. Detail is for forensics, not for trying to "interpret around" the failure |

The drill-down view never blocks the user — they can always reach the verifiable data — but it's never the primary surface. There is no surface on which the user is asked to interpret raw bytes.

### 2. When verification passes, show Helius's classification — don't second-guess it

Helius's classifiers are more mature than ours and better-maintained. If the raw bytes structurally match Helius's claims (same programs, same accounts, same amounts, no extra instructions), then we trust Helius's semantic interpretation and display it directly.

The device doesn't need to produce its own competing classification of the transaction. `parser::classification` remains useful for the **sign-message flow** (task #11, where Helius doesn't classify off-chain messages), but it is **not** in the critical path for transaction verification.

### 3. Verification is a structural match, not a semantic match

The comparison the device performs is at the byte/instruction/account level:

| What we check | What we don't check |
|---|---|
| Every program Helius claims is invoked → present in raw tx | "Does Helius's category match a category we computed?" |
| Every native transfer Helius claims → matching System.Transfer instruction with same accounts and amount | Whether the meaning of the transaction "fits a swap shape" |
| Every token transfer Helius claims → ATA re-derived from owner+mint matches `claimed_*_ata`, SPL Token instruction matches | Anything semantic — we don't try to interpret intent |
| Every authority change Helius claims → matching `SetAuthority` instruction | |
| **Reverse direction**: every instruction in the raw tx is accounted for by the metadata. Any unaccounted instruction → verification fails. | |

The trust handoff: *"Helius told us this tx does X — and the raw bytes really do X, with nothing extra. So Helius's interpretation is safe to display."*

### 4. Metadata is required — not optional

There are no third-party companions to be backward-compatible with. The Faraday extension, mobile app, and SDK all emit metadata. **A QR without verify metadata is treated as a verification error**, not a graceful degradation. The device shows "Verification metadata missing — do not sign through an outdated or untrusted companion" and refuses to proceed.

This also simplifies the threat model: there is no "no-meta" path for an attacker to exploit by stripping metadata to bypass checks.

---

## Existing primitives we reuse

Critical context: a lot of what this plan assumed we'd build is **already in the repo**. The verify routine is largely an integration layer over existing parts.

| Primitive | Path | Status |
|---|---|---|
| Device-side classifier (`Classification { category, confidence, summary, high_risk }`) | `src/parser/classification.rs` | **Done** — produces `defi_swap`, `stake_deposit`, `security_authority_change`, etc. **Used by the sign-message flow (task #11), NOT in the verify-tx critical path.** Verify-tx is a structural match against metadata, not a category comparison. |
| Known-program registry (`programs::identify`) | `src/parser/programs.rs` | **Done** — Phase 6 just extends this |
| Address Lookup Table expansion | `src/parser/lookup_tables.rs` | **Done** |
| Protocol-specific parsers (Jupiter, Raydium CPMM/AMM/CLMM, DFlow) | `src/parser/{jupiter,raydium/*,dflow}.rs` | **Done** |
| Stake / system / anchor / token parsers | `src/parser/{stake,system,anchor,token}.rs` | **Done** |
| Sign-message decoder | `src/parser/message.rs` | **Done** (task #11 builds on it) |
| Modular UI primitives (card, list, qr widgets, layout, tokens) | `src/ui/` | **Done** — Phase 5 adds a new `verify_screen.rs` next to the existing screens, doesn't touch legacy `src/gui/` |
| Animated UR QR (multi-frame) | `src/qr/ur_decoder.rs` (host-side) + `feat/animated-ur-qr` work | **In progress** — the bigger payload from verify metadata uses this |

Practical impact: **Phases 4 and 6 collapse from "build a new module" to "wire the existing classifier into the verify report and extend the known-program list."** Estimated effort drops accordingly (see updated table below).

---

## QR payload format

Extend the existing prefix-byte scheme:

| First byte | Type |
|---|---|
| `0x00–0xFE` | Transaction (current) |
| `0xFF` | Sign-message (current) |
| `0x01` | **NEW: bundled (raw tx + verify meta)** |

(Note: the new prefix occupies a value already inside the transaction range; this is fine because today the first byte of a transaction is always `num_signatures` which is realistically `0x01`–`0x10`. To avoid collision, we instead use a **trailer-based** approach — see below.)

**Approach: trailer with magic delimiter.** Metadata is **required** — see Design principle #4. The trailer format is a clean way to keep the raw tx bytes contiguous (so existing parsers don't need to be told about the trailer) while still bundling structured metadata in the same QR.

```
┌─────────────────────────────────────────────────────────┐
│  raw_tx_bytes      (Solana serialized tx, legacy or v0) │
│  ─────────────────────────────────────                  │
│  delim_marker      (4 bytes: 0xFA 0xCE 0xFA 0xCE)       │
│  meta_len          (u32 LE)                             │
│  meta_payload      (CBOR-encoded VerifyMeta)            │
└─────────────────────────────────────────────────────────┘
```

Device behavior:
- Read raw tx using normal Solana deserialization (it knows its own length)
- After raw tx, expect the `0xFACEFACE` delimiter
- If present → parse trailer, run verification
- **If absent → verification error.** The device displays *"Verification metadata missing — do not sign through an outdated or untrusted companion"* and refuses to proceed. (See Design principle #4.)

**Coordinated rollout:** because metadata is required, the firmware update and companion update must ship together (or the firmware must be tolerant *only* during the transition window — a config flag we flip off once all companions are upgraded). The deployment plan should treat firmware + companion as a single release.

---

## VerifyMeta schema

Don't ship the full Helius response — too noisy. Distill to the minimum required for verification:

```rust
// Rust side (firmware + sdk)
struct VerifyMeta {
    schema_version: u8,                  // start at 1
    helius_type: TxType,                 // enum, u16
    helius_source: TxSource,             // enum, u16
    expected_native_transfers: Vec<NativeTransferAssertion>,
    expected_token_transfers: Vec<TokenTransferAssertion>,
    expected_authority_changes: Vec<SetAuthorityAssertion>,
    expected_programs: Vec<Pubkey>,      // programs Helius says were invoked
    fee_lamports: u64,                   // claimed fee
    fee_payer: Pubkey,                   // claimed fee payer
    description: Option<String>,         // human-readable, max 80 bytes
}

struct NativeTransferAssertion {
    from: Pubkey,
    to: Pubkey,
    lamports: u64,
}

struct TokenTransferAssertion {
    from_owner: Pubkey,
    to_owner: Pubkey,
    mint: Pubkey,
    amount_raw: u64,                     // pre-decimal
    decimals: u8,
    claimed_from_ata: Pubkey,            // device re-derives and compares
    claimed_to_ata: Pubkey,
}

struct SetAuthorityAssertion {
    account: Pubkey,
    from: Pubkey,
    to: Pubkey,
}
```

```ts
// TS side (extension SDK)
interface VerifyMeta {
  schemaVersion: 1;
  heliusType: TxType;
  heliusSource: TxSource;
  expectedNativeTransfers: NativeTransferAssertion[];
  expectedTokenTransfers: TokenTransferAssertion[];
  expectedAuthorityChanges: SetAuthorityAssertion[];
  expectedPrograms: string[];
  feeLamports: bigint;
  feePayer: string;
  description?: string;
}
```

Approximate size for a typical Jupiter swap: **~150–250 bytes after CBOR**. Adds ~1 frame to existing animated UR QR for swap-class transactions. Acceptable.

---

## Verification routine (device side)

Bidirectional structural match. Returns `Result<VerifyReport, VerifyError>`. Any mismatch is an `Err`. Risk-class hints (durable nonce, ALT) come back inside `Ok(VerifyReport { warnings, .. })` and surface only on the verifiable-data drill-down.

```rust
fn verify(tx: &ParsedTx, meta: &VerifyMeta) -> Result<VerifyReport, VerifyError> {
    // ─── Forward direction: every Helius claim must appear in raw tx ───

    // Programs
    let device_programs = tx.programs_invoked();
    for prog in &meta.expected_programs {
        if !device_programs.contains(prog) {
            return Err(VerifyError::ProgramMissing { claimed: *prog });
        }
    }

    // Fee
    if tx.fee() != meta.fee_lamports || tx.fee_payer() != meta.fee_payer {
        return Err(VerifyError::FeeMismatch { /* ... */ });
    }

    // Native transfers
    for assertion in &meta.expected_native_transfers {
        if !tx.has_native_transfer(assertion) {
            return Err(VerifyError::NativeTransferMismatch { /* ... */ });
        }
    }

    // Token transfers — device independently re-derives ATAs
    for tt in &meta.expected_token_transfers {
        let derived_from = derive_ata(tt.from_owner, tt.mint);
        let derived_to = derive_ata(tt.to_owner, tt.mint);

        if derived_from != tt.claimed_from_ata {
            return Err(VerifyError::AtaDerivationMismatch {
                role: "source",
                claimed: tt.claimed_from_ata,
                derived: derived_from,
                owner: tt.from_owner,
                mint: tt.mint,
            });
        }
        if derived_to != tt.claimed_to_ata { /* same — return Err */ }

        if !tx.has_spl_transfer(derived_from, derived_to, tt.amount_raw) {
            return Err(VerifyError::TokenTransferMismatch { /* ... */ });
        }
    }

    // Authority changes
    for ac in &meta.expected_authority_changes {
        if !tx.has_set_authority(ac.account, ac.from, ac.to) {
            return Err(VerifyError::AuthorityChangeMissing { /* ... */ });
        }
    }

    // ─── Reverse direction (the Drift catch): every instruction in
    //      raw tx must be accounted for by the metadata ───

    let unaccounted = tx.unaccounted_instructions(meta);
    if !unaccounted.is_empty() {
        return Err(VerifyError::UnaccountedInstruction {
            program_ids: unaccounted.iter().map(|ix| ix.program_id).collect(),
        });
    }

    // ─── Risk-class hints (do not fail verification, surface on drill-down) ───

    let mut warnings = vec![];
    if tx.has_advance_nonce_account() {
        warnings.push(VerifyWarning::DurableNonce);
    }
    if tx.uses_address_lookup_tables() {
        warnings.push(VerifyWarning::AddressLookupTable {
            count: tx.alt_count(),
        });
    }

    Ok(VerifyReport {
        helius_type: meta.helius_type,
        helius_source: meta.helius_source,
        description: meta.description.clone(),
        confirmed_programs: meta.expected_programs.clone(),
        confirmed_native_transfers: meta.expected_native_transfers.clone(),
        confirmed_token_transfers: meta.expected_token_transfers.clone(),
        warnings,
    })
}
```

The reverse-direction check is the **Drift-class detection primitive**: Helius parses the visible swap; the hidden `SetAuthority` instruction is unaccounted; the function returns `Err(VerifyError::UnaccountedInstruction)` and the user is shown the failed-screen with the Approve button removed.

---

## Display: two states the user sees (matches existing device UX)

Two states. The main screen is always clean and human-readable — never raw bytes. The verifiable data lives one drill-down away. See Design principle #1.

### Verified — main screen (default surface)
```
SIGN TRANSACTION                            ✓
─────────────────────────────────────────────
  SWAP via Jupiter v6
  
  −0.5 SOL    (your wallet)
  +12.3 USDC  (your wallet)
  
  Fee: 0.000005 SOL
  
  ⓘ See verifiable data
─────────────────────────────────────────────
  [Approve]  [Reject]
```

The header text and effect lines come directly from `VerifyMeta` (Helius's classification). The device has structurally confirmed every claim and trusts the semantic interpretation. No raw bytes, no instruction list, no programIDs on this surface.

### Verified — verifiable data (drill-down behind the ⓘ link)
```
VERIFIABLE DATA
─────────────────────────────────────────────
  Programs (2 of 2 confirmed):
    ✓ Jupiter v6        JUP6LkbZbjS1jKK...
    ✓ SPL Token         TokenkegQfeZ...
  
  Native transfers (1 of 1):
    ✓ −0.5 SOL    your_wallet → router
  
  Token transfers (1 of 1):
    ✓ USDC ATA derivation matches
       owner: your_wallet
       mint:  EPjFW...
       ata:   FdUgr...  (re-derived on device ✓)
       amount: 12.3 USDC
  
  Risk classes:
    Durable nonce:  NO
    ALT used:       NO
─────────────────────────────────────────────
  [← Back]
```

This is the surface for users who want to spot-check the structural assertions. It's always one button-press away in verified state, but it's never the default surface.

### Verification failed — main screen
```
⛔ DO NOT SIGN
─────────────────────────────────────────────
  Verification failed.
  
  The transaction does not match what your
  companion claims it does.
  
  Reason:
  1 instruction in the transaction was not
  explained by the verification metadata.
  
  This is the same shape as the April 2026
  Drift exploit. Reject and rotate keys.
  
  ⓘ See details
─────────────────────────────────────────────
  [Reject]
```

The Approve button is **removed** in this state. The detail link surfaces the specific failure (which programs, which mismatched fields), but the user cannot proceed to sign from this surface.

Failure-mode `Reason:` text (one sentence each, plain language; details in drill-down):

| Failure mode | Main-screen reason text |
|---|---|
| Unaccounted instruction | *"1 instruction in the transaction was not explained by the verification metadata."* |
| Native transfer mismatch | *"A native transfer in the transaction does not match what the companion claims."* |
| Token transfer ATA mismatch | *"A token account in the transaction does not derive from the owner and mint your companion claims."* |
| Token transfer amount/instruction mismatch | *"A token transfer amount or destination does not match what the companion claims."* |
| Program missing | *"A program your companion claims to call is not actually invoked in this transaction."* |
| Authority change missing | *"An authority change your companion claims is not present in the transaction."* |
| Fee mismatch | *"The transaction fee does not match what your companion claims."* |
| Metadata missing | *"Verification metadata missing. Your companion may be outdated or untrusted."* |

---

## Phased rollout

### Phase 0 — Pre-work decisions (no code)
**Deliverable:** Locked decisions on the items in "Open questions" below.
**Effort:** ½ day team discussion.
**Blocks:** Everything else.

### Phase 1 — Schema + wire format
**Scope:** Define `VerifyMeta` Rust struct + TS types. CBOR codec. Magic delimiter handling.
**Files:**
- `src/verify/schema.rs` (new) — Rust types
- `src/verify/codec.rs` (new) — CBOR encode/decode + delimiter detection
- `extension/src/lib/verify-meta.ts` (new) — mirror TS types
- `docs/proposals/verify-protocol.md` (this file) — keep updated as schema evolves
**Deliverable:** Round-trip a `VerifyMeta` through CBOR encode → bytes → decode → struct. Bit-exact.
**Verified by:** Unit tests in both Rust and TS, fuzz test for malformed input.
**Effort:** 1 day.
**Depends on:** Phase 0.

### Phase 2 — Companion: Helius integration + bundling
**Scope:** Extension fetches Helius classification, distills to `VerifyMeta`, appends to raw tx, encodes via animated UR QR.
**Files:**
- `extension/src/lib/helius.ts` (new) — Helius client
- `extension/src/lib/verify-bundler.ts` (new) — distill Helius response → `VerifyMeta`
- `extension/entrypoints/sign/sign-app.tsx` (modify) — wire bundling into the sign flow
- `extension/.env.example` — add `VITE_HELIUS_API_KEY`
**Deliverable:** Extension can take a tx and produce `raw_tx + delim + meta` bytes, encoded as animated UR QR.
**Verified by:** Manual: build a Jupiter swap, run through the extension, scan the QR with a debug tool, decode trailer, confirm `VerifyMeta` matches Helius response.
**Effort:** 2 days.
**Depends on:** Phase 1.

### Phase 3 — Device: meta parsing
**Scope:** Device QR decoder detects delimiter, parses trailer, exposes `VerifyMeta` to the verify routine.
**Files:**
- `src/qr/decode_qr.rs` (modify) — detect trailer
- `src/verify/parser.rs` (new) — parse `VerifyMeta` from trailer bytes
- `src/main.rs` or `src/signer/mod.rs` (modify) — pass meta into verify routine
**Deliverable:** Device receives QR with trailer, returns parsed `VerifyMeta` struct.
**Verified by:** Test fixture: feed a known QR with trailer, assert parsed meta matches expected.
**Effort:** 1 day.
**Depends on:** Phase 1.

### Phase 4 — Device: verification routine (structural match against metadata)
**Scope:** Verify that the raw transaction's instructions, programs, accounts, and amounts structurally match every claim in `VerifyMeta`, and that no instruction in the raw tx is unaccounted for. ATA derivation. Risk-class checks. **No category/semantic comparison** — see Design principle #3.

**Architecture:** the verify routine is a structural matcher, not a classifier:

1. Receive `VerifyMeta` from QR trailer (parsed in Phase 3)
2. Parse raw tx into instructions + accounts (existing `parser::message::deserialize` + `lookup_tables::expand_accounts`)
3. **Forward direction — every Helius claim must appear in the raw tx:**
   - Each `expected_program` → assert it's a `program_id` of some instruction in the tx
   - Each `expected_native_transfer` → find a matching `System.Transfer` instruction (same `from`, `to`, `lamports`)
   - Each `expected_token_transfer`:
     - Re-derive `from_ata` from `(from_owner, mint)` and compare to `claimed_from_ata` (must match)
     - Re-derive `to_ata` from `(to_owner, mint)` and compare to `claimed_to_ata` (must match)
     - Find a matching SPL Token transfer instruction with those ATAs and `amount_raw`
   - Each `expected_authority_change` → find a matching `SetAuthority` instruction
   - Fee + fee payer match
4. **Reverse direction — every instruction in the raw tx must be accounted for by metadata.** Any unaccounted instruction is a verification failure (this is the Drift catch).
5. **Risk-class checks** that don't depend on metadata: `tx.has_advance_nonce_account()`, `tx.uses_address_lookup_tables()`. These are surfaced as warnings on the verifiable-data drill-down even when verification passes.

**The device does not produce its own competing classification.** When verification passes, the user-facing display uses the metadata's `description` / `helius_type` / `helius_source` directly (Design principle #2).

**Files:**
- `src/verify/mod.rs` (new) — module root
- `src/verify/routine.rs` (new) — `verify(tx, meta) → VerifyReport` (forward + reverse direction matching)
- `src/verify/ata.rs` (new) — `derive_ata(owner, mint) → Pubkey` (uses `find_program_address` with ATA program seeds)
- `src/verify/report.rs` (new) — `VerifyReport`, `VerifyError` enum (structured per failure mode), `VerifyWarning` enum for risk classes
- `src/parser/mod.rs` (modify) — expose helpers like `programs_invoked()`, `instructions_iter()`, `find_native_transfer(from, to, lamports)`, `find_spl_transfer(from_ata, to_ata, amount)` if not already public

**No `category_map.rs`** — we don't map between vocabularies because we don't compute a competing category on-device.

**Deliverable:** Given `(tx_bytes, wallet_pubkey, VerifyMeta)`, produce a `VerifyReport` that is either `Ok` (structural match complete) or `Err(VerifyError)` (with the specific failure mode).

**Verified by:** Unit tests covering each `VerifyError` variant. Integration tests with real fixtures (see Phase 7).

**Effort:** 1.5 days *(integration glue, not from-scratch — most parsing primitives already exist).*

**Depends on:** Phase 3.

### Phase 5 — Device UI: two screens (verified main + verifiable-data drill-down + failed)
**Scope:** Add a new `verify_screen.rs` in the modular UI layer with three render modes corresponding to Design principle #1: verified main screen, verifiable-data drill-down, verification-failed screen. Reuse existing widgets.

**Files:**
- `src/ui/screens/verify_screen.rs` (new) — verified main + failed (one screen, two modes)
- `src/ui/screens/verifiable_data_screen.rs` (new) — the drill-down behind the ⓘ link
- `src/ui/screens/mod.rs` (modify) — register both screens
- `src/ui/tokens.rs` (modify) — add status tokens (`token_success`, `token_error`) if not already present
- `src/ui/widgets/{card,list,header}.rs` — reuse without modification where possible
- `src/gui/flows/sign.rs` (modify) — route into the new screen when `VerifyReport` is present; bail out to failed-screen variant on `VerifyError`
- *(No edits to legacy `src/gui/screens.rs` or `src/gui/components.rs` — deprecated post-PR-#61.)*

**Deliverable:** Each `VerifyReport::Ok` and each `VerifyError` variant renders correctly on Pi Zero LCD (240×240). Multi-line wrap. The verifiable-data drill-down scrolls for long program/transfer lists. In failed state, the Approve button is *removed entirely* (not just defocused) — the user cannot reach a signing path from this surface.

**Verified by:** Visual on real device for each fixture from Phase 7.

**Effort:** 2.5 days.

**Depends on:** Phase 4.

### Phase 6 — Extend known-program registry (display-only, not severity-affecting)
**Scope:** Extend `src/parser/programs.rs` with program-ID → human-name mappings for any program the verify routine encounters. **In the new structural-match model, the registry is used for display only** (showing "Jupiter v6" instead of `JUP6LkbZbjS1jKK...` on the drill-down screen). It does *not* affect verification severity — that's strictly "does the raw match Helius's claims" regardless of whether we recognize the program by name.
**Files:**
- `src/parser/programs.rs` (modify) — append entries; the `programs::identify(...)` API is already there
- `src/ui/screens/verifiable_data_screen.rs` (consume) — call `programs::identify` to render program names; fall back to the truncated pubkey if `None`
- *(Optional)* `src/verify/routine.rs` — cross-check `programs::identify(id).map(|p| p.name)` against `meta.helius_source` and surface a soft warning in the drill-down if they disagree (e.g., registry says "Raydium" but Helius source claims "Jupiter")
**Deliverable:** Verifiable-data screen shows human-readable program names where available; truncated pubkey otherwise.
**Verified by:** Visual: a fixture with a known program shows the name; a fixture with an unrecognized-but-Helius-claimed program shows the pubkey + a "(unrecognized program — verify the address externally)" hint.
**Effort:** ½ day.
**Depends on:** Phase 4.

### Phase 7 — Fixtures and tests
**Scope:** Capture real on-chain fixtures and ship them as test cases.
**Fixtures needed:**
- Clean Jupiter v6 swap (green)
- Squads multisig propose (green)
- Squads multisig execute (green)
- Solana Pay payment with memo (green)
- The actual April 2026 Drift exploit tx — durable nonce + hidden authority change. **Must produce `VerifyError::UnaccountedInstruction`** (verification failed; Approve button removed)
- Synthesized "compromised companion" — meta says SWAP, raw tx has different amounts. **Must produce `VerifyError::NativeTransferMismatch`** or similar (verification failed)
- Tx using ALT extensively. Must produce `VerifyReport::Ok` with `risk_warnings: [AltUsed]` (verified, drill-down shows the warning)
- Tx with a program Helius claims but `programs::identify` doesn't recognize. **Must still produce `VerifyReport::Ok`** — Phase 6's registry is display-only, doesn't affect severity (drill-down shows the program by pubkey with an "unrecognized" hint)
**Files:**
- `testdata/verify/` (new directory) — `*.json` fixtures with raw_tx + expected_meta + expected_report
- `tests/verify_routine.rs` — integration tests
**Deliverable:** A reproducible fixture corpus the team can extend.
**Verified by:** CI runs all fixtures through `verify()` and asserts expected report severity.
**Effort:** 2 days for initial corpus + ongoing.
**Depends on:** Phase 4.

### Phase 8 — Companion UX: pre-send classification preview
**Scope:** Before the user clicks "send to device," extension shows a preview: *"Helius classifies this as: SWAP via Jupiter v6. 0.5 SOL → 12.3 USDC. Send to device for verification?"* This catches Helius failures (offline, rate-limited) gracefully and tells the user what the device is about to verify.
**Files:**
- `extension/entrypoints/sign/sign-app.tsx` (modify) — add classification preview screen
**Deliverable:** Pre-send screen shows Helius classification + raw fallback if Helius is down.
**Verified by:** Manual UX flow.
**Effort:** 1 day.
**Depends on:** Phase 2.

### Phase 9 — Documentation + rollout comms
**Scope:** README updates, verify protocol docs, security model doc, release notes.
**Deliverable:** Clear public docs explaining the threat model, what verify catches, what it doesn't.
**Effort:** 1 day.
**Depends on:** All of the above.

---

## Total estimated effort

| Phase | Effort | Notes |
|---|---|---|
| 0 — decisions | ½ day | |
| 1 — schema | 1 day | |
| 2 — companion bundling | 2 days | |
| 3 — device meta parse | 1 day | |
| 4 — verify routine | **1.5 days** | Reduced — `parser::message::deserialize`, `lookup_tables::expand_accounts`, ATA seeds, and per-program parsers all already exist; integration glue only |
| 5 — device UI | **2.5 days** | Reduced — modular `src/ui/` primitives already in place |
| 6 — allowlist | **½ day** | Reduced — `parser::programs::identify` already exists |
| 7 — fixtures + tests | 2 days | |
| 8 — companion preview | 1 day | |
| 9 — docs | 1 day | |
| **Total** | **~13 days** | Down from 15.5 due to existing primitives |

Realistically, with surprises and review cycles: **2.5–3 weeks** for one engineer, **~1.5 weeks** for two engineers in parallel (extension + firmware tracks naturally split).

---

## Critical path

```
Phase 0 → Phase 1 → ┬─→ Phase 2 → Phase 8
                    │
                    └─→ Phase 3 → Phase 4 ─┬─→ Phase 5
                                           │
                                           ├─→ Phase 6
                                           │
                                           └─→ Phase 7
                                                       
                                                  Phase 9
```

The firmware track (Phases 3–7) is the long pole. The extension track (Phases 2, 8) can ship and sit dormant until firmware catches up — backwards-compat means companions don't break anything by emitting metadata that old firmware ignores.

---

## Risks and unknowns

| Risk | Mitigation |
|---|---|
| **Helius outage / rate limit** | Companion falls back gracefully — sends raw tx without trailer, device shows "no verify metadata, parser-only review." User sees a soft warning that verification was unavailable. |
| **Helius schema drift** (new transaction types we don't display nicely) | `schema_version` field. Unknown `helius_type` values fall through to displaying the `description` string verbatim — structural verification still works regardless because we match on programs/accounts/amounts, not on the type label. |
| **Helius classification is wrong** (rare, but possible) | If Helius's structural claims (programs/accounts/amounts) don't match the raw bytes, verification fails — same as a malicious companion. The user is told to reject and either retry through a clean companion or contact support. We do not let the user "interpret around" a verification failure (per Design principle #1). False-positive risk is acceptable in exchange for the security guarantee. |
| **Pi Zero compute** | Worst-case: 4 ATA derivations + ~8 instruction matches + CBOR decode. Should be <100ms. Benchmark in Phase 4. |
| **QR size bloat** | Animated UR QR already handles multi-frame. Verify metadata adds ~1 frame for typical swaps, ~2–3 for complex Squads multisig. Acceptable. |
| **False positives from incomplete device coverage** | Not applicable in the structural-match model. The verify routine doesn't make "is this program known to me?" judgments. It only checks "is this program in the raw tx as Helius claimed?" Display fallback (truncated pubkey + "unrecognized") covers the gap on the drill-down screen. |
| **Helius API key leakage in extension** | Use a per-user proxy or rate-limited key. Don't bundle a master key in the extension. (Or: accept the risk for the public extension.) |
| **Companion lies in BOTH the meta AND the raw tx** (e.g., crafts a tx that genuinely matches the lie) | Then the lie is on-chain reality — no oracle can save the user. This is "user knowingly approves malicious tx," outside scope. |

---

## Open questions for the team

1. **CBOR or compact JSON for the trailer?** CBOR is smaller (~30% savings) but adds a dep on the firmware side. JSON is human-debuggable. *Recommendation: CBOR, with a `--debug` mode in the SDK that emits JSON for inspection.*

2. **Helius API key strategy.** Bundle a public-but-rate-limited key in the extension? Run a Faraday-hosted proxy? Require users to BYO key? *Recommendation: Faraday-hosted lightweight proxy in v1 — gives us telemetry on what classification gaps exist without burdening users. Migrate to BYO key for paid/institutional tier.*

3. **What happens when companion detects Helius is down?** Block sending the QR? Send raw-only and let device degrade? *Recommendation: send raw-only, let device degrade with a soft warning. Not blocking is the right tradeoff — verification is an enhancement, not a hard gate.*

4. **Do we ship verify support to existing v0 firmware via OTA, or require users to flash a new firmware image?** *Open — depends on existing OTA infrastructure and signing scheme.*

5. **Allowlist update mechanism.** Static-compiled-in vs. fetched-and-verified-at-boot vs. user-configurable. *Recommendation: static-compiled-in for v1, with explicit "this firmware allowlist version: X, last updated YYYY-MM-DD" shown in settings. OTA updates extend it.*

6. **Should the device record verify reports** (anonymous, off-device only via QR back to companion) so we can build a corpus of real-world classification disagreements? *Privacy-vs-product tradeoff. Probably no for v1, defer to v2.*

7. **Multi-tx bundles** (Solana Pay-style or Squads-style packed transactions). How does verify metadata represent multi-tx bundles? *Defer: handle one tx at a time in v1; Phase 7 fixtures should include a Squads multi-step flow as a known limitation.*

8. **Bear-case: Helius shuts down or pivots away from this product.** What's our fallback? *We control the schema; another classifier (Solana FM, Triton, our own) can fill the role. Schema is platform-independent. Lock-in risk is low.*

---

## Success criteria

A v1 ships when ALL of these are true:

- [ ] The actual April 2026 Drift exploit tx, fed through the verify routine, produces a **`VerifyError::UnaccountedInstruction`** — the failure-mode main screen renders, the Approve button is removed, and the drill-down identifies the hidden `SetAuthority` instruction.
- [ ] A clean Jupiter swap produces a **green** report.
- [ ] A synthesized "companion lied about the amount" tx produces a **red** report with the approve button removed.
- [ ] A Squads multisig propose/execute produces a **green** report.
- [ ] Verify routine + ATA derivations on Pi Zero adds <200ms to sign flow.
- [ ] Firmware binary growth from verify code <50KB.
- [ ] All fixtures in `testdata/verify/` pass in CI.
- [ ] Companion gracefully degrades to "verify unavailable" if Helius is down or returns an error.
- [ ] User-facing copy reviewed by someone who didn't write it.
- [ ] Threat model documented in `docs/security/verify-threat-model.md`.

---

## Out of scope for v1

- Real-time fetching of program-ID metadata (verified by hash) instead of static allowlist
- Cross-device verification (e.g., second Faraday confirms first Faraday's parse)
- ML-based anomaly detection on instruction shapes
- Full address-book / counterparty-name resolution ("transfer to *Jupiter Treasury*" vs "transfer to `4xK9...`")
- Verify protocol for sign-message flow (Phase X follow-up — covered in task #11)
