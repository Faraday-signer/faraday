import { describe, expect, it } from "vitest";
import { URDecoder } from "@ngraveio/bc-ur";

import { encodeBase64 } from "./solana";
import { encodeTxForQr } from "./ur-encode";

describe("encodeTxForQr", () => {
  it("returns static payload below threshold", () => {
    const bytes = new Uint8Array(120).fill(0xaa);
    const txBase64 = encodeBase64(bytes);
    const encoded = encodeTxForQr(txBase64, {
      thresholdBytes: 300,
      fragmentBytes: 100,
      frameIntervalMs: 123
    });

    expect(encoded.kind).toBe("static");
    if (encoded.kind === "static") {
      expect(encoded.value).toBe(txBase64);
    }
  });

  it("returns animated UR payload above threshold and round-trips", () => {
    const bytes = new Uint8Array(700);
    for (let i = 0; i < bytes.length; i += 1) {
      bytes[i] = (i * 31 + 7) & 0xff;
    }
    const txBase64 = encodeBase64(bytes);
    const encoded = encodeTxForQr(txBase64, {
      thresholdBytes: 300,
      fragmentBytes: 100,
      frameIntervalMs: 321
    });

    expect(encoded.kind).toBe("animated");
    if (encoded.kind !== "animated") {
      return;
    }

    expect(encoded.frames.length).toBeGreaterThan(1);
    expect(encoded.intervalMs).toBe(321);

    const seq = encoded.frames
      .map((frame) => {
        const m = frame.match(/^ur:[^/]+\/(\d+)-(\d+)\//i);
        return m ? { current: Number.parseInt(m[1], 10), total: Number.parseInt(m[2], 10) } : null;
      })
      .filter((value): value is { current: number; total: number } => value !== null);
    expect(seq.length).toBe(encoded.frames.length);
    const total = seq[0]?.total ?? 0;
    expect(total).toBeGreaterThan(0);
    expect(seq.some((entry) => entry.current > total)).toBe(true);

    const decoder = new URDecoder();
    for (const frame of encoded.frames) {
      decoder.receivePart(frame);
    }

    expect(decoder.isComplete()).toBe(true);
    expect(decoder.isSuccess()).toBe(true);
    const roundTrip = new Uint8Array(decoder.resultUR().cbor);
    expect(roundTrip).toEqual(bytes);
  });
});
