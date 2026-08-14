# FA-20 — Spatial screensaver: what we built, how Atropos actually works, and what we're missing

Status: analysis after 8 on-device iterations. The screensaver works and is
tunable, but it doesn't yet produce the "it's really 3D" sensation. This doc
specs what exists, dissects Atropos's model from its source, and names the
conceptual gap.

## 1. Spec of what we built

### Hardware / plumbing
- QMI8658 6-axis IMU on the Waveshare ESP32-S3-Touch-LCD-2, sharing the
  CST816D touch I2C bus (SDA=48, SCL=47) via `Rc<RefCell<I2cDriver>>`.
  Accelerometer only: ±4g, 58.75 Hz, address probed at 0x6A/0x6B
  (`esp32-touch2/src/imu.rs`).
- New `BoardImu` board trait + `NoImu` placeholder; `esp32_common::run`
  takes `Option<M: BoardImu>` and feeds `tilt()` into the splash draw. A
  board without the chip keeps the DVD-bounce (`esp32-common/src/board.rs`).
- Axis calibration (on hardware, photos 2026-08-14): chip +X points at the
  screen's top edge, chip +Y at its right edge; screen x = AY, y = −AX.
- Signal path: raw gravity → fast low-pass (α=0.55, noise) minus slow
  low-pass (α=0.10, resting pose) × GAIN 4 → tilt in [−1, 1] per axis.
  High-pass means the effect answers *changes* in tilt and re-centers in
  <1 s at any holding angle.
- Screensaver-only draw rate raise: 30 Hz → 40 Hz (`run.rs`).

### The scene (core/src/gui/screens.rs, draw_splash tilt branch)
Layer stack, back to front, each with a hand-picked velocity ratio of the
scene travel `NEAR_PX = 14 px`:

| layer | content | ratio (× tilt) |
|---|---|---|
| ticks | "+" crosses, 100 px lattice | −0.8 (counter) |
| wall | site measurement grid, 20/100 px minor/major | −0.4 (counter) |
| shadow | dark isotype silhouette on the wall | −0.5 (counter, with wall) |
| mark | 180 px solid-cyan isotype, 4 squares | +0.7 (with tilt) |
| glare | 22 px diagonal light band, clipped to the mark | ±(tx+ty)×70 px (fastest) |

- Sub-pixel motion: `fill_rect_aa` draws the mark at fractional positions
  (edge rows/cols coverage-blended against bg). Killed the "laggy stepping".
- Everything is axis-aligned rect fills; whole scene costs single-digit ms
  against a ~25 ms frame budget. Resources are not the constraint.

### Tried and rejected on-device
- Internal mark split (big pair vs small pair at different rates) — read as
  the mark falling apart, not depth.
- Vertical gradient fill on the mark — "don't like the gradient".
- Teal ghost copy as shadow — read as a smeared duplicate, replaced by
  bg-dark silhouette that occludes grid lines.
- Superlinear "fling" past the frame edge — rejected outright.

## 2. How Atropos actually works (from src/atropos.js)

The load-bearing fact: **Atropos is not translation parallax. It is a 3D
rotation of one scene, in perspective; the layer parallax is a derived
byproduct of that single rotation.**

1. **Master transform = rotation.** Pointer position maps to scene tilt:
   `rotateY = rotateYMax·(x−cx)/(w/2)·−1`, `rotateX = rotateXMax·(y−cy)/(h/2)`
   (defaults ±15°), applied as
   `translate3d(...) rotateX(θx) rotateY(θy)` on the scene element, inside a
   CSS `perspective`. The card physically **tips**: its silhouette
   foreshortens into a trapezoid, near edge grows, far edge shrinks.
2. **Layers derive from the same rotation.** A child with
   `data-atropos-offset=o` gets
   `translate3d(−rotYpct·−o%, rotXpct·−o%, 0)` — its displacement is the
   master rotation percentage scaled by its depth. One geometric source of
   truth; nothing is animated independently.
3. **Shadow is part of the 3D model:** `translate3d(0,0,−shadowOffset)
   scale(shadowScale)` under the scene — it tips with the card and stays
   planted, grounding it.
4. **Highlight breathes:** it translates at ratio 0.25 of rotation and its
   **opacity scales with rotation magnitude** — motionless card, no glare;
   the glint only lives while you move.
5. **Active z-lift:** on enter, the scene translates forward
   `translate3d(0,0,activeOffset)` (default 50 px) over `duration: 300` ms —
   the card *approaches you* when it wakes.

## 3. Comparison

| | ours | Atropos |
|---|---|---|
| master model | N independent 2D translations | one 3D rotation + perspective |
| silhouette under tilt | unchanged (always frontal) | foreshortens / tips |
| layer displacement | hand-tuned ratios | derived from rotation × offset |
| shadow | translates on the wall | fixed under scene, tips with it |
| glare | constant intensity, position-driven | opacity ∝ motion, dies at rest |
| wake moment | none | z-lift toward viewer, 300 ms |

The user-reported gap — "the isotype doesn't feel 3D or continuous" — is
row 2. A translation-only scene never changes any shape, and shape change
under tilt (foreshortening) is the strongest monocular 3D cue there is.
Every ratio we tuned was compensating for its absence.

## 4. What to port (ranked, all feasible on our renderer)

1. **Tip the mark (scanline trapezoid warp).** Small-angle perspective of
   an axis-aligned square is scanline-friendly: per row, width and x-offset
   interpolate linearly (`scale(y) ≈ 1 + k·(y−cy)/cy`, k ≈ 0.05–0.10 for
   ~10–15°). Render the 4 squares as row strips with per-row scale — same
   pixel count as today. rotateY = per-column version; both = projected
   corner lerp. This buys the missing foreshortening.
2. **Glare opacity ∝ motion magnitude.** We already have |tilt|; blend the
   band color toward FD_ACCENT as motion → 0 so the glint only lives in
   motion (one `blend565` call).
3. **Wake z-lift.** On entering the screensaver, scale the mark 96 % → 100 %
   over ~300 ms (row-strip scaling again). Gives the "it noticed you" beat.
4. **Keep** the high-pass recentring, sub-pixel edges, 40 Hz, layer stack —
   but re-derive every layer offset from the *rotation* value so the scene
   has one source of truth (offsets become `data-atropos-offset`-style
   depth numbers, not velocities).

Perspective-warping the full-screen grid is the expensive one (per-row line
plotting, ~4–8 ms) — defer; a tipped mark over a flat translating wall is
already how Atropos scenes read (bg layers there barely move either).

## 5. Open question

Even with all four ports, a lone geometric mark is a quiet subject. If the
tipped-card prototype still underwhelms, the honest options are (a) accept
it as a subtle idle nicety, or (b) park the effect and keep the IMU driver +
`BoardImu` plumbing, which are durable infrastructure either way (FA-20's
acceptance criteria cover the fallback path).

## 6. Outcome (2026-08-15)

The tipped-card prototype was built and flashed (all four ports from §4)
and still didn't earn its keep on-device. Option (b) won, with a twist:
the parallax was parked, the screensaver went back to the classic bounce
(theme-aware), and the IMU plumbing became the sensor for a **shake
gesture that toggles a light/dark theme** — the light palette mirroring
the site's light side, brand-ink accent included. FA-20 shipped as "IMU
support + shake-to-theme". This doc stays as the design record of why
translation parallax alone can't read as 3D, and what a future attempt
should reach for first (rotation/foreshortening, glare ∝ motion).
