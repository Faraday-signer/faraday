"use client";

import { Suspense, useMemo } from "react";
import { Canvas, useLoader } from "@react-three/fiber";
import { Bounds, Center, Edges, OrbitControls } from "@react-three/drei";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { Vector3 } from "three";
import type { BufferGeometry } from "three";

export type Device3DName = "esp32" | "pi";

const CASE_COLOR = "#15161a";
const EDGE_COLOR = "#1AF8FF";

function CaseMesh({ geometry, position }: { geometry: BufferGeometry; position?: [number, number, number] }) {
  return (
    <mesh geometry={geometry} position={position} castShadow receiveShadow>
      <meshStandardMaterial color={CASE_COLOR} roughness={0.55} metalness={0.15} />
      <Edges threshold={28} color={EDGE_COLOR} />
    </mesh>
  );
}

/** Pi case: the STLs are a print-bed layout, so assemble by footprint-centering
 * each part and stacking on Y (bottom then top → the closed ~29mm case). */
function PiAssembly() {
  const geos = useLoader(STLLoader, [
    "/models/pi/bottom.stl",
    "/models/pi/top.stl",
  ]) as BufferGeometry[];

  const parts = useMemo(() => {
    let y = 0;
    const out: { geometry: BufferGeometry; position: [number, number, number] }[] = [];
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
      // Center on X/Z (shared footprint), sit this part's base at the running Y.
      out.push({ geometry: g, position: [-center.x, y - bb.min.y, -center.z] });
      y += size.y;
    });
    return out;
  }, [geos]);

  return (
    <group>
      {parts.map((p, i) => (
        <CaseMesh key={i} geometry={p.geometry} position={p.position} />
      ))}
    </group>
  );
}

/** ESP32 case with the Waveshare ESP32-S3-Touch-LCD-2 up front: a 2" 240×320
 * screen (real ~30.5×40.6mm active area) in a dark bezel, over a dark interior
 * so the case reads as a finished, powered device rather than an empty shell. */
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
    };
  }, [geo]);

  return (
    <group>
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

function DeviceModel({ device }: { device: Device3DName }) {
  return (
    <Bounds fit clip margin={1.15} observe>
      <Center>
        {/* STLs are Z-up (3D-print convention); stand them up to face the camera. */}
        <group rotation={[-Math.PI / 2, 0, 0]}>
          {device === "pi" ? <PiAssembly /> : <Esp32Assembly />}
        </group>
      </Center>
    </Bounds>
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
      <OrbitControls
        makeDefault
        enablePan={false}
        enableZoom={false}
        autoRotate
        autoRotateSpeed={1.1}
        minPolarAngle={Math.PI / 3}
        maxPolarAngle={(2 * Math.PI) / 3}
      />
    </Canvas>
  );
}
