# Landing page — pending work

Status ledger for everything deliberately hidden or unfinished on the
landing redesign (branch `feat/landing-redesign`, PR #117). When an item
ships, re-enable it and delete its entry here.

Last updated: 2026-08-08

## Hidden sections (components kept in the tree, not rendered)

Removed from `app/page.tsx`; restore by re-adding the import + element.

- **SuiteSection** (`components/landing/suite-section.tsx`)
  Blocked on real destinations: the extension has no store listing yet
  (FA-08) and the mobile app has no public link. Re-enable when at least
  the extension card can link somewhere real.
- **DemoSection** (`components/landing/demo-section.tsx`)
  Renders a placeholder frame; there is no demo video. Record the
  90-second Ika approver run, then re-enable.
- **FlashTeaser** (`components/landing/flash-teaser.tsx`)
  Teases the web flasher, which does not exist yet (FA-13). The `/flash`
  page is still routable but unlinked. Re-enable teaser + nav/footer
  links when the flasher ships. The copy should also cover the Pi
  (SD-card image), not only the ESP32 WebSerial path.

## Pruned navigation/footer entries

- Nav: "The suite" and "Flash device · soon" removed with their sections.
- Footer: "Flash device" link and "Extension · soon" placeholder removed.
  Both come back with their targets.

## Wired and live (for contrast)

Hero (3D showcase + look-inside), The life of a key, How it works
(round-trip player), The device (specs/tabs), Waitlist (Supabase server
action), footer links to GitHub / privacy / X.

## Known follow-ups on the shipped sections

- **ESP32 web-flasher claim** was removed from device copy; the flash
  story now lives only in the hidden teaser + `/flash`.
- **Pi Zero board model** in the hero look-inside is a dimensioned
  approximation; a CC-BY Sketchfab model (needs a free account to
  download) is the upgrade path — see `docs/device-3d-hero.md`.
- **Case STLs** (~1.4MB) should be converted to compressed glTF like the
  board GLB before heavy traffic.
- **Real device colors**: cases are brand-stylized (near-black + cyan);
  if the team wants true filament colors, that pass never happened.
- **Demo video** (unblocks DemoSection) and **store listing** (unblocks
  SuiteSection's extension card) are the two content bottlenecks.
