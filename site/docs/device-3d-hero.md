# 3D device hero — implementation spec

Status: in progress (draft PR #117, branch `feat/landing-redesign`)
Last updated: 2026-08-07 (post-verification rewrite)
Component: `site/components/landing/device-3d.tsx`, consumed by `site/components/landing/device-showcase.tsx` in the hero.

## Goal

The landing hero must show that Faraday is a real, working piece of hardware, not a concept. It shows the two physical devices (Raspberry Pi Zero build and ESP32-S3 build), lets the visitor switch between them, and renders each from the team's actual 3D-printed case geometry so it is honest rather than an illustration. The device also carries the brand motif: dark body, cyan accents, the same look as the real screen.

## Inputs: the real case meshes

The team's printed-case STL files were copied into `site/public/models/`:

- `pi/bottom.stl` (3.4k triangles), `pi/top.stl` (12.7k), `pi/thumbstick.stl` (1.2k), `pi/buttons.stl` (1.4k)
- `esp32/touch2.stl` (9.9k)

Measured bounding boxes (mm), which drove the assembly and screen-fit math:

| Part | X | Y | Z | center (X,Y,Z) |
|------|---|---|---|----------------|
| pi/bottom | 73.8 | 16.2 | 34.8 | (-117.6, -2.4, -55.6) |
| pi/top | 73.8 | 12.8 | 34.8 | (10.2, 5.0, 19.4) |
| pi/thumbstick | 9.8 | 7.5 | 9.8 | (65.5, 23.2, -21.1) |
| pi/buttons | 9.8 | 4.8 | 19.8 | (-53.6, 64.9, -120.6) |
| esp32/touch2 | 40.7 | 59.3 | 19.5 | (0.0, -1.8, -5.1) |

Two facts from this table shaped the work:

1. The Pi parts are a print-bed layout, not an assembly. Their centers are scattered across roughly 230mm of space, so loading them at native coordinates renders four pieces floating apart, not a device.
2. `pi/bottom` and `pi/top` share the exact same footprint (73.8 x 34.8) and their heights sum to 29.0mm, which matches the real closed-case height. That is strong evidence they are meant to stack on the height (Y) axis, which is how the assembly is reconstructed below.

The ESP32 board was identified as the Waveshare ESP32-S3-Touch-LCD-2: a 2 inch 240x320 IPS capacitive touch panel with an onboard camera interface. The 240x320 ratio (0.75 portrait) and the ~30.5 x 40.6mm active area of a 2 inch panel drive the screen-fit math.

## Tech stack and why

- `three` 0.180, `@react-three/fiber` 9.7, `@react-three/drei` 10.7 (plus `@types/three`).
- react-three-fiber was chosen over a pre-rendered image/video because the meshes are low-poly (ESP32 ~10k tris, full Pi ~19k tris), so real-time rendering is smooth even on mobile, and it gives interactive rotate plus a live device switch with no per-device render pipeline.
- Rejected alternatives: (a) pre-rendered turntable images, which cannot be produced in this environment and lose interactivity; (b) sourcing full board CAD models, which is heavier (licensing, STEP-to-glTF conversion, alignment, 1 to 3MB downloads) and is documented as a possible later upgrade, not needed for the hero.
- The STLs are loaded directly (binary STL, ~1.4MB total, lazy-loaded after paint). A later optimization is converting to Draco-compressed glTF to cut this to roughly 150KB.

## Component architecture

`device-3d.tsx` (client component) exports `Device3D({ device, open })`:

- A single `<Canvas>` with a fixed camera at `[0, 0, 4]`, an ambient light, a key directional, a soft white front fill (so the real board's PCB reads), and a low cyan rim light (`#1AF8FF`).
- Framing is deterministic by construction — no drei `Bounds`/`Center` and no `OrbitControls`. Each assembly centers itself on the origin from its own known bounding boxes, and a per-device `PRESENTATION` entry supplies an exact mm→scene scale and a tilt. (The earlier `Bounds observe` + `OrbitControls` combination re-fit the camera mid-interaction and let the model drift out of the canvas; both are gone.)
- Motion is a `Turntable` group that rotates the model, not the camera, at 0.25 rad/s, honoring `prefers-reduced-motion` by holding a static three-quarter pose. The tilt sits OUTSIDE the spin: the ESP32 (screen on its front face) does a normal turntable, while the Pi (screen on its top face) is tipped toward the camera and spins about its own screen normal, so its screen stays presented all cycle.
- Orientation: no blanket Z-up correction. The ESP32 STL's front is natively +Z (verified by render), so it needs no rotation; the Pi assembly is stacked along Y and gets its presentation tilt only.
- `CaseMesh` is the shared material: `meshStandardMaterial` in near-black (`#15161a`), roughness 0.55, metalness 0.15, with a drei `<Edges color="#1AF8FF">` overlay. This is the device motif in 3D: dark matte body, cyan hairline edges.

`device-showcase.tsx` loads `Device3D` via `next/dynamic` with `ssr: false` (a WebGL canvas cannot server-render) and a text loading fallback. It holds the `esp32 | pi` selection and the `open` (look-inside) state, renders the device/segment switch as `aria-pressed` toggle buttons (not a tablist, since there is no tabpanel) with a visible focus ring, a "look inside / close it up" toggle for the ESP32, and the caption "two devices, same firmware".

## Pi assembly logic and why

Because the STLs are a print-bed layout, `PiAssembly` reconstructs the closed case rather than trusting native positions. For each of `bottom` then `top`:

1. Compute the geometry bounding box.
2. Recenter the part on X and Z (they share a footprint, so this aligns them).
3. Stack on Y: place each part's base at a running Y cursor, then advance the cursor by the part's height — with the lid sunk 3mm over the bottom's rim (`PI_LID_OVERLAP`). Stacking the raw bounding-box heights edge-to-edge left a visible, see-through seam between the shells (verified by render); the overlap closes the case.

The lid (`top`, index 1) was exported upside down, so it is rotated 180 degrees around X (`clone().rotateX(Math.PI)`) before its box is measured and stacked. The clone avoids mutating the loader-cached geometry (which would double-apply under React strict-mode remounts).

The thumbstick and buttons are deliberately omitted. Their mount positions are not encoded in the print-layout files (each sits at its own arbitrary coordinate), so they cannot be placed accurately without the assembled CAD. A misplaced control looks worse than none, and the closed bottom-plus-top case already reads as a complete device. Adding them back requires either an assembled export or explicit mount coordinates.

The Pi also gets its screen treatment now: a dark interior plane blocks see-through, and a lit plane recessed just under the lid shows through the screen aperture — the aperture itself masks it, so exact aperture coordinates aren't needed.

## ESP32: the real board inside, and the look-inside explode

The fake bezel plane is gone: the case now contains the real Waveshare ESP32-S3-Touch-LCD-2 board, converted from Waveshare's official STEP model (from their wiki's "2D/3D file" resource) to a 339KB meshopt-compressed GLB at `site/public/models/esp32/board.glb`. Pipeline: `cascadio` (Python, OpenCascade) STEP→GLB, then `gltfpack -si 0.6 -cc` (simplify to 60%, meshopt-compress); drei's `useGLTF` decodes meshopt out of the box. The source CAD is 418 parts / 247k triangles with real material colors (blue PCB, gold GPIO headers, USB-C, FPC connector, camera).

Placement (all verified by render): the GLB shares the case's orientation (front +Z, portrait Y-up) and is in meters, so it scales ×960 into the mm scene — 4% under true size so the pin header clears the case walls — and sits 2.5mm forward of the case-center so the pin tips don't pierce the back wall. The lit screen plane (66% of case height, 240:320 aspect, cyan emissive 0.45) rides in front of the board's own dark panel so the device reads as powered.

`open` drives a look-inside explode: one damped 0→1 progress slides the board 34mm out of the case front, shifts the pair back by half that so it stays centered, and shrinks the wrapper 28% so the longer exploded assembly still fits the fixed framing.

## Resolved by visual verification (2026-08-07)

Everything in the earlier "known approximations" list has been confirmed or fixed against live renders (headless-browser screenshots of the running site plus a standalone three.js test harness):

- The ESP32 STL's front is natively +Z — the earlier blanket -90° X rotation was wrong and made the hero show the empty back shell with the screen facing up. Removed.
- Pi lid flip around X is correct; the seam needed a 3mm overlap.
- The Pi is no longer an empty shell (interior plane + lit aperture screen).
- The board fits the case with a 4% shrink and 2.5mm forward offset (pin tips pierced the back wall at true scale/center).

## Accessibility and performance

- The turntable is gated on `prefers-reduced-motion` (holds a static three-quarter pose).
- Device switch and look-inside buttons use `aria-pressed` and a visible `focus-visible` ring on `--ring`.
- Canvas dpr is clamped to `[1, 2]`; there is no user camera control at all, so the model can never leave the frame.
- Board GLB is meshopt-compressed to 339KB and lazy-loaded with the canvas. Typecheck clean.

## Next steps / open options

1. Real Pi Zero board for a Pi look-inside: no official CAD exists (Raspberry Pi only publishes STEP for the Pi 5); best sources are two CC-BY 4.0 Sketchfab models ("Raspberry Pi Zero" by GameDevGiant, "Raspberry Pi Zero W v1.1" by Vacui — attribution required, downloading needs a Sketchfab account). GrabCAD models are non-commercial-only; AI image-to-3D was researched and rejected (melts small angular PCBs).
2. Optional: put the live firmware UI on the screen (the menu to review to signed-QR cycle) as a texture.
3. Before ship: convert the case STLs (~1.4MB) to compressed glTF like the board to cut download weight.

## Sources

- Waveshare ESP32-S3-Touch-LCD-2 product page and wiki (2 inch, 240x320 IPS, onboard camera).
