import "./polyfills";

import { UR, UREncoder } from "@ngraveio/bc-ur";
import { Buffer } from "buffer";

import { decodeBase64 } from "./solana";

export const DEFAULT_UR_THRESHOLD_BYTES = 300;
export const DEFAULT_UR_FRAGMENT_BYTES = 80;
export const DEFAULT_UR_FRAME_INTERVAL_MS = 450;
const REDUNDANCY_MULTIPLIER = 3;

export type EncodedQrPayload =
  | { kind: "static"; value: string }
  | { kind: "animated"; frames: string[]; intervalMs: number };

interface EncodeTxForQrOptions {
  thresholdBytes?: number;
  fragmentBytes?: number;
  frameIntervalMs?: number;
}

export function encodeTxForQr(
  txBase64: string,
  options: EncodeTxForQrOptions = {}
): EncodedQrPayload {
  const value = txBase64.trim();
  const thresholdBytes = options.thresholdBytes ?? DEFAULT_UR_THRESHOLD_BYTES;
  const fragmentBytes = options.fragmentBytes ?? DEFAULT_UR_FRAGMENT_BYTES;
  const frameIntervalMs = options.frameIntervalMs ?? DEFAULT_UR_FRAME_INTERVAL_MS;

  let txBytes: Uint8Array;
  try {
    txBytes = decodeBase64(value);
  } catch {
    return { kind: "static", value };
  }

  if (txBytes.length <= thresholdBytes) {
    return { kind: "static", value };
  }

  const encoder = new UREncoder(new UR(Buffer.from(txBytes), "bytes"), fragmentBytes, 0);
  const totalFragments = Math.max(encoder.fragmentsLength, 1);
  const frameCount = totalFragments * REDUNDANCY_MULTIPLIER;
  const frames: string[] = [];
  for (let i = 0; i < frameCount; i += 1) {
    frames.push(encoder.nextPart());
  }

  if (frames.length <= 1) {
    return { kind: "static", value };
  }

  return { kind: "animated", frames, intervalMs: frameIntervalMs };
}
