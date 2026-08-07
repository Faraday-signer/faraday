# 3D device hero — implementation spec

Status: in progress (draft PR #117, branch `feat/landing-redesign`)
Last updated: 2026-08-07
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

`device-3d.tsx` (client component) exports `Device3D({ device })`:

- A single `<Canvas>` with an ambient light, a key directional light, and a low cyan rim light (`#1AF8FF`) so the cyan reads on the dark body.
- `DeviceModel` wraps the active device in drei `<Bounds fit clip observe>` and `<Center>` so each device is auto-framed regardless of its native coordinates and size. A `<group rotation={[-Math.PI/2, 0, 0]}>` stands the meshes up, because STLs use the 3D-print Z-up convention and the scene is Y-up.
- `<OrbitControls>` with `autoRotate` (slow turntable) and drag-to-rotate enabled, but pan and zoom disabled and the polar angle clamped so the visitor cannot flip it to an unflattering angle.
- `CaseMesh` is the shared material: `meshStandardMaterial` in near-black (`#15161a`), roughness 0.55, metalness 0.15, with a drei `<Edges color="#1AF8FF">` overlay. This is the device motif in 3D: dark matte body, cyan hairline edges.

`device-showcase.tsx` loads `Device3D` via `next/dynamic` with `ssr: false` (a WebGL canvas cannot server-render) and a text loading fallback. It holds the `esp32 | pi` selection, renders the device/segment switch as `aria-pressed` toggle buttons (not a tablist, since there is no tabpanel) with a visible focus ring, and the caption "two devices, same firmware, drag to rotate".

## Pi assembly logic and why

Because the STLs are a print-bed layout, `PiAssembly` reconstructs the closed case rather than trusting native positions. For each of `bottom` then `top`:

1. Compute the geometry bounding box.
2. Recenter the part on X and Z (they share a footprint, so this aligns them).
3. Stack on Y: place each part's base at a running Y cursor, then advance the cursor by the part's height. Bottom occupies Y 0 to 16.2, top occupies 16.2 to 29.0, matching the real 29mm case height.

The lid (`top`, index 1) was exported upside down, so it is rotated 180 degrees around X (`clone().rotateX(Math.PI)`) before its box is measured and stacked. The clone avoids mutating the loader-cached geometry (which would double-apply under React strict-mode remounts).

The thumbstick and buttons are deliberately omitted. Their mount positions are not encoded in the print-layout files (each sits at its own arbitrary coordinate), so they cannot be placed accurately without the assembled CAD. A misplaced control looks worse than none, and the closed bottom-plus-top case already reads as a complete device. Adding them back requires either an assembled export or explicit mount coordinates.

## ESP32 screen fit and why

The first pass put a generic emissive plane sized to 0.72 x 0.82 of the case face, which had the wrong aspect and did not match the case opening. `Esp32Assembly` now fits the real panel:

- Screen height is 66 percent of the case height; screen width is height times 0.75 (the 240:320 ratio). This reproduces the real 2 inch panel proportions inside the case.
- A dark bezel/board plane (90 percent of the case face, near-black) sits just inside the front opening so the case is not a see-through hollow shell.
- The lit screen plane sits just proud of the front face, dark base color with a cyan emissive at intensity 0.45, so it reads as a powered display.

Both planes are positioned from the case bounding box (front is +Z, centered on X and Y) and live inside the same transformed group, so they inherit the Center/Bounds/rotation with the case.

## Known approximations (need visual confirmation)

These were placed without a rendered view and are the most likely to need a nudge:

- ESP32 screen is centered on the front face; the real board may sit high with a chin at the bottom.
- Screen face is assumed to be +Z; if it renders on the back, one sign flip fixes it.
- Pi lid flip is around X; if still wrong, Z is the alternative axis.
- The Pi case is still an empty shell (no screen/backing yet); the ESP32 treatment applies to it next, with the square Waveshare 1.3 inch 240x240 screen on the top face.

## Accessibility and performance

- `prefers-reduced-motion` is honored elsewhere on the page; the canvas auto-rotate is a slow ambient motion and should be gated for reduced-motion in a follow-up (noted).
- Device switch buttons use `aria-pressed` and a visible `focus-visible` ring on `--ring`.
- Canvas dpr is clamped to `[1, 2]`; zoom/pan disabled; polar angle clamped.
- Verified with a production `next build`: all routes prerender and the 3D module bundles cleanly. Typecheck clean.

## Next steps / open options

1. Confirm ESP32 screen placement and face; adjust position/aspect as needed.
2. Apply the screen-plus-bezel-plus-backing treatment to the Pi (top-face 240x240 square screen).
3. Optional realism upgrade: source manufacturer CAD (Waveshare STEP for the ESP32-S3-Touch-LCD-2; Pi Zero and Waveshare 1.3 inch LCD models), convert to Draco glTF, and place inside the cases.
4. Optional: put the live firmware UI on the screen (the earlier menu to review to signed-QR cycle) as a texture.
5. Before ship: convert STLs to compressed glTF to cut download weight.

## Sources

- Waveshare ESP32-S3-Touch-LCD-2 product page and wiki (2 inch, 240x320 IPS, onboard camera).
