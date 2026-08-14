# 2026-07-29 — FA-16 in review: landing page redesign (faraday.to)

A full redesign of the `site/` landing page — a real product site for the
air-gapped Solana signer, not a care pass. Branch `feat/landing-redesign`,
PR #117 (**draft**).

**Awaiting the one gating item:** design direction must be approved by cxalem
(Alejandro) on the Vercel preview **before merge** (card FA-16, acceptance
criterion). The draft PR's preview deployment is that approval artifact — do
not merge until it is signed off.

## Design decision (needs cxalem's eyes)
- Moved from the previous **light** one-pager to a **dark** "instrument panel"
  theme: cyan `#1AF8FF` on deep cool slate, Departure Mono display type, hairline
  borders, QR/finder-pattern and measurement-grid motifs. This matches the brand
  direction in the card brief (cyan-on-dark, technical/precision aesthetic) and
  is the human design call the plan reserves for cxalem. If the light direction
  is preferred, that is a preview-time decision, not a silent one.

## Landing page (`site/app/page.tsx`, new components in `site/components/landing/`)
Single scrolling page, sections numbered like a datasheet:
- **Hero** — headline, air-gap value prop, primary CTA (early access) + GitHub,
  and a signature `AirGapVisual` (browser ⇄ QR ⇄ offline signer).
- **How it works** — the one round trip across the air gap (build → cross →
  review on-device → sign & return).
- **Security model** — no antennas / keys in RAM only / you sign what you see /
  open all the way down. Copy names the specific property, never bare "secure";
  the clear-signing card states unknown transactions are flagged, never guessed.
- **The device** — Pi Zero 1.3 reference build (~$35, no radio chip); ESP32-S3
  build noted as in progress; desktop simulator mentioned.
- **The suite** — signer / extension (heading to the Chrome Web Store) / mobile.
- **Demo** — framed placeholder for the 90-second Ika-approver clip.
- **Flash your device** — teaser for the browser flasher (FA-13), links to
  `/flash`.
- **Early access** — the existing waitlist form, unchanged logic.

## Reserved slots (so FA-13 and FA-08 need no second redesign)
- **Flasher (FA-13):** nav link "Flash device · soon", a landing teaser, and a
  new `/flash` stub page (`site/app/flash/page.tsx`) that states ESP32-S3 /
  WebSerial target and that Pi Zero uses the SD-card image. FA-13 replaces the
  placeholder block with the real ESP Web Tools flow.
- **Extension store (FA-08):** suite card + footer carry a "Chrome Web Store ·
  soon" slot; drop the listing URL in when it goes live.

## Preserved / not regressed
- Email capture untouched (`waitlist-form.tsx` server action + Supabase path);
  only the error text color was adjusted for dark-theme legibility, and two
  pre-existing unescaped-apostrophe lint errors in that file were fixed.
- `/privacy` (PR #115) still linked in the footer; it is fully token-driven, so
  it re-themes to dark automatically and still serves HTTP 200.

## OpenGraph / meta
- Set `metadataBase = https://faraday.to` in `layout.tsx` — clears the build
  warning and makes OG/Twitter image URLs absolute.
- `opengraph-image.tsx` reskinned to the dark theme; still renders (HTTP 200,
  `image/png`).

## Verification
- `pnpm typecheck` clean, `pnpm build` green (routes `/`, `/flash`, `/privacy`,
  `/opengraph-image` all prerender), `pnpm lint` clean.
- Production server smoke test: every section string present on `/`, `/flash`
  stub renders with the ESP32-S3 / WebSerial / SD-card copy, `/privacy` 200, OG
  image 200 `image/png`.
- Not verified locally: the live Vercel preview render and cross-device/visual
  polish — that is exactly what cxalem reviews on the PR preview.

Site uses **pnpm** (not npm). `site/package.json` gains a `typecheck` script.
