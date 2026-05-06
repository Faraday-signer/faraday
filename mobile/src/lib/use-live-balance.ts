import { useEffect, useRef, useState } from "react";

import { address as toAddress } from "@solana/kit";

import { solanaRpcSubscriptions } from "./sol-client";

export type LiveConnectionState =
  | "idle"
  | "connecting"
  | "live"
  | "reconnecting"
  | "failed";

const LOG_PREFIX = "[Faraday][live-balance]";
const BACKOFF_MS = [1_000, 2_000, 4_000, 8_000, 16_000, 30_000] as const;
const MAX_CONSECUTIVE_FAILURES = 20;

export function useLiveBalance(
  pubkey: string | null,
  onChange: () => void
): LiveConnectionState {
  const [state, setState] = useState<LiveConnectionState>("idle");
  const onChangeRef = useRef(onChange);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (!pubkey) {
      setState("idle");
      return;
    }

    let cancelled = false;
    let abort = new AbortController();
    let consecutiveFailures = 0;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    const addr = (() => {
      try {
        return toAddress(pubkey);
      } catch (error) {
        console.warn(`${LOG_PREFIX} invalid pubkey, bailing out`, error);
        setState("failed");
        return null;
      }
    })();

    if (!addr) return;

    async function runOnce(): Promise<"completed" | "errored"> {
      try {
        setState((prev) => (prev === "idle" ? "connecting" : prev));
        const notifications = await solanaRpcSubscriptions
          .accountNotifications(addr!, { commitment: "confirmed" })
          .subscribe({ abortSignal: abort.signal });

        setState("live");
        consecutiveFailures = 0;
        onChangeRef.current();

        for await (const _notif of notifications) {
          if (cancelled) return "completed";
          onChangeRef.current();
        }
        return "completed";
      } catch (error) {
        if (cancelled) return "completed";
        console.warn(`${LOG_PREFIX} subscription error`, error);
        return "errored";
      }
    }

    function scheduleReconnect() {
      if (cancelled) return;
      consecutiveFailures += 1;
      if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
        console.error(`${LOG_PREFIX} giving up after ${consecutiveFailures} failures`);
        setState("failed");
        return;
      }
      setState("reconnecting");
      const delay = BACKOFF_MS[Math.min(consecutiveFailures - 1, BACKOFF_MS.length - 1)];
      reconnectTimer = setTimeout(() => {
        if (cancelled) return;
        abort = new AbortController();
        void loop();
      }, delay);
    }

    async function loop(): Promise<void> {
      const outcome = await runOnce();
      if (cancelled) return;
      if (outcome === "errored") scheduleReconnect();
      else scheduleReconnect();
    }

    void loop();

    return () => {
      cancelled = true;
      abort.abort();
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };
  }, [pubkey]);

  return state;
}
