"use client";

import { Suspense, useMemo } from "react";
import { Canvas, useLoader } from "@react-three/fiber";
import { Bounds, Center, Edges, OrbitControls } from "@react-three/drei";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import type { BufferGeometry } from "three";

export type Device3DName = "esp32" | "pi";

const MODELS: Record<Device3DName, string[]> = {
  esp32: ["/models/esp32/touch2.stl"],
  pi: [
    "/models/pi/bottom.stl",
    "/models/pi/top.stl",
    "/models/pi/thumbstick.stl",
    "/models/pi/buttons.stl",
  ],
};

/** One STL part: dark matte case body with a cyan hairline edge (the device motif in 3D). */
function Part({ url }: { url: string }) {
  const geometry = useLoader(STLLoader, url) as BufferGeometry;
  const geo = useMemo(() => {
    geometry.computeVertexNormals();
    return geometry;
  }, [geometry]);
  return (
    <mesh geometry={geo} castShadow receiveShadow>
      <meshStandardMaterial color="#15161a" roughness={0.55} metalness={0.15} />
      <Edges threshold={28} color="#1AF8FF" />
    </mesh>
  );
}

function DeviceModel({ device }: { device: Device3DName }) {
  const urls = MODELS[device];
  return (
    <Bounds fit clip margin={1.15} observe>
      <Center>
        {/* STLs are Z-up (3D-print convention); tip toward camera so the screen faces us. */}
        <group rotation={[-Math.PI / 2, 0, 0]}>
          {urls.map((u) => (
            <Part key={u} url={u} />
          ))}
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
      <directionalLight position={[-4, -2, -3]} intensity={0.4} color="#1AF8FF" />
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
