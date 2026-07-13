# Faraday Verify Service Proposal

**Status:** draft for team review
**Scope:** verifier service, extension flow, and device flow for clear pre-sign transaction review.
**Related:** `docs/proposals/verify-protocol.md`, `docs/proposals/qr-uri-envelope.md`

## Summary

Faraday should not rely on companion-provided transaction descriptions. The companion is inside the threat model, so any text or metadata it sends can be forged.

The safer architecture is a signed verification report:

1. The extension receives an unsigned transaction from a dapp.
2. The extension sends the exact unsigned transaction to a Faraday verifier service.
3. The verifier service parses, resolves, simulates, classifies, and risk-scores the transaction.
4. The verifier service signs a canonical report over the exact transaction message hash.
5. The extension QR-bundles the raw transaction plus signed report.
6. The offline device verifies the report signature and hash binding.
7. The device renders a clear, human-readable review from structured signed fields.
8. The device signs only the raw Solana transaction message, never the report or envelope.

The verifier service may use Helius, other providers, and our own parsers. Helius is a useful enrichment and comparison source, not the root of trust. Faraday's value is the signed, pre-sign, offline-verifiable report and the policy engine behind it.

## Goals

- Show the clearest possible transaction summary on the device.
- Keep the device air-gapped.
- Protect against a compromised companion lying about what the user is signing.
- Support unsigned pre-sign transactions, including CPIs and swap effects that the device cannot derive offline.
- Make the verifier service good enough to become standalone infrastructure for other wallets or signing devices.
- Keep the schema ready for durable nonces even if durable nonce signing is not implemented yet.

## Non-goals

- The device does not simulate transactions.
- The device does not fetch RPC data.
- The device does not blindly display Helius or companion free text.
- V1 does not need to perfectly classify every Solana transaction. It must fail safely and explain uncertainty.
- V1 does not need public publication of every report.

## Core Trust Model

The device trusts:

- Its own firmware.
- Its own transaction hash calculation.
- A small set of embedded verifier public keys.
- A signed report only when its signature is valid and it is bound to the exact transaction message hash being signed.

The device does not trust:

- The extension.
- The dapp.
- Unsigned metadata.
- Arbitrary display strings.
- A report whose hash does not match the raw transaction.

The verifier service is trusted to classify honestly. This is a product and operational trust boundary, so it needs audit logs, regression tests, provider comparison, and eventually third-party verifiers.

## Why Not Just Use Helius Directly?

Helius is useful, but it is not enough by itself.

Problems with using Helius as the direct device display source:

- Helius responses are not signed for offline device verification.
- A compromised companion can forge a Helius-shaped payload.
- Some Helius enhanced transaction APIs are built around finalized transaction signatures, while Faraday needs pre-sign unsigned transaction review.
- Helius output is general-purpose. Faraday needs signer-specific risk policy, offline display constraints, and fail-closed semantics.
- Helius can change schemas or coverage without Faraday controlling the user experience.

How Helius should help:

- Use Helius RPC for simulation and account/token metadata where appropriate.
- Use Helius DAS/token metadata for token names, symbols, decimals, and NFT metadata.
- Use Helius enhanced classifications as an enrichment source when available.
- Compare Helius output against our own classifiers in CI and production canaries.
- Use Helius disagreements as training data for improving Faraday classifiers.

Faraday's own classifiers add value because they define the security policy and display contract. Helius can be better at broad ecosystem coverage, but Faraday must own the final report format, risk flags, and signed attestation.

## Dapp Input Variability

Different dapps hand wallets different shapes of data. Jupiter, Squads, Solana Pay, NFT marketplaces, and custom dapps may differ in request envelope, transaction construction path, simulation assumptions, and optional context.

The verifier plan handles this by making the canonical input the signed payload's Solana transaction bytes, not the dapp-specific request shape.

Rules:

- The extension may accept many wallet-adapter or dapp-specific request shapes, but it must normalize them into exact `raw_tx` bytes before verification.
- The verifier service classifies from the normalized transaction, resolved accounts, simulation, and provider enrichment, not from dapp-provided labels.
- Dapp origin and dapp-provided context are advisory fields only. They can help explain the source, but they cannot override parsed facts.
- If a dapp supplies useful intent metadata, the service may store it under `untrusted_context` and compare it against verified facts. A mismatch becomes a warning or blocker.
- Provider-specific adapters belong in the verifier service ingestion/enrichment layer, not on the device.
- The device sees one stable envelope and one stable report schema regardless of which dapp produced the transaction.

This means we do not need a bespoke device classifier for every dapp. We need robust normalization plus classifier rules over common Solana facts: balance deltas, program calls, CPIs, approvals, authority changes, account creation, and signer requirements.

## Classification Strategy

Do not classify directly from one source. Classify from a normalized fact model.

Pipeline:

1. **Raw parse facts:** signers, fee payer, outer instructions, program IDs, account keys, blockhash, durable nonce shape, ALT usage, token approvals, authority changes, program upgrade instructions.
2. **Account resolution facts:** ALT entries, token accounts, mint owners, decimals, metadata, known program labels.
3. **Simulation facts:** success/failure, logs, inner instructions, SOL balance deltas, token balance deltas, account creation/closure, CPI effects.
4. **Provider enrichment facts:** Helius classification, Helius metadata, other indexer outputs, token reputation, address labels.
5. **Classifier outputs:** action summary, risk flags, confidence, display model.

The output should separate three concepts:

```rust
struct VerificationReport {
    action: ActionSummary,
    risk: RiskAssessment,
    facts: VerifiedFacts,
    display: DisplayModel,
}
```

`ActionSummary` answers: what is the user probably doing?

Examples:

- `Swap`
- `Transfer`
- `Stake`
- `Unstake`
- `CreateTokenAccount`
- `ApproveDelegate`
- `SetAuthority`
- `MultisigAction`
- `Unknown`

`RiskAssessment` answers: should the user sign this?

Examples:

- `Safe`
- `Caution`
- `Danger`
- `Blocked`

`VerifiedFacts` answers: what concrete facts were observed?

Examples:

- Signer set.
- Fee payer.
- Programs invoked.
- Net token deltas.
- Native SOL delta.
- Authority changes.
- Approvals.
- Durable nonce usage.
- ALT resolution status.
- Simulation status.

`DisplayModel` answers: how should the device show this within 240x240 constraints?

The display model must be generated from structured fields. Avoid arbitrary prose from providers. If text is signed, it should be deterministic text generated by Faraday's renderer from enums and fields.

## Verification Report Schema

The QR payload should be an explicit typed envelope, not a trailer delimiter.

```rust
struct VerifiedUnsignedTxEnvelope {
    schema_version: u16,
    raw_tx: Vec<u8>,
    report: SignedVerificationReport,
}

struct SignedVerificationReport {
    payload: VerificationReportPayload,
    verifier_id: VerifierId,
    signature: [u8; 64],
}

struct VerificationReportPayload {
    schema_version: u16,
    network: Network,
    raw_tx_sha256: [u8; 32],
    message_sha256: [u8; 32],
    issued_at_unix: u64,
    expires_at_unix: Option<u64>,
    verifier_policy_version: u32,
    tx_lifetime: TxLifetime,
    resolved_accounts: AccountResolutionSummary,
    simulation: SimulationSummary,
    report: VerificationReport,
}
```

`message_sha256` is the critical binding. Solana signs the transaction message, so the device must recompute this hash from `raw_tx` and verify it matches the signed report.

`raw_tx_sha256` is also useful for debugging, storage, and extension/service consistency.

### Durable Nonce Readiness

Even before durable nonce signing is implemented, the schema should reserve a first-class lifetime field:

```rust
enum TxLifetime {
    RecentBlockhash {
        blockhash: [u8; 32],
    },
    DurableNonce {
        nonce_account: Pubkey,
        nonce_authority: Pubkey,
        nonce_value: [u8; 32],
        advance_nonce_instruction_index: u16,
    },
    Unknown,
}
```

V1 policy can be conservative:

- If durable nonce is detected and unsupported: `Blocked` or `Danger`.
- If no durable nonce is detected: normal flow.
- If the shape is ambiguous: `Blocked`.

This keeps future support additive without redesigning the report format.

### ALT Readiness

Address Lookup Tables need explicit handling because the offline device cannot fetch ALT contents.

```rust
struct AccountResolutionSummary {
    uses_alt: bool,
    lookup_tables: Vec<LookupTableReport>,
    all_instruction_accounts_resolved: bool,
}

struct LookupTableReport {
    table_address: Pubkey,
    resolved_at_slot: u64,
    writable_addresses: Vec<Pubkey>,
    readonly_addresses: Vec<Pubkey>,
    status: LookupTableStatus,
}
```

V1 policy options:

- Best security: block if ALT accounts cannot be fully resolved by the verifier.
- Practical option: allow if the signed report includes all resolved account keys and the device confirms instruction indices are in range, but show an ALT warning.
- Do not silently accept unresolved ALTs.

## Verifier Service Flow

The verifier service is the source of signed reports.

Input:

- `raw_tx` bytes.
- Expected signer pubkey.
- Network/cluster.
- Optional dapp origin.
- Optional user-visible context from the extension.

Steps:

1. Decode and validate the transaction envelope.
2. Extract message bytes and compute `message_sha256`.
3. Parse static account keys, signers, fee payer, blockhash, outer instructions, and version.
4. Resolve ALTs through RPC if present.
5. Detect transaction lifetime: recent blockhash, durable nonce, or unknown.
6. Run simulation with signature verification disabled for normal unsigned transactions.
7. For durable nonce transactions, do not use simulation settings that replace the blockhash or change nonce semantics.
8. Collect balance deltas, token deltas, inner instructions, logs, and simulation errors.
9. Enrich accounts, tokens, programs, and labels using providers such as Helius.
10. Run classifiers over normalized facts.
11. Run policy rules over classifier outputs and risk facts.
12. Build a canonical display model from structured fields.
13. Sign the canonical report payload.
14. Return `SignedVerificationReport` to the extension.

Failure modes:

- Cannot parse tx: return `Blocked` report if possible, otherwise return service error.
- Cannot resolve ALT: return `Blocked` or `Danger` report, depending on policy.
- Simulation failed: return a signed report with `simulation.status = Failed`; policy decides severity.
- Provider unavailable: continue with degraded facts if safe, but mark provider coverage missing.
- Classification uncertain: use `ActionSummary::Unknown` and explain the uncertainty.

## Extension Flow

The extension is the online coordinator. It is not trusted by the device.

Steps:

1. Receive unsigned transaction from dapp or Faraday wallet send flow.
2. Validate basic shape locally: base64, signer slots, expected signer included.
3. Submit exact raw transaction bytes to the verifier service.
4. Receive signed verification report.
5. Verify the service signature locally as a UX/debug check. The device will verify again.
6. Confirm report `message_sha256` matches the local unsigned transaction message.
7. Show a pre-send preview based only on the signed report.
8. Bundle `raw_tx + signed_report` in a typed QR envelope, likely `faraday:unsigned:v1.<base64url-cbor>`.
9. Encode as animated UR when payload exceeds static QR limits.
10. After device signs, scan the `faraday:sig:` response.
11. Verify the returned signature against the original unsigned transaction message.
12. Splice signature and submit/broadcast.

Extension must not:

- Rewrite report text.
- Change raw transaction bytes after verification.
- Send raw-only fallback silently.
- Treat an unsigned provider response as verified.

Open product decision:

- If verifier service is unavailable, should the extension block signing or allow raw-parser-only mode?

Recommendation for V1:

- Default to blocking for dapp-originated transactions.
- Allow a clearly labeled developer override in simulator/dev builds only.

## Device Flow

The device is the offline verifier and signer.

Steps:

1. Scan typed QR/UR envelope.
2. Decode `VerifiedUnsignedTxEnvelope`.
3. Reject unknown schema versions unless explicitly supported.
4. Recompute `raw_tx_sha256` from `raw_tx`.
5. Extract Solana message bytes from `raw_tx`.
6. Recompute `message_sha256`.
7. Verify report signature using embedded verifier public key.
8. Confirm signed `raw_tx_sha256` and `message_sha256` match local computation.
9. Parse minimal raw transaction facts locally: signers, fee payer, version, outer program IDs, instruction count, ALT usage, lifetime shape.
10. Sanity-check those raw facts against the signed report.
11. Confirm loaded wallet pubkey is a required signer.
12. Render report display model.
13. If risk is `Blocked`, remove approve path entirely.
14. If risk is `Danger`, require explicit reject-by-default flow or block, depending on policy.
15. If approved, sign only the raw transaction message bytes.
16. Return signature-only QR as today.

Device must not:

- Simulate.
- Fetch RPC.
- Trust unsigned display strings.
- Sign the full QR envelope.
- Sign raw tx bytes if the signed report binds to a different message hash.

## Device Display Model

The current device review model should be preserved. This proposal is not a redesign of the first screen.

Compatibility requirements:

- Keep the current first-screen pattern: clear general transaction info, existing hero/action layout, and existing approve/reject flow.
- The verifier report feeds the same conceptual slots the current parser already uses: action kind, spend line, receive line, fee, warning state, and details pages.
- Transaction-specific pages may change when the report provides better facts, especially for CPIs, net balance changes, approvals, authority changes, ALTs, and durable nonce status.
- The first screen should not become a raw instruction dump or a provider-branded report page.
- The report should improve confidence and accuracy behind the current UX, not force users into a new mental model.

The main screen should be derived from signed structured fields, but rendered through Faraday's existing display grammar.

Examples:

Swap:

```text
SIGN TRANSACTION
Verified by Faraday

SWAP
-0.50 SOL
+123.40 USDC

Risk: Low
Fee: 0.000005 SOL
```

Dangerous approval:

```text
DO NOT SIGN
Verified by Faraday

TOKEN APPROVAL
Delegate can spend USDC
Amount: Unlimited

Risk: Critical
```

Unknown transaction:

```text
CAUTION
Verified by Faraday

UNKNOWN ACTION
3 programs invoked
Simulation succeeded

Review details
```

The drill-down should show verified facts:

- Message hash.
- Fee payer.
- Signers.
- Programs.
- Net balance changes.
- Authority changes.
- Approvals.
- ALT usage.
- Durable nonce status.
- Provider coverage.
- Classifier confidence.

## Public and Immutable Reports

Reports should be immutable, but not public by default.

Recommendation:

- The signed report is immutable because it is signed over a canonical payload hash.
- The report should be private by default because it can leak user intent, wallet addresses, token holdings, dapp usage, and future trades.
- The schema should be public.
- The verifier public keys should be public.
- Policy versions and classifier versions should be public.
- A transparency log can be added later, but it should default to redacted entries such as report hash, policy version, timestamp bucket, and verifier key id.
- Users or integrators can opt in to publishing full reports for debugging, audits, or incident response.

This gives immutability and auditability without forcing every signing intent onto a public ledger.

## Verifier Service as Infrastructure

This can become a product.

Possible external API:

```http
POST /v1/verify/solana/unsigned-transaction
Content-Type: application/cbor or application/json

{
  "network": "mainnet-beta",
  "raw_tx_base64": "...",
  "expected_signer": "...",
  "origin": "https://example-dapp.com"
}
```

Response:

```json
{
  "schema_version": 1,
  "report_payload": "base64url-cbor",
  "verifier_id": "faraday-mainnet-v1",
  "signature": "base64url-ed25519"
}
```

Why other projects may want this:

- Pre-sign transaction simulation and classification is hard.
- Wallets need better human-readable transaction previews.
- Hardware wallets need offline-verifiable reports.
- Teams need a stable risk policy layer above raw RPC.
- Provider disagreement and classifier regression testing are valuable infrastructure.

Commercial shape:

- Free tier for Faraday users.
- API tier for wallets.
- Enterprise tier with custom policies, allowlists, private deployment, and audit logs.
- Optional self-hosted verifier for institutions.

## Classifier Quality Plan

We should assume Helius is better than us in some areas today. The solution is not to avoid our classifiers. The solution is to measure, compare, and improve them.

Quality loop:

1. Build a fixture corpus: swaps, transfers, approvals, authority changes, stake, multisig, NFT, compressed NFT, failed simulation, ALT-heavy txs, durable nonce txs.
2. Store raw tx, simulation output, Helius output, expected Faraday facts, expected display model, and expected risk level.
3. Run all fixtures in CI.
4. Diff Faraday classification against Helius and other providers.
5. Treat disagreement as a review item, not automatically as a Faraday bug.
6. Add real-world anonymized telemetry only if users opt in.
7. Version every classifier and policy rule.
8. Keep a regression dashboard: coverage, unknown rate, false danger rate, missed danger rate, provider disagreement rate.

Success metrics:

- Low unknown rate for common dapp transactions.
- Near-zero missed critical approvals/authority changes.
- Clear explanations when classification is uncertain.
- Stable display for the same tx across releases unless policy intentionally changes.
- Better risk policy than Helius for signer-specific threats.

## Implementation Phases

### Phase 0: Decisions

- Choose fail-closed vs fallback when verifier unavailable.
- Choose canonical encoding: CBOR recommended.
- Choose signing key strategy and rotation plan.
- Choose first policy for ALTs.
- Choose first policy for durable nonce detection.
- Choose whether third-party verifier keys are supported in V1.

### Phase 1: Shared Schema

- Rust and TypeScript types for `VerifiedUnsignedTxEnvelope` and `SignedVerificationReport`.
- Canonical encoding.
- Hash binding helpers.
- Signature verification test vectors.

### Phase 2: Verifier Service MVP

- Parse unsigned tx.
- Resolve ALTs.
- Simulate unsigned tx.
- Extract balance deltas and high-risk facts.
- Produce signed report.
- Basic classifiers: transfer, swap, approval, set authority, create account, unknown.

### Phase 3: Extension Integration

- Call verifier service before showing QR.
- Verify returned signature locally.
- Bundle raw tx plus signed report in typed QR envelope.
- Show signed-report preview.
- Remove silent raw-only fallback.

### Phase 4: Device Integration

- Decode typed verify envelope.
- Verify service signature.
- Verify tx hash/message hash binding.
- Sanity-parse raw tx.
- Render verified report states.
- Block signing on invalid report or blocked risk.

### Phase 5: Classifier Quality Corpus

- Fixture corpus.
- Helius comparison harness.
- Regression dashboard.
- Policy versioning.

### Phase 6: External Infrastructure Product

- Public API docs.
- Tenant keys and quotas.
- Optional self-hosting path.
- Redacted transparency log design.
- SLA and incident response process.

## Open Questions

1. Do we block signing when the verifier service is down?
2. Which verifier public keys ship in firmware V1?
3. Do we allow third-party verifier keys, or only Faraday keys?
4. What is the first ALT policy: block unresolved, warn, or require full signed account resolution?
5. What is the first durable nonce policy: block all, danger warning, or hidden feature flag?
6. How much provider output do we retain in signed reports for debugging?
7. Do we publish redacted report hashes in a transparency log?
8. What telemetry, if any, is acceptable for classifier improvement?

## Recommended V1 Position

- Build the verifier service.
- Use Helius heavily, but behind Faraday's verifier.
- Own the final classifier, risk policy, and display model.
- Sign every report.
- Device verifies signatures and hashes offline.
- Block unsigned or unverifiable reports in production.
- Keep reports private by default.
- Make schemas, verifier keys, and policy versions public.
- Treat durable nonce and unresolved ALT as explicit risk classes from day one, even if support is initially blocked.
