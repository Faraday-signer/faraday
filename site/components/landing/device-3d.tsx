"use client";

import { Suspense, useEffect, useMemo, useRef } from "react";
import { Canvas, useFrame, useLoader, useThree } from "@react-three/fiber";
import { Edges, Environment, Lightformer, Text, useGLTF } from "@react-three/drei";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { Box3, MathUtils, Shape, Vector3 } from "three";
import type { BufferGeometry, Group } from "three";

export type Device3DName = "esp32" | "pi";

const CASE_COLOR = "#17191d";
const EDGE_COLOR = "#1AF8FF";
/** Hairline accent, dialed down so cyan reads as a detail, not a paint job. */
const EDGE_DIM = "#0d5f6b";
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
      {/* soft-touch shell: matte base under a light clearcoat sheen */}
      <meshPhysicalMaterial
        color={color}
        roughness={0.42}
        metalness={0.1}
        clearcoat={0.55}
        clearcoatRoughness={0.3}
      />
      {edges && <Edges threshold={28} color={EDGE_DIM} />}
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
      {/* the live 240×240 firmware UI, recessed just under the lid aperture
          (~25mm), its edges masked by the aperture rim */}
      <group
        position={[-0.4, topY - 1.2, -0.3]}
        rotation={[-Math.PI / 2, 0, 0]}
        scale={25.8 / 240}
      >
        <PiScreenUI />
      </group>
    </group>
  );
}

/** How far the ESP32 board slides out of the case when exploded (mm). */
const ESP32_EXPLODE_MM = 34;

/** Firmware UI palette (Theme::faraday_320 verbatim). */
const FW = {
  bg: "#001721",
  text: "#E7E7E7",
  muted: "#8C9CA8",
  dim: "#5E7180",
  accent: EDGE_COLOR,
  border: "#114358",
  danger: "#FF6B6B",
};
const FW_FONT = "/fonts/DepartureMono-Regular.ttf";

/** One firmware-styled text run, in the screen's logical 240×320 px space. */
function FwText({
  x,
  y,
  size,
  color,
  align = "left",
  children,
}: {
  x: number;
  y: number;
  size: number;
  color: string;
  align?: "left" | "right" | "center";
  children: string;
}) {
  return (
    <Text
      font={FW_FONT}
      fontSize={size}
      color={color}
      anchorX={align}
      anchorY="middle"
      letterSpacing={0.02}
      position={[x, -y, 0.4]}
    >
      {children}
    </Text>
  );
}

/** Thin filled rect in logical px (hairlines, dividers, icon strokes). */
function FwRect({
  x,
  y,
  w,
  h,
  color,
  rotation = 0,
}: {
  x: number; // center x, logical px
  y: number; // center y, logical px
  w: number;
  h: number;
  color: string;
  rotation?: number;
}) {
  return (
    <mesh position={[x, -y, 0.3]} rotation={[0, 0, rotation]}>
      <planeGeometry args={[w, h]} />
      <meshBasicMaterial color={color} toneMapped={false} />
    </mesh>
  );
}

const QP = Math.PI / 4;

/** Rounded-rect Shape centered on the origin (dims in mm). */
function roundedRectShape(w: number, h: number, r: number): Shape {
  const s = new Shape();
  const hw = w / 2, hh = h / 2;
  s.moveTo(-hw + r, -hh);
  s.lineTo(hw - r, -hh); s.quadraticCurveTo(hw, -hh, hw, -hh + r);
  s.lineTo(hw, hh - r); s.quadraticCurveTo(hw, hh, hw - r, hh);
  s.lineTo(-hw + r, hh); s.quadraticCurveTo(-hw, hh, -hw, hh - r);
  s.lineTo(-hw, -hh + r); s.quadraticCurveTo(-hw, -hh, -hw + r, -hh);
  return s;
}

/** The ESP32 touch build's zoned APPROVE SEND screen, built as live geometry
 * (SDF text + rects) instead of a bitmap — crisp at any angle. Layout is the
 * firmware's own 240×320 grid: 29px header, three body zones at y 29/111/193,
 * and the 44px bottom touch action bar (reject / next / sign). */
function Esp32ScreenUI() {
  const FROM = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
  const TO = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
  const roundedScreen = useMemo(() => {
    const s = new Shape();
    const w = 120, h = 160, r = 14;
    s.moveTo(-w + r, -h);
    s.lineTo(w - r, -h); s.quadraticCurveTo(w, -h, w, -h + r);
    s.lineTo(w, h - r); s.quadraticCurveTo(w, h, w - r, h);
    s.lineTo(-w + r, h); s.quadraticCurveTo(-w, h, -w, h - r);
    s.lineTo(-w, -h + r); s.quadraticCurveTo(-w, -h, -w + r, -h);
    return s;
  }, []);

  return (
    <>
      {/* panel background */}
      <mesh>
        <shapeGeometry args={[roundedScreen]} />
        <meshBasicMaterial color={FW.bg} toneMapped={false} />
      </mesh>
      <group position={[-120, 160, 0]}>

      {/* header */}
      <FwText x={12} y={17} size={14} color={FW.accent}>APPROVE SEND</FwText>
      <FwText x={228} y={17} size={14} color={FW.dim} align="right">1/3</FwText>
      <FwRect x={120} y={28.5} w={240} h={1} color={FW.border} />

      {/* zone dividers */}
      <FwRect x={120} y={111} w={240} h={1} color={FW.border} />
      <FwRect x={120} y={193} w={240} h={1} color={FW.border} />

      {/* FROM zone */}
      <FwText x={12} y={45} size={14} color={FW.muted}>FROM</FwText>
      <FwText x={56} y={45} size={14} color={FW.accent}>(you)</FwText>
      <FwText x={12} y={66} size={13.5} color={FW.text}>{FROM.slice(0, 22)}</FwText>
      <FwText x={12} y={84} size={13.5} color={FW.text}>{FROM.slice(22)}</FwText>

      {/* TO zone */}
      <FwText x={12} y={127} size={14} color={FW.muted}>TO</FwText>
      <FwText x={12} y={148} size={13.5} color={FW.text}>{TO.slice(0, 22)}</FwText>
      <FwText x={12} y={166} size={13.5} color={FW.text}>{TO.slice(22)}</FwText>

      {/* AMOUNT zone */}
      <FwText x={12} y={210} size={17} color={FW.muted}>AMOUNT</FwText>
      <FwText x={228} y={210} size={17} color={FW.accent} align="right">SOL</FwText>
      <FwText x={228} y={252} size={26} color={FW.text} align="right">1.5</FwText>

      {/* bottom touch action bar */}
      <FwRect x={120} y={276} w={240} h={1} color={FW.border} />
      <FwRect x={80} y={298} w={1} h={44} color={FW.border} />
      <FwRect x={160} y={298} w={1} h={44} color={FW.border} />
      {/* reject (danger) */}
      <FwRect x={40} y={298} w={17} h={3} color={FW.danger} rotation={QP} />
      <FwRect x={40} y={298} w={17} h={3} color={FW.danger} rotation={-QP} />
      {/* next page (arrow) */}
      <FwRect x={118} y={298} w={12} h={3} color={FW.text} />
      <FwRect x={124} y={295.5} w={9} h={3} color={FW.text} rotation={-QP} />
      <FwRect x={124} y={300.5} w={9} h={3} color={FW.text} rotation={QP} />
      {/* sign (accent check) */}
      <FwRect x={195.5} y={301.5} w={9} h={3} color={FW.accent} rotation={-QP} />
      <FwRect x={202} y={298.5} w={15} h={3} color={FW.accent} rotation={QP} />
      </group>
    </>
  );
}

/** The Pi build's zoned APPROVE SEND screen (240×240, physical keys): same
 * three zones on the firmware's grid — header 29px, zones at y 29/99/169 —
 * with the K1/K2/K3 edge-hint gutter column on the right instead of a
 * bottom bar, matching the case's real side buttons. */
function PiScreenUI() {
  const FROM = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
  const TO = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
  const bg = useMemo(() => roundedRectShape(240, 240, 6), []);
  return (
    <>
      <mesh>
        <shapeGeometry args={[bg]} />
        <meshBasicMaterial color={FW.bg} toneMapped={false} />
      </mesh>
      <group position={[-120, 120, 0]}>

      {/* header */}
      <FwText x={12} y={17} size={14} color={FW.accent}>APPROVE SEND</FwText>
      <FwText x={228} y={17} size={14} color={FW.dim} align="right">1/3</FwText>
      <FwRect x={120} y={28.5} w={240} h={1} color={FW.border} />

      {/* zone dividers (full width) + gutter column border */}
      <FwRect x={120} y={99} w={240} h={1} color={FW.border} />
      <FwRect x={120} y={169} w={240} h={1} color={FW.border} />
      <FwRect x={212} y={134} w={1} h={212} color={FW.border} />

      {/* FROM zone */}
      <FwText x={12} y={43} size={13} color={FW.muted}>FROM</FwText>
      <FwText x={52} y={43} size={13} color={FW.accent}>(you)</FwText>
      <FwText x={12} y={61} size={12.5} color={FW.text}>{FROM.slice(0, 22)}</FwText>
      <FwText x={12} y={79} size={12.5} color={FW.text}>{FROM.slice(22)}</FwText>

      {/* TO zone */}
      <FwText x={12} y={113} size={13} color={FW.muted}>TO</FwText>
      <FwText x={12} y={131} size={12.5} color={FW.text}>{TO.slice(0, 22)}</FwText>
      <FwText x={12} y={149} size={12.5} color={FW.text}>{TO.slice(22)}</FwText>

      {/* AMOUNT zone (right edge excludes the 28px gutter) */}
      <FwText x={12} y={188} size={15} color={FW.muted}>AMOUNT</FwText>
      <FwText x={204} y={188} size={15} color={FW.accent} align="right">SOL</FwText>
      <FwText x={204} y={224} size={22} color={FW.text} align="right">1.5</FwText>

      {/* gutter cells: dividers + K1 sign / K2 next / K3 reject icons */}
      <FwRect x={226} y={98} w={28} h={1} color={FW.border} />
      <FwRect x={226} y={169} w={28} h={1} color={FW.border} />
      {/* K1 check (accent) */}
      <FwRect x={222} y={65.5} w={8} h={2.5} color={FW.accent} rotation={-QP} />
      <FwRect x={228} y={62.5} w={13} h={2.5} color={FW.accent} rotation={QP} />
      {/* K2 arrow-right */}
      <FwRect x={224.5} y={134} w={10} h={2.5} color={FW.text} />
      <FwRect x={229.5} y={131.8} w={8} h={2.5} color={FW.text} rotation={-QP} />
      <FwRect x={229.5} y={136.2} w={8} h={2.5} color={FW.text} rotation={QP} />
      {/* K3 cross (danger) */}
      <FwRect x={226} y={204} w={15} h={2.5} color={FW.danger} rotation={QP} />
      <FwRect x={226} y={204} w={15} h={2.5} color={FW.danger} rotation={-QP} />
      </group>
    </>
  );
}

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
    // The display is recessed INTO the case opening (not proud of the face)
    // so the case rim overlaps it — that parallax is what makes it read as
    // an embedded screen. Panel keeps the true 240:320 aspect, slightly
    // wider than the opening so its edges hide under the rim; a dark module
    // backing fills the rest of the opening behind it.
    const screenW = size.x * 0.95;
    const screenH = (screenW * 320) / 240;
    return {
      cx: center.x,
      cy: center.y,
      cz: center.z,
      pxScale: screenW / 240,
      // rounded like the case corners so no square corner pokes past the shell
      backShape: roundedRectShape(size.x * 0.97, size.y * 0.97, 6),
      screenZ: bb.max.z - 1.0,
      glassZ: bb.max.z - 0.45,
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
        {/* display-module backing fills the whole case opening behind the panel */}
        <mesh position={[fit.cx, fit.cy, fit.screenZ - 0.3]}>
          <shapeGeometry args={[fit.backShape]} />
          <meshStandardMaterial color="#05080b" roughness={0.85} />
        </mesh>
        {/* live firmware UI in the screen's logical px space (1px = pxScale mm) */}
        <group position={[fit.cx, fit.cy, fit.screenZ]} scale={fit.pxScale}>
          <Esp32ScreenUI />
        </group>
        {/* glass: a faint gloss layer the key light glints across while it
            spins. depthWrite off + late renderOrder so it overlays the UI
            text without ever depth-culling it. */}
        <mesh position={[fit.cx, fit.cy, fit.glassZ]} renderOrder={10}>
          <shapeGeometry args={[fit.backShape]} />
          <meshStandardMaterial
            color="#000000"
            transparent
            opacity={0.16}
            roughness={0.08}
            metalness={0.7}
            depthWrite={false}
          />
        </mesh>
      </group>
      </group>
    </group>
  );
}

useGLTF.preload("/models/esp32/board.glb");

/** Yaw the explode swings to when opened: a three-quarter side view, since
 * the board slides along the device's front axis and the separation is
 * invisible dead-on. */
const OPEN_YAW = -1.05;
const TWO_PI = Math.PI * 2;

/** Slow turntable driven on the model (not the camera): framing is fixed by
 * construction, so the device can never drift out of the canvas. While the
 * explode is open it swings to (and holds) a side pose so the separation is
 * actually visible; closing resumes the spin. Honors prefers-reduced-motion
 * by holding static poses. */
function Turntable({ hold, children }: { hold: boolean; children: React.ReactNode }) {
  const ref = useRef<Group>(null);
  const { gl } = useThree();
  const targetYaw = useRef(OPEN_YAW);
  const dragging = useRef(false);
  const lastX = useRef(0);
  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // While inspecting, the visitor can spin the model themselves. The drag
  // only ever adjusts the model's own yaw — framing is untouched, so the
  // drift that made us drop OrbitControls cannot come back.
  useEffect(() => {
    if (!hold) return;
    if (ref.current) {
      const y = ref.current.rotation.y;
      targetYaw.current = OPEN_YAW + TWO_PI * Math.round((y - OPEN_YAW) / TWO_PI);
    }
    const el = gl.domElement;
    const down = (e: PointerEvent) => {
      dragging.current = true;
      lastX.current = e.clientX;
      el.setPointerCapture(e.pointerId);
      el.style.cursor = "grabbing";
    };
    const move = (e: PointerEvent) => {
      if (!dragging.current) return;
      targetYaw.current += (e.clientX - lastX.current) * 0.012;
      lastX.current = e.clientX;
    };
    const up = () => {
      dragging.current = false;
      el.style.cursor = "grab";
    };
    el.style.cursor = "grab";
    el.style.touchAction = "none";
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    return () => {
      el.style.cursor = "";
      el.style.touchAction = "";
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      dragging.current = false;
    };
  }, [hold, gl]);

  useFrame((_, delta) => {
    if (!ref.current) return;
    if (hold) {
      // swing to the showcase yaw (shortest way around), then follow drags
      const y = ref.current.rotation.y;
      ref.current.rotation.y = reduced
        ? targetYaw.current
        : MathUtils.damp(y, targetYaw.current, 4, delta);
    } else if (!reduced) {
      ref.current.rotation.y += delta * 0.25;
    }
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
      <Turntable hold={device === "esp32" && open}>
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
      <ambientLight intensity={0.35} />
      <directionalLight position={[3, 4, 5]} intensity={0.9} />
      <directionalLight position={[-4, -2, -3]} intensity={0.5} color={EDGE_COLOR} />
      {/* procedural studio: soft top box + two side strips (no HDR download) */}
      <Environment resolution={256} frames={1}>
        <Lightformer
          intensity={1.1}
          rotation-x={Math.PI / 2}
          position={[0, 5, 0]}
          scale={[10, 10, 1]}
        />
        <Lightformer
          intensity={0.9}
          rotation-y={Math.PI / 2}
          position={[-5, 1, 2]}
          scale={[7, 2.5, 1]}
        />
        <Lightformer
          intensity={0.6}
          rotation-y={-Math.PI / 2}
          position={[5, 0, 1]}
          scale={[7, 2.5, 1]}
          color="#bff4f8"
        />
      </Environment>
      <Suspense fallback={null}>
        <DeviceModel key={device} device={device} open={open} />
      </Suspense>
    </Canvas>
  );
}
