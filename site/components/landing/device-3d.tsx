"use client";

import { Suspense, useMemo, useRef } from "react";
import { Canvas, useFrame, useLoader } from "@react-three/fiber";
import { Edges } from "@react-three/drei";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { Vector3 } from "three";
import type { BufferGeometry, Group } from "three";

export type Device3DName = "esp32" | "pi";

const CASE_COLOR = "#15161a";
const EDGE_COLOR = "#1AF8FF";
/** How far the lid skirt sinks over the bottom shell's rim (mm). */
const PI_LID_OVERLAP = 3;

function CaseMesh({ geometry, position }: { geometry: BufferGeometry; position?: [number, number, number] }) {
  return (
    <mesh geometry={geometry} position={position} castShadow receiveShadow>
      <meshStandardMaterial color={CASE_COLOR} roughness={0.55} metalness={0.15} />
      <Edges threshold={28} color={EDGE_COLOR} />
    </mesh>
  );
}

/** Pi case: the STLs are a print-bed layout, so assemble by footprint-centering
 * each part and stacking on Y — with the lid overlapping the bottom's rim so
 * the case reads closed, plus the square 1.3" screen glowing in the aperture. */
function PiAssembly() {
  const geos = useLoader(STLLoader, [
    "/models/pi/bottom.stl",
    "/models/pi/top.stl",
  ]) as BufferGeometry[];

  const { parts, topY, footprint } = useMemo(() => {
    let y = 0;
    const out: { geometry: BufferGeometry; position: [number, number, number] }[] = [];
    let fp: [number, number] = [0, 0];
    geos.forEach((src, i) => {
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
    return { parts: out, topY: y, footprint: fp };
  }, [geos]);

  return (
    // Assembly is X/Z-centered by construction; shift so Y is centered too.
    <group position={[0, -topY / 2, 0]}>
      {parts.map((p, i) => (
        <CaseMesh key={i} geometry={p.geometry} position={p.position} />
      ))}
      {/* dark board filling the interior so apertures don't show through */}
      <mesh position={[0, topY - 2.6, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[footprint[0] * 0.92, footprint[1] * 0.85]} />
        <meshStandardMaterial color="#0b0c0f" roughness={0.7} />
      </mesh>
      {/* the lit 240×240 screen, recessed just under the lid aperture */}
      <mesh position={[0, topY - 1.2, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[30, 26]} />
        <meshStandardMaterial
          color="#08121a"
          emissive={EDGE_COLOR}
          emissiveIntensity={0.45}
          roughness={0.25}
        />
      </mesh>
    </group>
  );
}

/** ESP32 case with the Waveshare ESP32-S3-Touch-LCD-2 up front: a 2" 240×320
 * screen in a dark bezel over a dark interior. The STL's front is already +Z,
 * so no re-orientation is needed. */
function Esp32Assembly() {
  const geo = useLoader(STLLoader, "/models/esp32/touch2.stl") as BufferGeometry;
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
    const frontZ = bb.max.z;
    return {
      cx: center.x,
      cy: center.y,
      screenW,
      screenH,
      bezelW: size.x * 0.9,
      bezelH: size.y * 0.9,
      bezelZ: frontZ - 0.6,
      screenZ: frontZ + 0.2,
      offset: [-center.x, -center.y, -center.z] as [number, number, number],
    };
  }, [geo]);

  return (
    // Offset the whole group so the case is centered on the origin.
    <group position={fit.offset}>
      <CaseMesh geometry={geo} />
      {/* dark bezel/board filling the front opening (kills the hollow look) */}
      <mesh position={[fit.cx, fit.cy, fit.bezelZ]}>
        <planeGeometry args={[fit.bezelW, fit.bezelH]} />
        <meshStandardMaterial color="#0b0c0f" roughness={0.7} />
      </mesh>
      {/* the lit screen */}
      <mesh position={[fit.cx, fit.cy, fit.screenZ]}>
        <planeGeometry args={[fit.screenW, fit.screenH]} />
        <meshStandardMaterial
          color="#08121a"
          emissive={EDGE_COLOR}
          emissiveIntensity={0.45}
          roughness={0.25}
        />
      </mesh>
    </group>
  );
}

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

function DeviceModel({ device }: { device: Device3DName }) {
  const p = PRESENTATION[device];
  return (
    <group rotation={p.tilt}>
      <Turntable>
        <group scale={p.scale}>
          {device === "pi" ? <PiAssembly /> : <Esp32Assembly />}
        </group>
      </Turntable>
    </group>
  );
}

export function Device3D({ device }: { device: Device3DName }) {
  return (
    <Canvas
      camera={{ position: [0, 0, 4], fov: 40 }}
      dpr={[1, 2]}
      className="!h-72 !w-72 sm:!h-80 sm:!w-80"
    >
      <ambientLight intensity={0.9} />
      <directionalLight position={[3, 4, 5]} intensity={1.4} />
      <directionalLight position={[-4, -2, -3]} intensity={0.4} color={EDGE_COLOR} />
      <Suspense fallback={null}>
        <DeviceModel key={device} device={device} />
      </Suspense>
    </Canvas>
  );
}
