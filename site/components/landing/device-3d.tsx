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
    for (const g of geos) {
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
    }
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

/** ESP32 case + an emissive cyan screen on the front face, so it reads as a
 * powered device rather than an empty shell. */
function Esp32Assembly() {
  const geo = useLoader(STLLoader, "/models/esp32/touch2.stl") as BufferGeometry;
  const screen = useMemo(() => {
    geo.computeVertexNormals();
    geo.computeBoundingBox();
    const bb = geo.boundingBox!;
    const size = new Vector3();
    const center = new Vector3();
    bb.getSize(size);
    bb.getCenter(center);
    // Portrait front face (X × Y) at the +Z side; screen fills most of it.
    return {
      w: size.x * 0.72,
      h: size.y * 0.82,
      pos: [center.x, center.y, bb.max.z + 0.4] as [number, number, number],
    };
  }, [geo]);

  return (
    <group>
      <CaseMesh geometry={geo} />
      <mesh position={screen.pos}>
        <planeGeometry args={[screen.w, screen.h]} />
        <meshStandardMaterial
          color="#0a1418"
          emissive={EDGE_COLOR}
          emissiveIntensity={0.35}
          roughness={0.3}
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
