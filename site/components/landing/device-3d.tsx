"use client";

import { Suspense, useMemo, useRef } from "react";
import { Canvas, useFrame, useLoader } from "@react-three/fiber";
import { Edges, useGLTF, useTexture } from "@react-three/drei";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { Box3, MathUtils, SRGBColorSpace, Vector3 } from "three";
import type { BufferGeometry, Group, Texture } from "three";

export type Device3DName = "esp32" | "pi";

const CASE_COLOR = "#15161a";
const EDGE_COLOR = "#1AF8FF";
/** How far the lid skirt sinks over the bottom shell's rim (mm). */
const PI_LID_OVERLAP = 3;

function CaseMesh({
  geometry,
  position,
  color = CASE_COLOR,
  edges = true,
}: {
  geometry: BufferGeometry;
  position?: [number, number, number];
  color?: string;
  edges?: boolean;
}) {
  return (
    <mesh geometry={geometry} position={position} castShadow receiveShadow>
      <meshStandardMaterial color={color} roughness={0.55} metalness={0.15} />
      {edges && <Edges threshold={28} color={EDGE_COLOR} />}
    </mesh>
  );
}

/** Aperture centers in the assembled lid's X/Z frame, measured by ray-casting
 * a grid onto the flipped lid geometry (see docs/device-3d-hero.md): three
 * 7×4mm button holes around (-26.2, 0) and a 9.2mm thumbstick hole. */
const PI_BUTTONS_XZ: [number, number] = [-26.15, -0.03];
const PI_STICK_XZ: [number, number] = [23.31, -0.93];

/** Controls (buttons + thumbstick) get their own material so they read against
 * the case. Color/edges are the current design-exploration candidate. */
const CONTROL_COLOR = "#e8eaed";
const CONTROL_EDGES = false;

/** Screen content: real pixels, not a flat glow. The Pi shows an actual
 * firmware capture (the SPL-transfer review screen); the ESP32 shows a boot
 * splash built from the brand mark + Departure Mono, matching firmware style. */
function useScreenTexture(url: string): Texture {
  const tex = useTexture(url);
  return useMemo(() => {
    tex.colorSpace = SRGBColorSpace;
    tex.anisotropy = 8;
    tex.needsUpdate = true;
    return tex;
  }, [tex]);
}

/** Pi case: the STLs are a print-bed layout, so assemble by footprint-centering
 * each part and stacking on Y — with the lid overlapping the bottom's rim so
 * the case reads closed, the square 1.3" screen glowing in the aperture, and
 * the printed buttons/thumbstick seated in their measured holes. */
function PiAssembly() {
  const geos = useLoader(STLLoader, [
    "/models/pi/bottom.stl",
    "/models/pi/top.stl",
    "/models/pi/buttons.stl",
    "/models/pi/thumbstick.stl",
  ]) as BufferGeometry[];
  const piScreen = useScreenTexture("/models/screens/pi-review.png");

  const { parts, topY, footprint } = useMemo(() => {
    let y = 0;
    const out: { geometry: BufferGeometry; position: [number, number, number] }[] = [];
    let fp: [number, number] = [0, 0];
    geos.slice(0, 2).forEach((src, i) => {
      // The top/lid (index 1) is exported upside down; flip it before stacking.
      const g = i === 1 ? src.clone().rotateX(Math.PI) : src;
      g.computeVertexNormals();
      g.computeBoundingBox();
      const bb = g.boundingBox!;
      const size = new Vector3();
      const center = new Vector3();
      bb.getSize(size);
      bb.getCenter(center);
      // Center on X/Z (shared footprint); the lid sinks over the bottom's rim.
      if (i === 1) y -= PI_LID_OVERLAP;
      out.push({ geometry: g, position: [-center.x, y - bb.min.y, -center.z] });
      y += size.y;
      if (i === 0) fp = [size.x, size.z];
    });
    // Buttons and thumbstick print base-down: recenter on X/Z, then drop each
    // under the lid top so only its caps poke through the holes.
    ([
      [geos[2], PI_BUTTONS_XZ, 3.3],
      [geos[3], PI_STICK_XZ, 3.5],
    ] as const).forEach(([src, [hx, hz], sink]) => {
      src.computeVertexNormals();
      src.computeBoundingBox();
      const bb = src.boundingBox!;
      const center = new Vector3();
      bb.getCenter(center);
      out.push({
        geometry: src,
        position: [hx - center.x, y - sink - bb.min.y, hz - center.z],
      });
    });
    return { parts: out, topY: y, footprint: fp };
  }, [geos]);

  return (
    // Assembly is X/Z-centered by construction; shift so Y is centered too.
    <group position={[0, -topY / 2, 0]}>
      {parts.map((p, i) => (
        <CaseMesh
          key={i}
          geometry={p.geometry}
          position={p.position}
          color={i >= 2 ? CONTROL_COLOR : CASE_COLOR}
          edges={i >= 2 ? CONTROL_EDGES : true}
        />
      ))}
      {/* dark board filling the interior so apertures don't show through */}
      <mesh position={[0, topY - 2.6, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[footprint[0] * 0.92, footprint[1] * 0.85]} />
        <meshStandardMaterial color="#0b0c0f" roughness={0.7} />
      </mesh>
      {/* the 240×240 screen, recessed just under the lid aperture; sized to
          the measured ~25mm aperture with a small bleed so no gap shows */}
      <mesh position={[-0.4, topY - 1.2, -0.3]} rotation={[-Math.PI / 2, 0, -Math.PI / 2]}>
        <planeGeometry args={[26.5, 26]} />
        <meshBasicMaterial map={piScreen} toneMapped={false} />
      </mesh>
    </group>
  );
}

/** How far the ESP32 board slides out of the case when exploded (mm). */
const ESP32_EXPLODE_MM = 34;

/** The real Waveshare ESP32-S3-Touch-LCD-2 board (manufacturer STEP → GLB,
 * meshopt-compressed). GLB is in meters and shares the case's orientation
 * (front = +Z, portrait Y-up), so it only needs a ×1000 scale into mm space. */
function Esp32Board({ position }: { position: [number, number, number] }) {
  const { scene } = useGLTF("/models/esp32/board.glb");
  const board = useMemo(() => {
    const s = scene.clone(true);
    const bb = new Box3().setFromObject(s);
    const center = bb.getCenter(new Vector3());
    s.position.sub(center); // center the board on its own origin
    return s;
  }, [scene]);
  return (
    <group position={position}>
      {/* 4% under true size so the pin header tucks inside the case walls */}
      <primitive object={board} scale={960} />
    </group>
  );
}

/** ESP32 case with the real Waveshare ESP32-S3-Touch-LCD-2 board inside: its
 * bezel fills the front opening, the lit screen plane rides on the board, and
 * `open` slides the board out of the case. The STL's front is already +Z, so
 * no re-orientation is needed. */
function Esp32Assembly({ open }: { open: boolean }) {
  const geo = useLoader(STLLoader, "/models/esp32/touch2.stl") as BufferGeometry;
  const bootScreen = useScreenTexture("/models/screens/esp32-boot.png");
  const boardRef = useRef<Group>(null);
  const wrapRef = useRef<Group>(null);
  const progress = useRef(0);
  const fit = useMemo(() => {
    geo.computeVertexNormals();
    geo.computeBoundingBox();
    const bb = geo.boundingBox!;
    const size = new Vector3();
    const center = new Vector3();
    bb.getSize(size);
    bb.getCenter(center);
    // Real 2" 240×320 active area, scaled to the case so proportions match.
    const screenH = size.y * 0.66;
    const screenW = screenH * 0.75; // 240:320
    return {
      cx: center.x,
      cy: center.y,
      cz: center.z,
      screenW,
      screenH,
      screenZ: bb.max.z + 0.2,
      offset: [-center.x, -center.y, -center.z] as [number, number, number],
    };
  }, [geo]);

  useFrame((_, delta) => {
    if (!boardRef.current || !wrapRef.current) return;
    // One damped 0→1 progress drives the slide, the recentering that keeps
    // the case+board pair in the middle, and a shrink so the longer exploded
    // assembly still fits the fixed framing.
    const t = (progress.current = MathUtils.damp(
      progress.current,
      open ? 1 : 0,
      4,
      delta,
    ));
    const s = 1 - 0.28 * t;
    boardRef.current.position.z = t * ESP32_EXPLODE_MM;
    wrapRef.current.position.z = (-s * t * ESP32_EXPLODE_MM) / 2;
    wrapRef.current.scale.setScalar(s);
  });

  return (
    // Outer group animates the explode (pivot = model center); inner offset
    // centers the case on the origin.
    <group ref={wrapRef}>
      <group position={fit.offset}>
      <CaseMesh geometry={geo} />
      {/* the board (and its lit screen) slide out together when open */}
      <group ref={boardRef}>
        <Esp32Board position={[fit.cx, fit.cy, fit.cz + 2.5]} />
        <mesh position={[fit.cx, fit.cy, fit.screenZ]}>
          <planeGeometry args={[fit.screenW, fit.screenH]} />
          <meshBasicMaterial map={bootScreen} toneMapped={false} />
        </mesh>
      </group>
      </group>
    </group>
  );
}

useGLTF.preload("/models/esp32/board.glb");

/** Slow turntable driven on the model (not the camera): framing is fixed by
 * construction, so the device can never drift out of the canvas. Honors
 * prefers-reduced-motion by holding a static three-quarter pose. */
function Turntable({ children }: { children: React.ReactNode }) {
  const ref = useRef<Group>(null);
  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  useFrame((_, delta) => {
    if (!ref.current || reduced) return;
    ref.current.rotation.y += delta * 0.25;
  });
  return (
    <group ref={ref} rotation={[0, -0.45, 0]}>
      {children}
    </group>
  );
}

/** Per-device presentation: deterministic scale (mm → scene units) and a tilt
 * applied OUTSIDE the spin. The ESP32 (screen on its front face) does a normal
 * turntable; the Pi (screen on its top face) is tipped toward the camera and
 * spins about its own screen normal, so the screen stays presented all cycle. */
const PRESENTATION: Record<
  Device3DName,
  { scale: number; tilt: [number, number, number] }
> = {
  // 59.3mm tall case → ~2.3 units high, screen already facing +Z.
  esp32: { scale: 2.3 / 59.3, tilt: [0, 0, 0] },
  // 73.8mm wide case, screen on the top face → tip it toward the camera.
  pi: { scale: 2.5 / 73.8, tilt: [Math.PI / 3.2, 0, 0] },
};

function DeviceModel({ device, open }: { device: Device3DName; open: boolean }) {
  const p = PRESENTATION[device];
  return (
    <group rotation={p.tilt}>
      <Turntable>
        <group scale={p.scale}>
          {device === "pi" ? <PiAssembly /> : <Esp32Assembly open={open} />}
        </group>
      </Turntable>
    </group>
  );
}

export function Device3D({ device, open = false }: { device: Device3DName; open?: boolean }) {
  return (
    <Canvas
      camera={{ position: [0, 0, 4], fov: 40 }}
      dpr={[1, 2]}
      className="!h-72 !w-72 sm:!h-80 sm:!w-80"
    >
      <ambientLight intensity={0.9} />
      <directionalLight position={[3, 4, 5]} intensity={1.4} />
      <directionalLight position={[-3, 2, 4]} intensity={0.6} />
      <directionalLight position={[-4, -2, -3]} intensity={0.4} color={EDGE_COLOR} />
      <Suspense fallback={null}>
        <DeviceModel key={device} device={device} open={open} />
      </Suspense>
    </Canvas>
  );
}
