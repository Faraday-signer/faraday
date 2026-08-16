# Failure-mode map — what could fail, how we'd notice, where to look

The forward-looking companion to [`external-surface.md`](./external-surface.md)
(which inventories *who* we trust; this maps *what breaks*). Not a list of
vulnerabilities — a specification of the failure modes this system is
exposed to, each with how it would manifest, the earliest signal we'd get,
what defends it today, and the standing audit hook: what a future review
(human or automated) should re-check. Every entry carries the real-world
precedent its failure class comes from — this system has been broken
before, elsewhere, and the goal is to never rediscover the same lesson
the hard way.

Format per entry:

> **FM-nn · name** — *what could fail.*
> Manifests: how users/tests would experience it. Signal: earliest tripwire.
> Defense today: what stands in the way. Audit hook: what to re-check, when.
> Precedent: the real-world incident this failure class comes from.

Last reviewed: 2026-08-16. Owner: whole team; re-read before any release
that touches `core/crypto`, `core/qr`, `core/parser`, or the build pipeline.

---

## 1. Entropy & seed generation

**FM-01 · Conditioned RNG with a dead entropy source** — *`esp_random`
keeps producing statistically clean output while its true entropy input
(SAR-ADC) silently failed, was never enabled, or was disabled by a
driver conflict.*
Manifests: seeds derived from a deterministic stream; invisible on-device
— the output still passes every statistical test, because it's the
*conditioner* that's fine, not the entropy feeding it. Signal: none from
output statistics alone (that's exactly the trap); only the boot health
checks (PR #119) catch gross failure. Defense today: camera-frame hash
XORed into every seed (seed is never weaker than the stronger source);
boot RCT + monobit + distinct-value gates; `bootloader_random_enable()`
called explicitly at boot with a documented mutual-exclusion note against
future ADC-based battery sensing. Audit hook: any change touching ADC
drivers, `bootloader_random_enable`, or battery sensing must re-read that
note; re-verify the health check still runs before any wallet code.
**Precedent: this is the exact documented failure mode of ESP32's own
hardware RNG** — Espressif states plainly that `esp_random()` is
pseudo-random only when no entropy source (RF or the bootloader ADC
source) is active, and ESP-IDF calls `bootloader_random_disable()` before
the app starts by default (Insomnihack 2026, "Breaking the Random:
Exploiting ESP32's RNG Vulnerabilities in Offline Applications";
docs.espressif.com …/system/random.html). It is also structurally
identical to the **Coldcard 2026 entropy incident** ($38M–$116M drained
across four waves, July–Aug 2026): a guard checked whether the RNG
symbol was *defined*, not whether it was actually *enabled*, so
production boards silently fell through to a deterministic PRNG seeded
from UID + boot timing (engineering.block.xyz write-up;
blog.coinkite.com advisory). Same shape, different chip: hardware RNG
*assumed* live, never actually checked. **Do not confuse this with
CVE-2025-27840** (Tarlogic) — that is undocumented ESP32 Bluetooth debug
commands, unrelated to RNG, and media conflated the two; don't repeat
that conflation in our own docs.

**FM-02 · A mixing source silently dropped** — *a refactor makes one
entropy contributor (camera hash, RNG, or time salt) a no-op while the
flow still "works".*
Manifests: nothing visible — wallets keep generating. Signal: code review
only; this has already happened once in this repo (the camera frame was
silently dropped on ESP32 and later restored — see the comment in
`flows/create.rs`). Defense: the XOR structure localizes the mix in one
function; the mixing chi-square test (PR #129) catches gross bias but NOT
a dropped source (XOR of fewer good sources still looks uniform to a
distribution test). Audit hook: any diff to `CreateCameraEntropy`
handling must show all three sources reaching the XOR; consider a
debug-build assertion that the camera digest is non-zero when a frame
exists. Precedent: general class — see FM-04's Debian citation for the
canonical "an innocuous refactor discards entropy" incident.

**FM-03 · Entropy reuse or truncation across wallets** — *state
carried between creations, or a buffer reused, so two wallets share
entropy bytes.*
Manifests: correlated or identical seeds for different users. Signal: the
cross-wallet decorrelation suite (PR #129, 400 wallets/size) would
collapse; on-device, identical addresses across "different" wallets.
Defense: entropy vec is constructed fresh per flow; suite guards the
pipeline shape. Audit hook: `entropy` buffer lifecycle in
`flows/create.rs` on every change; keep the decorrelation test green in
CI. Precedent: **Randstorm** (BitcoinJS/JSBN `SecureRandom()` falling
back to browser `Math.random()`, 2011–2015 keys, disclosed Nov 2023,
$1B+ in wallets judged potentially affected) — the general lesson that a
"random-looking" source can be silently narrow or state-correlated
across invocations without any single obvious bug (unciphered.com
disclosure).

**FM-04 · Whitening removed** — *someone "simplifies"
`mnemonic_from_entropy` to use raw input, so structure in the input
(camera-image bias, timestamp bytes) reaches the words directly.*
Manifests: subtle bias in produced words. Signal: the round-trip test in
PR #129 asserts the QR equals the *whitened* entropy — a semantics
change breaks it loudly. Defense: SHA-256 whitening documented in the
fn's doc comment; the coin-flip path deliberately bypasses it via a
separately named fn (`mnemonic_from_raw_entropy`) so intent is explicit
in the type signature, not just a comment. Audit hook: the pair of
functions must keep distinct names and docs; any new entropy method must
state which one it calls and why. **Precedent: Debian OpenSSL PRNG**
(CVE-2008-0166) — a maintainer removed two lines flagged by Valgrind as
"uninitialized data," which happened to be the actual entropy-mixing
code; every key generated on Debian/Ubuntu for 18 months drew from a
32,768-value keyspace (debian.org/security/2008/dsa-1571). The lesson
this repo already half-learned once (FM-02): a change made for
completely unrelated reasons — silencing a linter, satisfying a tool,
"cleaning up" — is how entropy dies. Any diff touching `bip39.rs` or the
entropy-mixing path in `flows/create.rs` is security-critical by
definition, not by how big the diff looks.

**FM-05 · Secrets outlive their screen** — *seed/mnemonic copies survive
in RAM after the flow ends, or reach flash.*
Manifests: keys recoverable from a powered or dumped device. Signal:
none at runtime. Defense: `Zeroizing` wrappers on mnemonics and derived
seeds; wallet wipe on power-off; RAM-only design (no filesystem writes
on the seed path); power-off wipes before deep sleep. Audit hook: grep
any new seed-touching code for `Zeroizing`; verify no `std::fs`/NVS
writes enter `core`; confirm the power-off path still wipes before
sleep; note that `Zeroizing` is a best-effort volatile-write, not a
guarantee against compiler dead-store elimination on all optimization
levels — worth an explicit check after any toolchain bump (see FM-15).
Precedent: general RAM-forensics class; the strongest structural defense
here is architectural (no flash writes at all), which sidesteps most of
the published cold-boot/RAM-remanence literature rather than needing to
individually defeat it.

## 2. BIP39 words & key derivation

**FM-06 · Wordlist corruption** — *the build-time-downloaded wordlist is
swapped, reordered, or truncated.*
Manifests: words that other wallets reject, or backups that restore to
different keys elsewhere. Signal: SHA-256 pin in `core/build.rs` fails
the build on tampering; nothing guards against the pin itself being
changed in the same commit. Defense: hash pin; review. Audit hook:
vendor the file in-repo (hardening #1 in external-surface.md) and treat
any diff to it or its pin as security-critical. Precedent: supply-chain
class — see §7 below (FM-19); a wordlist swap is functionally identical
to a dependency substitution attack, just on a data file instead of code.

**FM-07 · Checksum/bit-packing off-by-one** — *the 11-bit word packing
or checksum truncation shifts, so words and entropy disagree.*
Manifests: **backups that restore to a different wallet than the one
holding funds** — the worst silent failure this product can have, because
a *valid-looking* checksum gives false confidence. Signal: PR #129's
exact round-trip (QR bytes == entropy the words encode); official BIP39
vectors (Trezor's canonical set) once added close this further. Defense:
tests; the encode and decode paths are independent implementations.
Audit hook: any diff in `bip39.rs`/`encode_qr.rs` bit-twiddling requires
the vector suite green; add Trezor vectors before the next release.
**Precedent: this exact failure mode is why the Trust Wallet, Profanity,
and Milk Sad incidents were so damaging despite every produced mnemonic
having a perfectly valid BIP39 checksum** — Trust Wallet's browser
extension (CVE-2023-31290) seeded a Mersenne Twister with only 32 bits,
so ~4 billion possible mnemonics existed, every one checksum-valid,
brute-forced with no user interaction; the same shape hit `bx seed`
(Milk Sad, CVE-2023-39910, 227,000+ addresses, $900k+, milksad.info) and
Cake Wallet's Bitcoin path (Dart's non-cryptographic `Random()`, ~549
BTC across 8,757 wallets). **The cross-cutting lesson from all three: a
valid checksum proves format, never entropy** — worth stating explicitly
anywhere this codebase documents what the checksum does and doesn't
guarantee.

**FM-07b · Passphrase normalization mismatch** — *`mnemonic_to_seed`
feeds the user's passphrase straight into PBKDF2 as raw UTF-8 bytes,
with no Unicode NFKD normalization.*
Manifests: a passphrase containing any non-ASCII character (accents,
non-Latin scripts, or certain "look-alike" Unicode forms) derives a
**different seed** on this device than on any spec-compliant wallet
(Trezor, Ledger, most software wallets all NFKD-normalize per spec) —
the user believes they've backed up correctly, and years later a restore
on another wallet silently produces an empty wallet, not an error.
English mnemonic words themselves are ASCII and therefore NFKD-invariant
(unaffected), but this gap is entirely in the user-supplied passphrase
path, which is unconstrained by design. **Currently unmitigated in this
codebase** — confirmed while writing this doc:
`core/src/crypto/bip39.rs::mnemonic_to_seed` calls
`passphrase.as_bytes()` with no normalization step. Signal: none —
this fails silently and only surfaces at restore time, potentially on
a different device entirely. Audit hook: **actionable now** — either (a)
apply Unicode NFKD normalization to the passphrase before PBKDF2 to
match spec, or (b) if intentionally deferred, restrict passphrase input
to ASCII at the UI layer and document the restriction loudly at both
entry and backup-warning screens, so the limitation is a stated design
choice rather than a silent gap. Precedent: BIP39's own spec calls out
the **Japanese wordlist / ideographic-space** trap by name (words must
join with U+3000, not ASCII space, and NFKD does not collapse the two)
— test vectors exist (`bip32JP`) specifically because implementations
routinely skip this and pass every English vector while being broken
for anyone else (github.com/bitcoin/bips bip-0039.mediawiki).

**FM-08 · Derivation divergence** — *SLIP-0010/ed25519 path changes, or
the hardware SHA-512 (mbedtls) diverges from spec, silently producing
different addresses from the same seed.*
Manifests: restored wallets show wrong addresses; funds "vanish" from
the user's view — indistinguishable from FM-07's symptom, different root
cause. Signal: BIP39 seed-derivation known-answer self-test at every
ESP32 boot (fail-closed, cross-checks the hardware mbedtls path against
the pure-Rust `sha2` path); no equivalent vectors yet for the full
seed→address path in CI. Defense: boot self-test; shared core between
targets so Pi and ESP32 can't silently diverge from each other. Audit
hook: add derivation vectors (regression plan, external-surface.md §6);
any `slip0010.rs`/`derivation.rs` change requires them; a derivation-path
change unaccompanied by a discussion of BIP44/49/84/86-style vendor
divergence is a red flag on its own — even without malice, a
default-path mismatch is the single most common wallet support-load bug
industry-wide.

## 3. The backup QR path

**FM-09 · Encode/decode disagreement** — *the QR we draw decodes (by us
or by another wallet) to different bytes than the words encode.*
Manifests: paper backups that fail or mis-restore at recovery time —
possibly years later, with no way to know until it's tested. Signal:
round-trip test (encode side, PR #129) — the full loop through the real
*decoder* is still an open item in the regression plan
(external-surface.md §6). Defense: independent encode (`qrcode` crate)
and decode (`rxing`) implementations. Audit hook: add the encode→decode
loop test; re-run whenever either crate is bumped. **Precedent: this is
the single most-researched failure class in QR-based signing** — the
canonical case is **multi-segment mode-switching**, where a QR bitstream
mixes numeric/alphanumeric/byte/ECI segments and two decoders with
different tolerance for malformed mode transitions reconstruct different
final strings from the *same physical modules*
(zread.ai/nuintun/qrcode encoding-modes writeup); a close cousin is
**charset-sniffing divergence** on byte-mode payloads with no explicit
ECI header (imperialviolet.org/2021/08/26/qrencoding.html). Blockchain
Commons' UR spec was explicitly designed to dodge this entire class by
staying in the alphanumeric/bytewords lane rather than binary mode
(developer.blockchaincommons.com/animated-qrs) — worth confirming our
own encoder never drifts into ambiguous byte-mode territory as a design
constraint, not just a test.

**FM-09b · QR decoder memory-safety bug on attacker-controlled input**
— *the decode path (rxing today; any future decoder) processes a fully
attacker-crafted image, and a malformed QR triggers a buffer overflow
in the decoder rather than a clean parse failure.*
Manifests: crash, or worse, memory corruption from scanning a malicious
QR — the device's camera decode path is, by the nature of an air-gapped
signer, the single most attacker-exposed piece of code on the whole
device (anyone can print a QR and hold it up to the camera). Signal:
none without dedicated fuzzing. Defense: `rxing` is a Rust port, which
structurally rules out the classic C buffer-overflow class (see
precedent), but **logic bugs in the underlying zxing algorithm the port
inherits are not eliminated by the language change** — Rust prevents
memory corruption, not misdecode/parser-confusion. Audit hook: fuzz the
decode entry point (`detect_and_decode`) with malformed/truncated/
oversized images via `cargo fuzz` or similar; treat any `rxing` version
bump as touching attacker-facing code, reviewed accordingly. **Precedent:
ZBar, the most widely deployed C QR decoder, had two CVEs in 2023 from
this exact scenario** — CVE-2023-40889 (heap buffer overflow in
finder-pattern matching) and CVE-2023-40890 (stack buffer overflow in
sequence lookup), both triggered by a crafted QR image, both capable of
info disclosure or RCE (sentinelone.com vulnerability database). quirc,
the small embedded-C decoder more common in constrained/air-gapped
devices specifically, has had comparable dimension-mismatch heap issues
surface downstream (FFmpeg's `vf_quirc` filter). The pattern across all
of them: **QR decoding is parsing untrusted binary input, full stop**,
and should get the same fuzzing budget any other untrusted-input parser
would.

**FM-10 · Hand-transcription mapping drift** — *the block-by-block
walkthrough (7×7/5×5 windows) or the printed template grid shifts
relative to the true matrix.*
Manifests: users faithfully copy a backup that isn't the QR. Signal:
none automated today — the walkthrough and `gen_templates.rs` embed
independent copies of the geometry. Audit hook: any change to
`draw_qr_block`, block math in `app.rs`, or the template generator must
cross-check the other two; a shared-constant refactor would remove the
whole class. **Precedent: SeedSigner's own community documentation treats
this class of thing extremely seriously** — their SeedQR design
philosophy states that CompactSeedQR's compactness is deliberately so it
*stays* an analog, hand-transcribed backup, never a printed or
digitally-stored image, precisely because any indirection between "what
the device shows" and "what the user commits to paper" is where this
kind of drift lives (github.com/SeedSigner/seedsigner seed_qr README).

**FM-10b · Seed QRs are themselves the maximum-sensitivity artifact on
screen** — *a backup QR rendered on-screen is a high-density,
error-corrected, machine-readable rendering of the raw seed — easier for
a hostile camera to capture reliably (at distance, at an angle, in low
light) than reading digits off a screen, precisely because ECC tolerates
a partial or blurry capture.*
Manifests: a single photograph — by a shoulder-surfer, a compromised
nearby device, or a malicious app with camera access on a phone left
nearby — is complete, instant compromise, no further steps needed.
Signal: none; this is a property of the medium, not a bug. Defense: the
backup QR is only ever shown in an explicit, user-initiated export flow
(never ambient); no networked component of this system ever displays or
transmits a seed QR. Audit hook: any new feature must never make backup
QR export easier to trigger accidentally, faster to reach from the main
menu than deliberate, or reachable without the existing warning gate;
"convenience" here is directly a security regression. **Precedent:
SeedSigner explicitly warns users to never scan a SeedQR with a
phone/webcam — it is secret data that must never be digitized a second
time** (seedsigner.com independent-custody guide); shoulder-surfing
studies report 85% of IT professionals have observed unauthorized
viewing of sensitive on-screen data, and QR specifically compresses
"glance at a secret" into "one frame is enough" (rublon.com). Academic
work on power-line side channels (USENIX Security 2021, "Charger-
Surfing") even demonstrates on-screen dynamic content leaking through a
charging cable, which is a good reminder that "don't render the secret
while attached to an untrusted charger" belongs in any future hardening
of the export flow, however unlikely.

## 4. Transaction review & signing

**FM-11 · Display-vs-sign divergence** — *the review renders one
intent, the signature covers another (parser bug, zone extraction bug,
or a tx crafted to exploit parser differentials).*
Manifests: user approves a swap, funds route elsewhere — this is the
single worst possible failure of a "review before you sign" device,
because it defeats the entire reason the device exists. Signal: parser
test corpus; the structural guarantee that review decodes **the same raw
bytes the signature covers** — divergence requires a parser bug, not a
channel bug, which narrows the attack surface considerably. Defense:
that single-input design; outflow-leg summation warnings; high-risk
classifier; explicit-confirm gate (#89). Audit hook: every parser change
runs the tx corpus; fuzz `parse()`/`extract_zoned` periodically; new
instruction types must add corpus entries. **Precedent: the canonical
industry incident is Ledger's LSB-004** (donjon.ledger.com/lsb/004) —
Nano S firmware before 1.5.5 let an attacker inject an **unverified
change output** into an otherwise-legitimate transaction; the device
displayed and confirmed the original send details while actually signing
a transaction that also routed change to an arbitrary address, because
the firmware never verified the change output belonged to the device's
own keys before treating it as safe to hide from the review screen. The
generalizable rule this teaches, worth stating as a standing constraint
for any future review-screen work in this codebase: **every output that
affects where funds go — including anything the code decides not to
show the user, like "obvious" change — must be proven to belong to the
device's own keys before it is ever hidden from review.** Nothing gets a
free pass from the review screen on the assumption that it's routine.

**FM-11b · Blind-signing gap** — *a future transaction type or
instruction the parser doesn't yet decode falls back to showing only a
hash or "data present" instead of the actual decoded intent, and the
user confirms without seeing what they're really approving.*
Manifests: identical symptom to FM-11 but the root cause is coverage
gap, not a bug — the parser correctly declines to lie about content it
can't decode, but a hash-only confirm screen gives the user nothing to
actually verify. Signal: none automated; this needs an explicit policy
decision per new transaction/instruction type added. Audit hook:
maintain a "can we clear-sign this?" checklist for every new instruction
type the parser learns; any code path that would show only a signing
hash instead of decoded fields should be treated as a launch blocker for
that instruction type, not a follow-up. Precedent: this is the exact
failure class implicated in the **2024 Safe{Wallet} incident**
statements around blind-signed manipulated payloads
(x.com/safe/status/1847253904246878553) and is why Ledger shipped
dedicated "Clear Signing" messaging as a product feature
(ledger.com/blog/clear-sign-your-worries-away) — the industry consensus
is that a hash-only confirmation is functionally equivalent to no
confirmation at all, because no human can verify a hash means what the
dapp claims it means.

**FM-12 · ALT display staleness/divergence** — *accounts named from a
baked snapshot of a mutable on-chain table no longer match what the
validator resolves.*
Manifests: device names an account that isn't the one used at execution
(the signature itself is unaffected — the signature covers the table
*reference*, not its resolved contents, so funds cannot be misdirected
by this alone, only mis-displayed). Signal: `fetch_alt.py` freshness
check is manual; #128 showed 3 months of drift in a live table in
practice. Defense: snapshot dates on the consts; the zoned view falls
back to legacy review when a mint can't be resolved deterministically;
sentinels for unknown entries, never fake addresses (FA-24). Audit hook:
CI staleness warning (hardening #5 in external-surface.md); prefer
frozen (immutable-authority) tables going forward; re-snapshot mutable
ones on a schedule. Precedent: internal (#128) — no external incident
needed; this is a self-discovered and self-fixed instance of the general
"snapshot of mutable external state goes stale" class that also shows up
in FM-06 (wordlist) and FM-19 (dependency pinning).

**FM-13 · Signature-envelope confusion** — *the `faraday:sig:` payload
is spliced into the wrong slot or replayed across sessions.*
Manifests: extension submits a tx with a mismatched signature (fails
on-chain — fail-safe by construction, Solana rejects a signature that
doesn't match) or "replays" a signature for an identical re-built tx,
which is not actually a replay attack since Ed25519 is deterministic —
identical bytes always re-sign identically, so this is expected, correct
behavior, not a vulnerability. Signal: on-chain rejection. Defense:
envelope carries version + pubkey; extension matches the signer slot.
Audit hook: envelope version bump discipline on any layout change;
extension-side slot-matching logic on change. Precedent: none directly
applicable found; flagged here mainly to record that the "isn't
determinism a replay risk?" question has an explicit, considered answer
rather than being an unexamined gap.

## 5. Device platform (ESP32-S3 / Pi)

**FM-14 · Radio code linked by accident** — *a dependency or SDK update
pulls Wi-Fi/BT symbols into the ESP32 binary.*
Manifests: the air-gap claim silently becomes false — arguably the
single most brand-critical failure mode in this entire document, since
"no radios" is the headline claim on the landing page. Signal: the CI
`nm` radio-symbol audit fails the build — this is the strongest
automated guarantee in the repo, and deliberately so. Defense: CI gate
on every PR touching hardware. Audit hook: keep the audit's symbol list
in sync with new ESP-IDF versions; verify the job still runs on
`hardware/` path changes if CI is ever restructured (FA-14); note that
Espressif's own undocumented-command research (the CVE-2025-27840
episode, see FM-01) is a reminder that "the radio driver isn't linked"
and "the radio silicon has no other addressable surface" are two
different claims — the audit only proves the first, and the landing
page copy should stay precise about which claim is being made.
Precedent: general "feature creep re-enables transport" class; no
specific external incident, kept as the highest-priority audit hook in
the whole document regardless.

**FM-15 · SDK/toolchain behavior change** — *an ESP-IDF or Xtensa
toolchain upgrade changes RNG conditioning, mbedtls output, or optimizes
away zeroization.*
Manifests: anything from FM-01/FM-05/FM-08 above, without any code diff
on our side to point to — the hardest category to catch, because
`git blame` shows nothing changed in our repository. Signal: boot
self-tests (seed derivation KAT, RNG health) are the canary; zeroization
has no runtime check today. Defense: version pins
(`ESP_IDF_VERSION = v5.3.2`); self-tests. Audit hook: treat every SDK
bump as a security-review event on par with a crypto-code change, not a
routine dependency chore — re-run the full on-device suite, re-read
release notes specifically for RNG/crypto-adjacent changes; consider
adding a volatile-write check that zeroization actually clears memory
under the release optimization level, since compiler dead-store
elimination is a real and separately documented risk class for
`zeroize`-style crates in general. Precedent: the toolchain-as-attack-
surface class is covered in depth under FM-18 (build infrastructure);
this entry is the narrower "no attacker required, just an upstream
behavior change" version of the same risk.

**FM-16 · Physical-access attacks** — *fault injection/voltage glitching
to skip the review gate or extract RAM; flash readout on a seized
device.*
Manifests: targeted, physical-presence attacks against a specific known
device and owner. Signal: none in software. Defense: RAM-only keys
(power-off = gone) is a genuinely strong mitigation here — there is no
flash to read out, which sidesteps most of the published cold-boot/
flash-extraction literature entirely rather than needing to defeat it.
Audit hook: keep the no-flash-writes property testable (shares FM-05's
hook); document plainly that glitch/fault resistance against a
determined physical attacker is out of scope for the current threat
model — stating that honestly is better than implying a protection that
isn't there. Audit hook (secondary): the ed25519 fault-attack literature
below is relevant if this threat model is ever revisited. **Precedent:
academic fault-attack research against deterministic EdDSA signing**
(Poddebniak et al., eprint.iacr.org/2017/1014; Romailler & Pelissier,
FDTC 2017) demonstrates that a device performing two deterministic
signatures of the same message under a glitch-induced fault can leak
its private key by comparing the two signatures — relevant background
for this device's ed25519 signing path specifically, and worth noting
that the standard defense (RFC 6979-style deterministic nonces, which
this device already benefits from via ed25519-dalek's design) creates
exactly this fault-attack surface as a tradeoff against the RNG-bias
risk that determinism was chosen to avoid in the first place. Neither
choice is free; both are documented here so the tradeoff is visible
rather than assumed.

## 6. Host side (extension / dapp / npm)

**FM-17 · Malicious npm dependency in the extension** — *a compromised
package alters the tx it relays or phishes in the extension UI.*
Manifests: dapp UI promises X, QR carries Y. Signal: **the device
review is the tripwire by design** — amounts/programs/fees on-screen
come from the signed bytes, not from anything the extension merely
claims. Defense: host-untrusted architecture; small direct-dep list;
lockfile. Audit hook: `pnpm audit` in CI (hardening #3 in
external-surface.md); treat any new direct dependency in `extension/`
as review-worthy; never let device display trust host-supplied data
unlabeled. **Precedent: crypto wallets are the single most targeted
vertical in the npm supply-chain-attack landscape from 2018 to today,
and several incidents targeted exactly this device's threat model.**
`event-stream`/`flatmap-stream` (2018) is the founding case: a
maintainer handoff via social engineering led to a payload
*target-keyed to activate only inside the Copay Bitcoin wallet's build*,
harvesting seed phrases and private keys from wallets holding over 100
BTC (blog.npmjs.org incident writeup) — proof that "a dependency
several levels removed can be purpose-built to steal exactly what this
device is designed to protect." More recently: the **Ledger connect-kit
compromise** (Dec 2023, ~$600k stolen) came from a former employee's
npm publish rights that were never revoked after offboarding — a process
failure, not a code failure (ledger.com/blog/security-incident-report);
**@solana/web3.js** (Dec 2024, CVE-2024-54134) shipped a backdoor that
exfiltrated private keys passing through its signing code paths after a
phished maintainer account, directly relevant since this is the same
ecosystem the extension is built on; and the **Shai-Hulud npm worm**
(Sept–Nov 2025, 796 packages, 20M+ weekly downloads at peak) demonstrates
that install-time compromise can now self-propagate by stealing and
reusing the *victim's own* publish credentials — no attacker C2 needed
once it's running on a developer machine. None of these could have
stolen a Faraday seed (the device never exposes it to the host), which
is worth stating plainly as the architectural payoff of the air-gap
design — but they could each have shown a user a manipulated
transaction, which is exactly what the on-device review exists to catch.

## 7. Build & release infrastructure

**FM-18 · CI action hijack** — *a mutable action tag is repointed at
malicious code with repo-token access.*
Manifests: poisoned artifacts, neutered audits, leaked tokens. Signal:
none today without dedicated tooling. Defense: none beyond GitHub's own
account-security controls. Audit hook: pin every action by full 40-char
commit SHA, not by tag (hardening #2 in external-surface.md); minimal
`GITHUB_TOKEN` permissions per workflow, no long-lived secrets sitting
in CI for something to harvest. **Precedent: tj-actions/changed-files**
(CVE-2025-30066, March 2025) is the textbook case for exactly this
document's warning — the attacker first compromised a *different* action
(`reviewdog/action-setup`, itself via a script-injection bug), stole a
bot's PAT, then used it to **retag nearly every version tag** (v1
through v45.0.7) of a widely-used action to a malicious commit; the
payload dumped CI runner process memory looking for secrets and printed
them into build logs (cisa.gov advisory; wiz.io technical writeup). The
one-sentence lesson the CVE number exists to teach: **`uses: action@v4`
is not a pin. `v4` is a movable label an attacker can repoint. Only a
commit SHA is immutable.** Every action in `.github/workflows/*.yml`
should be re-audited against this specific fact.

**FM-19 · Dependency substitution at build** — *a crate version is
republished/typosquatted, or the registry serves altered content.*
Manifests: FM-class of whatever the affected crate touches — could be
anything from a build-time credential harvester (Rust precedent below)
to a runtime backdoor. Signal: `Cargo.lock` checksums catch content
changes for pinned versions already in the lockfile; nothing reviews
*new* versions on a routine bump today. Defense: lockfile; small
direct-dep surface. Audit hook: `cargo-deny` advisories + source-pinning
checks in CI (hardening #3); a dependency-bump PR should get the same
review depth as a code-change PR, not less, since `build.rs` and proc-
macros both grant arbitrary code execution at `cargo build` time.
**Precedent: `faster_log`/`async_println` on crates.io** (live May–Sept
2025, ~4 months, 8,424 downloads) is the most directly on-point incident
in all this research — a typosquat of the popular `fast_log` logging
crate that worked as an actual functioning logger (real cover, not an
obvious fake), while at build/run time it regex-scanned project source
files specifically for **Ethereum private keys and Solana base58 keys**
and exfiltrated any matches to a hardcoded C2 (blog.rust-lang.org
advisory; socket.dev technical writeup). This is not a hypothetical
concern for "some other project" — it is a crate that existed
specifically to steal Solana keys from Rust codebases, live in the
crates.io registry for four months, undetected until external
researchers found it. `rustdecimal` (2022, CrateDepression) is the
earlier sibling case, targeting CI environment variables via a
typosquat of `rust_decimal`. The standing defense against this whole
class for a small team, in order of effort: `cargo-deny` (automates the
advisory check), then `cargo-vet` (imports Mozilla/Google's existing
audit trails so only the *delta* needs human review), then vendoring
dependencies for release builds entirely (`cargo vendor`, which converts
most registry-window attacks into non-events for anything already
vendored — the strongest single control, and arguably the most in
keeping with this project's own air-gap philosophy applied to the build
step).

**FM-20 · Unreproducible release** — *the published firmware image
can't be independently rebuilt bit-for-bit, so "verify the hash"
verifies nothing but our own CI's word for it.*
Manifests: the landing page's verification claim is hollow in the
specific case that matters most — a compromised CI could ship
signed-looking artifacts and nothing external would ever catch it, no
matter how clean the source repository looks. Signal: none until someone
actually attempts an independent rebuild and it either matches or
doesn't. Defense: none yet — this is squarely FA-17 territory. Audit
hook: make bit-for-bit reproducibility a release-blocking check once
FA-17 lands; publish the exact rebuild recipe next to every release, not
buried in docs. **Precedent: two incidents define why this matters.**
**xz-utils** (CVE-2024-3094, 2024) shows that even fully open-source
projects with public commit history can ship a backdoor that lives *only*
in the release tarball's build script (`build-to-host.m4`), not in the
git repository anyone would think to review — a maintainer earned over
2.5 years of social engineering, then modified the release artifact,
not the source; caught only because a Microsoft engineer happened to
notice SSH logins taking 500ms longer than expected
(openwall.com/lists/oss-security disclosure). **SolarWinds/SUNBURST**
(2020) is the more severe version of the same lesson at a much larger
scale: attackers compromised the *build environment* itself, so the
in-memory-only implant swapped a source file during compilation and the
resulting binary was **legitimately code-signed** — signing proved where
the binary came from, not that the build process producing it was
clean (mandiant.com technical analysis). The through-line for this
project: **code signing and even a clean git history both answer "did
this come from us," never "was the build itself untampered." Only an
independent party rebuilding from source and getting byte-identical
output answers that second question**, which is precisely what
reproducible builds exist to let anyone do without having to trust this
team's CI at all.

---

## Standing audit cadence (the "look here in the future" list)

- **Every PR touching `core/crypto` / `core/qr` / `core/parser`:** the
  regression suites (#129 + planned vectors) are the gate; re-read the
  relevant FM entries above — especially FM-04, FM-07, and FM-11, the
  three where a "small" diff has historically been the whole bug.
- **Every dependency bump (Rust or npm):** which FM entries does the
  crate appear in? Treat it as a security review, not a version-number
  chore — this is the single most under-invested control relative to
  its historical payoff (FM-19).
- **Every SDK/toolchain bump:** security-review event (FM-15), full
  on-device suite re-run, release notes read specifically for
  RNG/crypto-adjacent changes.
- **Every release:** boot self-tests verified on hardware; radio audit
  green; ALT snapshot age checked; (once FA-17 lands) reproducible
  rebuild verified independently, not just by the same CI that built
  the artifact being checked.
- **Quarterly:** re-run `fetch_alt.py` against baked tables;
  `cargo-deny` / `pnpm audit` reports triaged; re-read this document and
  update "Last reviewed" — a failure-mode map that nobody re-reads decays
  into exactly the false confidence FM-07 warns about.
